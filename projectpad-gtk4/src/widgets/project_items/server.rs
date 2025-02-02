use crate::{
    app::ProjectpadApplication,
    notes, perform_insert_or_update,
    sql_thread::SqlFunc,
    widgets::{
        project_item::{ProjectItem, WidgetMode},
        project_item_list::ProjectItemList,
        project_item_model::ProjectItemType,
        project_items::common::{display_item_edit_dialog, DialogClamp},
    },
};
use adw::prelude::*;
use diesel::prelude::*;
use glib::GString;
use gtk::subclass::prelude::*;
use itertools::Itertools;
use projectpadsql::{
    get_project_group_names, get_server_group_names,
    models::{
        EnvironmentType, InterestType, RunOn, Server, ServerAccessType, ServerDatabase,
        ServerExtraUserAccount, ServerNote, ServerPointOfInterest, ServerType, ServerWebsite,
    },
};
use std::str::FromStr;
use std::{
    collections::{BTreeSet, HashMap},
    sync::mpsc,
    time::Duration,
};

use super::{
    common::{self},
    item_header_edit::ItemHeaderEdit,
    item_header_view::ItemHeaderView,
    server_items::{
        interest_type_get_icon, server_database_view_edit::ServerDatabaseViewEdit,
        server_extra_user_account_view_edit::ServerExtraUserAccountViewEdit,
        server_poi_view_edit::ServerPoiViewEdit, server_website_view_edit::ServerWebsiteViewEdit,
    },
    server_view_edit::ServerViewEdit,
};
use crate::widgets::project_items::note::Note;

#[derive(Clone, Debug)]
pub enum ServerItem {
    Website(ServerWebsite),
    PointOfInterest(ServerPointOfInterest),
    Note(ServerNote),
    ExtraUserAccount(ServerExtraUserAccount),
    Database(ServerDatabase),
}

impl ServerItem {
    pub fn group_name(&self) -> Option<&str> {
        match self {
            ServerItem::Website(w) => w.group_name.as_deref(),
            ServerItem::PointOfInterest(p) => p.group_name.as_deref(),
            ServerItem::Note(n) => n.group_name.as_deref(),
            ServerItem::ExtraUserAccount(u) => u.group_name.as_deref(),
            ServerItem::Database(d) => d.group_name.as_deref(),
        }
    }

    pub fn get_id(&self) -> i32 {
        match self {
            ServerItem::Website(w) => w.id,
            ServerItem::PointOfInterest(p) => p.id,
            ServerItem::Note(n) => n.id,
            ServerItem::ExtraUserAccount(u) => u.id,
            ServerItem::Database(d) => d.id,
        }
    }

    pub fn server_id(&self) -> i32 {
        match self {
            ServerItem::Website(w) => w.server_id,
            ServerItem::PointOfInterest(p) => p.server_id,
            ServerItem::Note(n) => n.server_id,
            ServerItem::ExtraUserAccount(u) => u.server_id,
            ServerItem::Database(d) => d.server_id,
        }
    }
}

#[derive(Debug)]
pub struct ChannelData {
    server: Server,
    server_items: Vec<ServerItem>,
    group_start_indices: HashMap<i32, String>,
    databases_for_websites: HashMap<i32, ServerDatabase>,
    websites_for_databases: HashMap<i32, Vec<ServerWebsite>>,
    project_group_names: Vec<String>,
}

pub fn load_and_display_server(
    parent: &adw::Bin,
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    server_item_id: Option<i32>,
    project_item: &ProjectItem,
) {
    let (sender, receiver) = async_channel::bounded(1);
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server::dsl as srv;
            use projectpadsql::schema::server_database::dsl as srv_db;
            use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
            use projectpadsql::schema::server_note::dsl as srv_note;
            use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
            use projectpadsql::schema::server_website::dsl as srv_www;
            let (server, items, databases_for_websites, websites_for_databases) = {
                let server = srv::server
                    .filter(srv::id.eq(server_id))
                    .first::<Server>(sql_conn)
                    .unwrap();

                let server_websites = srv_www::server_website
                    .filter(srv_www::server_id.eq(server_id))
                    .order(srv_www::desc.asc())
                    .load::<ServerWebsite>(sql_conn)
                    .unwrap();

                let databases_for_websites = srv_db::server_database
                    .filter(
                        srv_db::id
                            .eq_any(server_websites.iter().filter_map(|w| w.server_database_id)),
                    )
                    .load::<ServerDatabase>(sql_conn)
                    .unwrap()
                    .into_iter()
                    .map(|db| (db.id, db))
                    .collect::<HashMap<_, _>>();

                let mut servers = server_websites
                    .into_iter()
                    .map(ServerItem::Website)
                    .collect::<Vec<_>>();

                servers.extend(
                    srv_poi::server_point_of_interest
                        .filter(srv_poi::server_id.eq(server_id))
                        .order(srv_poi::desc.asc())
                        .load::<ServerPointOfInterest>(sql_conn)
                        .unwrap()
                        .into_iter()
                        .map(ServerItem::PointOfInterest),
                );
                servers.extend(
                    srv_note::server_note
                        .filter(srv_note::server_id.eq(server_id))
                        .order(srv_note::title.asc())
                        .load::<ServerNote>(sql_conn)
                        .unwrap()
                        .into_iter()
                        .map(ServerItem::Note),
                );
                servers.extend(
                    &mut srv_usr::server_extra_user_account
                        .filter(srv_usr::server_id.eq(server_id))
                        .order(srv_usr::desc.asc())
                        .load::<ServerExtraUserAccount>(sql_conn)
                        .unwrap()
                        .into_iter()
                        .map(ServerItem::ExtraUserAccount),
                );

                let databases = srv_db::server_database
                    .filter(srv_db::server_id.eq(server_id))
                    .order(srv_db::desc.asc())
                    .load::<ServerDatabase>(sql_conn)
                    .unwrap();
                let mut dbs_as_server_items =
                    databases.clone().into_iter().map(ServerItem::Database);
                servers.extend(&mut dbs_as_server_items);

                let mut websites_for_databases = HashMap::new();
                for (key, group) in &srv_www::server_website
                    .filter(srv_www::server_database_id.eq_any(databases.iter().map(|db| db.id)))
                    .order(srv_www::server_database_id.asc())
                    .load::<ServerWebsite>(sql_conn)
                    .unwrap()
                    .into_iter()
                    .group_by(|www| www.server_database_id.unwrap())
                {
                    websites_for_databases.insert(key, group.collect());
                }

                (
                    server,
                    servers,
                    databases_for_websites,
                    websites_for_databases,
                )
            };

            let group_names: BTreeSet<&str> = items.iter().filter_map(|i| i.group_name()).collect();
            let mut group_start_indices = HashMap::new();

            let mut grouped_items = vec![];
            grouped_items.extend(items.iter().filter(|i| i.group_name() == None));
            for group_name in &group_names {
                group_start_indices.insert(grouped_items.len() as i32, group_name.to_string());
                grouped_items.extend(
                    items
                        .iter()
                        .filter(|i| i.group_name().as_ref() == Some(group_name)),
                );
            }

            let project_group_names = get_project_group_names(sql_conn, server.project_id);

            sender
                .send_blocking(ChannelData {
                    server,
                    server_items: grouped_items.into_iter().cloned().collect(),
                    group_start_indices,
                    databases_for_websites,
                    websites_for_databases,
                    project_group_names,
                })
                .unwrap();
        }))
        .unwrap();

    let p = parent.clone();
    let mut pi = project_item.clone();
    glib::spawn_future_local(async move {
        let channel_data = receiver.recv().await.unwrap();
        display_server(&p, channel_data, server_item_id, &mut pi);
    });
}

fn display_server(
    parent: &adw::Bin,
    channel_data: ChannelData,
    server_item_id: Option<i32>,
    project_item: &ProjectItem,
) {
    let (header_box, header_edit, vbox, _) = server_contents(
        &channel_data.server,
        &channel_data.project_group_names,
        WidgetMode::Show,
    );
    let add_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .build();
    let server_id = channel_data.server.id;
    add_btn.connect_clicked(move |_| display_add_project_item_dialog(server_id));
    header_box.append(&add_btn);

    let edit_btn = gtk::Button::builder()
        .icon_name("document-edit-symbolic")
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .build();
    header_box.append(&edit_btn);

    let delete_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .build();
    header_box.append(&delete_btn);

    let pgn = channel_data.project_group_names.clone();
    edit_btn.connect_closure("clicked", false,
            glib::closure_local!(@strong channel_data.server as s, @strong pgn as pgn_, @strong vbox as v => move |_b: gtk::Button| {
                let (_, header_edit, vbox, server_view_edit) = server_contents(&s, &pgn_, WidgetMode::Edit);

                let (dlg, save_btn) = display_item_edit_dialog(&v, "Edit Server", vbox, 600, 600, DialogClamp::Yes);
                let he = header_edit.unwrap().clone();
                save_btn.connect_clicked(move|_| {
                    let receiver = save_server(
                        Some(s.id),
                        he.single_env(),
                        server_view_edit.property("is_retired"),
                        he.property("title"),
                        server_view_edit.property("ip"),
                        server_view_edit.property("username"),
                        server_view_edit.property("password"),
                        server_view_edit.property("text"),
                        ServerType::from_str(&server_view_edit.property::<String>("server_type"))
                        .unwrap(),
                        ServerAccessType::from_str(&server_view_edit.property::<String>("access_type"))
                        .unwrap(),
                    );

                    let app = gio::Application::default()
                        .and_downcast::<ProjectpadApplication>()
                        .unwrap();
                    let db_sender = app.get_sql_channel();
                    let dlg = dlg.clone();
                    glib::spawn_future_local(async move {
                        let server_after_result = receiver.recv().await.unwrap();
                        let window = app.imp().window.get().unwrap();
                        let win_binding = window.upgrade();
                        let win_binding_ref = win_binding.as_ref().unwrap();
                        let pi = &win_binding_ref.imp().project_item;
                        let pi_bin = &win_binding_ref.imp().project_item.imp().project_item;
                        load_and_display_server(pi_bin, db_sender, s.id, None, pi);
                        dlg.close();
                    });
                });
            }),
        );

    add_server_items(&channel_data, server_item_id, &vbox, project_item);

    parent.set_child(Some(&vbox));
}

pub fn server_contents(
    server: &Server,
    project_group_names: &[String],
    widget_mode: WidgetMode,
) -> (gtk::Box, Option<ItemHeaderEdit>, gtk::Box, ServerViewEdit) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .build();

    // let server_ar = adw::ActionRow::builder().title("Server name").build();
    // server_ar.add_suffix(
    //     &gtk::Button::builder()
    //         .icon_name("open-menu-symbolic")
    //         .has_frame(false)
    //         .valign(gtk::Align::Center)
    //         .build(),
    // );
    // server.add(&server_ar);
    let server_item0 = adw::PreferencesGroup::builder().build();

    let (project_item_header_edit, header_box) = if widget_mode == WidgetMode::Edit {
        let project_item_header = ItemHeaderEdit::new(
            ProjectItemType::Server.get_icon(),
            server.group_name.as_deref(),
            project_group_names,
            common::EnvOrEnvs::Env(server.environment),
        );
        project_item_header.set_title(server.desc.clone());
        vbox.append(&project_item_header);
        (
            Some(project_item_header.clone()),
            project_item_header.header_box(),
        )
    } else {
        let project_item_header = ItemHeaderView::new(ProjectItemType::Server);
        project_item_header.set_title(server.desc.clone());
        vbox.append(&project_item_header);
        (None, project_item_header.header_box())
    };

    let server_view_edit = ServerViewEdit::new();
    server_view_edit.set_ip(server.ip.clone());
    server_view_edit.set_is_retired(server.is_retired);
    server_view_edit.set_username(server.username.clone());
    server_view_edit.set_access_type(server.access_type.to_string());
    server_view_edit.set_server_type(server.server_type.to_string());
    server_view_edit.set_password(server.password.clone());
    server_view_edit.set_text(server.text.clone());
    server_view_edit.set_auth_key_filename(
        server
            .auth_key_filename
            .clone()
            .unwrap_or_else(|| "".to_string()),
    );
    server_view_edit.prepare(widget_mode);
    vbox.append(&server_view_edit);

    vbox.append(&server_item0);

    (header_box, project_item_header_edit, vbox, server_view_edit)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IncludeAddGroup {
    Yes,
    No,
}

fn add_server_items(
    channel_data: &ChannelData,
    focused_server_item_id: Option<i32>,
    vbox: &gtk::Box,
    project_item: &ProjectItem,
) {
    let mut cur_parent = vbox.clone();
    let mut cur_group_name = None::<&str>;
    dbg!(focused_server_item_id);

    for server_item in channel_data.server_items.iter() {
        let group_name = server_item.group_name();

        if group_name != cur_group_name {
            if let Some(grp) = group_name {
                let (frame, frame_box) = group_frame(grp);
                cur_parent = frame_box;
                vbox.append(&frame);
                cur_group_name = group_name;
            }
        }
        if server_item.group_name().is_none() {
            cur_parent = vbox.clone();
        }
        match server_item {
            ServerItem::Website(w) => display_server_website(w, &cur_parent),
            ServerItem::PointOfInterest(poi) => display_server_poi(poi, &cur_parent),
            ServerItem::Note(n) => display_server_note(n, &cur_parent, focused_server_item_id),
            ServerItem::ExtraUserAccount(u) => display_server_extra_user_account(u, &cur_parent),
            ServerItem::Database(u) => display_server_database(u, &cur_parent),
        }
        if Some(server_item.get_id()) == focused_server_item_id {
            let me = cur_parent.last_child().unwrap().clone();
            let v = vbox.clone();

            let pi = project_item.clone();
            glib::spawn_future_local(async move {
                // TODO crappy but doesn't work without the wait..
                glib::timeout_future(Duration::from_millis(50)).await;
                pi.emit_by_name::<()>(
                    "request-scroll",
                    &[&me
                        .compute_bounds(&v.upcast::<gtk::Widget>())
                        .unwrap()
                        .top_left()
                        .y()],
                );
            });
        }
    }
}

fn group_frame(group_name: &str) -> (gtk::Frame, gtk::Box) {
    let frame = gtk::Frame::builder().build();
    let frame_box = gtk::Box::builder()
        .css_classes(["card", "frame-group"])
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .build();
    let frame_header = gtk::Box::builder().build();
    frame_header.append(
        &gtk::Label::builder()
            .css_classes(["heading"])
            .halign(gtk::Align::Start)
            .hexpand(true)
            .label(group_name)
            .build(),
    );

    // let edit_btn = gtk::Button::builder()
    //     .icon_name("document-edit-symbolic")
    //     .css_classes(["flat"])
    //     .valign(gtk::Align::Center)
    //     .halign(gtk::Align::End)
    //     .build();
    // frame_header.append(&edit_btn);

    // let add_btn = gtk::Button::builder()
    //     .icon_name("list-add-symbolic")
    //     .css_classes(["flat"])
    //     .valign(gtk::Align::Center)
    //     .halign(gtk::Align::End)
    //     .build();
    // add_btn.connect_clicked(move |_| display_add_project_item_dialog());
    // frame_header.append(&add_btn);

    frame_box.append(&frame_header);
    frame_box.append(&gtk::Separator::builder().build());
    frame.set_child(Some(&frame_box));
    (frame, frame_box)
}

fn display_add_project_item_dialog(server_id: i32) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let header_bar = adw::HeaderBar::builder()
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();

    let cancel_btn = gtk::Button::builder().label("Cancel").build();
    header_bar.pack_start(&cancel_btn);
    vbox.append(&header_bar);

    let cbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_top(15)
        .margin_start(15)
        .margin_end(15)
        .margin_bottom(15)
        .spacing(10)
        .build();

    let poi_btn = gtk::Button::builder()
        .child(&ProjectItemList::create_project_item_box(
            "cog",
            "Add point of interest",
            "a command to run or a relevant file or folder located on that server.",
        ))
        .build();
    cbox.append(&poi_btn);

    let website_btn = gtk::Button::builder()
        .child(&ProjectItemList::create_project_item_box(
            "globe",
            "Add website",
            "a service (website or not) that's reachable over the network that lives on that server.",
        ))
        .build();
    cbox.append(&website_btn);

    let db_btn = gtk::Button::builder()
        .child(&ProjectItemList::create_project_item_box(
            "database",
            "Add database",
            "a database that lives on that server.",
        ))
        .build();
    cbox.append(&db_btn);

    let user_btn = gtk::Button::builder()
        .child(&ProjectItemList::create_project_item_box(
            "user",
            "Add extra user",
            "username and password or authentication key, somehow tied to this server.",
        ))
        .build();
    cbox.append(&user_btn);

    let note_btn = gtk::Button::builder()
        .child(&ProjectItemList::create_project_item_box(
            "clipboard",
            "Add note",
            "markdown-formatted note containing free-form text.",
        ))
        .build();
    cbox.append(&note_btn);

    let stack = gtk::Stack::builder().build();
    stack.add_child(&cbox);
    vbox.append(&stack);

    let dialog = adw::Dialog::builder()
        .title("Add server item")
        .child(&vbox)
        .build();

    let s = stack.clone();
    let dlg = dialog.clone();
    let (header_edit, server_contents_child, server_view_edit) =
        server_poi_contents(&ServerPointOfInterest::default(), WidgetMode::Edit);
    let hb = header_bar.clone();
    let he = header_edit.unwrap().clone();
    poi_btn.connect_clicked(move |_| {
        prepare_add_server_poi_dlg(
            server_id,
            &dlg,
            &s,
            &hb,
            &he,
            &server_view_edit,
            &server_contents_child,
        );
    });

    let s = stack.clone();
    let dlg = dialog.clone();
    let (header_edit, server_contents_child, server_view_edit) =
        server_website_contents(&ServerWebsite::default(), WidgetMode::Edit);
    let hb = header_bar.clone();
    let he = header_edit.unwrap().clone();
    website_btn.connect_clicked(move |_| {
        prepare_add_server_website_dlg(
            server_id,
            &dlg,
            &s,
            &hb,
            &he,
            &server_view_edit,
            &server_contents_child,
        );
    });

    let s = stack.clone();
    let dlg = dialog.clone();
    let (header_edit, server_contents_child, server_view_edit) =
        server_database_contents(&ServerDatabase::default(), WidgetMode::Edit);
    let hb = header_bar.clone();
    let he = header_edit.unwrap().clone();
    db_btn.connect_clicked(move |_| {
        prepare_add_server_database_dlg(
            server_id,
            &dlg,
            &s,
            &hb,
            &he,
            &server_view_edit,
            &server_contents_child,
        );
    });

    let s = stack.clone();
    let dlg = dialog.clone();
    let (header_edit, server_contents_child, server_view_edit) =
        server_extra_user_account_contents(&ServerExtraUserAccount::default(), WidgetMode::Edit);
    let hb = header_bar.clone();
    let he = header_edit.unwrap().clone();
    user_btn.connect_clicked(move |_| {
        prepare_add_server_extra_user_account_dlg(
            server_id,
            &dlg,
            &s,
            &hb,
            &he,
            &server_view_edit,
            &server_contents_child,
        );
    });

    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    dialog.present(&app.active_window().unwrap());
}

fn prepare_add_server_poi_dlg(
    server_id: i32,
    dlg: &adw::Dialog,
    s: &gtk::Stack,
    hb: &adw::HeaderBar,
    he: &ItemHeaderEdit,
    server_poi_view_edit: &ServerPoiViewEdit,
    server_contents_child: &adw::PreferencesGroup,
) {
    dlg.set_title("Add Server Point of Interest");
    dlg.set_content_width(600);
    dlg.set_content_height(600);

    let contents = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    contents.append(he);
    contents.append(server_contents_child);
    s.add_named(
        &adw::Clamp::builder()
            .margin_top(10)
            .child(&contents)
            .build(),
        Some("second"),
    );
    s.set_visible_child_name("second");

    let save_btn = gtk::Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let d = dlg.clone();
    let server_poi_view_edit = server_poi_view_edit.clone();
    let he = he.clone();
    save_btn.connect_clicked(move |_| {
        let receiver = save_server_poi(
            server_id,
            None,
            he.property("title"),
            server_poi_view_edit.property("path"),
            server_poi_view_edit.property("text"),
            InterestType::from_str(&server_poi_view_edit.property::<String>("interest_type"))
                .unwrap(),
            RunOn::from_str(&server_poi_view_edit.property::<String>("run_on")).unwrap(),
        );
        let d = d.clone();
        glib::spawn_future_local(async move {
            let server_poi_after_result = receiver.recv().await.unwrap();
            d.close();

            if let Ok(server_poi) = server_poi_after_result {
                ProjectItemList::display_project_item(
                    server_poi.server_id,
                    ProjectItemType::Server,
                );
            }
        });
    });
    hb.pack_end(&save_btn);
}

pub fn save_server_poi(
    server_id: i32,
    server_poi_id: Option<i32>,
    new_desc: String,
    new_path: String,
    new_text: String,
    new_interest_type: InterestType,
    new_run_on: RunOn,
) -> async_channel::Receiver<Result<ServerPointOfInterest, (String, Option<String>)>> {
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    let db_sender = app.get_sql_channel();
    let (sender, receiver) = async_channel::bounded(1);

    // TODO commented fields (group and so on)
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
            let changeset = (
                srv_poi::desc.eq(new_desc.as_str()),
                srv_poi::path.eq(new_path.as_str()),
                srv_poi::text.eq(new_text.as_str()),
                // never store Some("") for group, we want None then.
                // prj_poi::group_name.eq(new_group
                //     .as_ref()
                //     .map(|s| s.as_str())
                //     .filter(|s| !s.is_empty())),
                srv_poi::interest_type.eq(new_interest_type),
                srv_poi::run_on.eq(new_run_on),
                srv_poi::server_id.eq(server_id),
            );
            let project_poi_after_result = perform_insert_or_update!(
                sql_conn,
                server_poi_id,
                srv_poi::server_point_of_interest,
                srv_poi::id,
                changeset,
                ServerPointOfInterest,
            );
            sender.send_blocking(project_poi_after_result).unwrap();
        }))
        .unwrap();
    receiver
}

fn prepare_add_server_website_dlg(
    server_id: i32,
    dlg: &adw::Dialog,
    s: &gtk::Stack,
    hb: &adw::HeaderBar,
    he: &ItemHeaderEdit,
    server_website_view_edit: &ServerWebsiteViewEdit,
    server_contents_child: &adw::PreferencesGroup,
) {
    dlg.set_title("Add Server website");
    dlg.set_content_width(600);
    dlg.set_content_height(600);
    let contents = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    contents.append(he);
    contents.append(server_contents_child);
    s.add_named(
        &adw::Clamp::builder()
            .margin_top(10)
            .child(&contents)
            .build(),
        Some("second"),
    );
    s.set_visible_child_name("second");

    let save_btn = gtk::Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let d = dlg.clone();
    let server_poi_view_edit = server_website_view_edit.clone();
    let he = he.clone();
    save_btn.connect_clicked(move |_| {
        let receiver = save_server_website(
            server_id,
            None,
            he.property("title"),
            server_poi_view_edit.property("url"),
            server_poi_view_edit.property("text"),
            server_poi_view_edit.property("username"),
            server_poi_view_edit.property("password"),
        );
        let d = d.clone();
        glib::spawn_future_local(async move {
            let server_poi_after_result = receiver.recv().await.unwrap();
            d.close();

            if let Ok(server_poi) = server_poi_after_result {
                ProjectItemList::display_project_item(
                    server_poi.server_id,
                    ProjectItemType::Server,
                );
            }
        });
    });
    hb.pack_end(&save_btn);
}

pub fn save_server_website(
    server_id: i32,
    server_www_id: Option<i32>,
    new_desc: String,
    new_url: String,
    new_text: String,
    new_username: String,
    new_password: String,
) -> async_channel::Receiver<Result<ServerWebsite, (String, Option<String>)>> {
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    let db_sender = app.get_sql_channel();
    let (sender, receiver) = async_channel::bounded(1);

    // TODO commented fields (group and so on)
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server_website::dsl as srv_www;
            let changeset = (
                srv_www::desc.eq(new_desc.as_str()),
                srv_www::url.eq(new_url.as_str()),
                srv_www::text.eq(new_text.as_str()),
                // never store Some("") for group, we want None then.
                // prj_poi::group_name.eq(new_group
                //     .as_ref()
                //     .map(|s| s.as_str())
                //     .filter(|s| !s.is_empty())),
                srv_www::username.eq(new_username.as_str()),
                srv_www::password.eq(new_password.as_str()),
                srv_www::server_id.eq(server_id),
            );
            let server_www_after_result = perform_insert_or_update!(
                sql_conn,
                server_www_id,
                srv_www::server_website,
                srv_www::id,
                changeset,
                ServerWebsite,
            );
            sender.send_blocking(server_www_after_result).unwrap();
        }))
        .unwrap();
    receiver
}

fn prepare_add_server_database_dlg(
    server_id: i32,
    dlg: &adw::Dialog,
    s: &gtk::Stack,
    hb: &adw::HeaderBar,
    he: &ItemHeaderEdit,
    server_database_view_edit: &ServerDatabaseViewEdit,
    server_contents_child: &adw::PreferencesGroup,
) {
    dlg.set_title("Add Server database");
    dlg.set_content_width(600);
    dlg.set_content_height(600);
    let contents = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    contents.append(he);
    contents.append(server_contents_child);
    s.add_named(
        &adw::Clamp::builder()
            .margin_top(10)
            .child(&contents)
            .build(),
        Some("second"),
    );
    s.set_visible_child_name("second");

    let save_btn = gtk::Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let d = dlg.clone();
    let server_db_view_edit = server_database_view_edit.clone();
    let he = he.clone();
    save_btn.connect_clicked(move |_| {
        let receiver = save_server_database(
            server_id,
            None,
            he.property("title"),
            server_db_view_edit.property("name"),
            server_db_view_edit.property("text"),
            server_db_view_edit.property("username"),
            server_db_view_edit.property("password"),
        );
        let d = d.clone();
        glib::spawn_future_local(async move {
            let server_poi_after_result = receiver.recv().await.unwrap();
            d.close();

            if let Ok(server_poi) = server_poi_after_result {
                ProjectItemList::display_project_item(
                    server_poi.server_id,
                    ProjectItemType::Server,
                );
            }
        });
    });
    hb.pack_end(&save_btn);
}

pub fn save_server_database(
    server_id: i32,
    server_db_id: Option<i32>,
    new_desc: String,
    new_name: String,
    new_text: String,
    new_username: String,
    new_password: String,
) -> async_channel::Receiver<Result<ServerDatabase, (String, Option<String>)>> {
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    let db_sender = app.get_sql_channel();
    let (sender, receiver) = async_channel::bounded(1);

    // TODO commented fields (group and so on)
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server_database::dsl as srv_db;
            let changeset = (
                srv_db::desc.eq(new_desc.as_str()),
                srv_db::name.eq(new_name.as_str()),
                srv_db::text.eq(new_text.as_str()),
                // never store Some("") for group, we want None then.
                // prj_poi::group_name.eq(new_group
                //     .as_ref()
                //     .map(|s| s.as_str())
                //     .filter(|s| !s.is_empty())),
                srv_db::username.eq(new_username.as_str()),
                srv_db::password.eq(new_password.as_str()),
                srv_db::server_id.eq(server_id),
            );
            let server_db_after_result = perform_insert_or_update!(
                sql_conn,
                server_db_id,
                srv_db::server_database,
                srv_db::id,
                changeset,
                ServerDatabase,
            );
            sender.send_blocking(server_db_after_result).unwrap();
        }))
        .unwrap();
    receiver
}

fn prepare_add_server_extra_user_account_dlg(
    server_id: i32,
    dlg: &adw::Dialog,
    s: &gtk::Stack,
    hb: &adw::HeaderBar,
    he: &ItemHeaderEdit,
    server_user_view_edit: &ServerExtraUserAccountViewEdit,
    server_contents_child: &adw::PreferencesGroup,
) {
    dlg.set_title("Add Server extra user");
    dlg.set_content_width(600);
    dlg.set_content_height(600);
    let contents = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    contents.append(he);
    contents.append(server_contents_child);
    s.add_named(
        &adw::Clamp::builder()
            .margin_top(10)
            .child(&contents)
            .build(),
        Some("second"),
    );
    s.set_visible_child_name("second");

    let save_btn = gtk::Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let d = dlg.clone();
    let server_db_view_edit = server_user_view_edit.clone();
    let he = he.clone();
    save_btn.connect_clicked(move |_| {
        let receiver = save_server_extra_user_account(
            server_id,
            None,
            he.property("title"),
            // server_db_view_edit.property("auth_key"),
            // server_db_view_edit.property("auth_key_filename"),
            server_db_view_edit.property("username"),
            server_db_view_edit.property("password"),
        );
        let d = d.clone();
        glib::spawn_future_local(async move {
            let server_poi_after_result = receiver.recv().await.unwrap();
            d.close();

            if let Ok(server_poi) = server_poi_after_result {
                ProjectItemList::display_project_item(
                    server_poi.server_id,
                    ProjectItemType::Server,
                );
            }
        });
    });
    hb.pack_end(&save_btn);
}

pub fn save_server_extra_user_account(
    server_id: i32,
    server_user_id: Option<i32>,
    new_desc: String,
    // new_auth_key: Option<Vec<u8>>,
    // new_auth_key_filename: Option<String>,
    new_username: String,
    new_password: String,
) -> async_channel::Receiver<Result<ServerExtraUserAccount, (String, Option<String>)>> {
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    let db_sender = app.get_sql_channel();
    let (sender, receiver) = async_channel::bounded(1);

    // TODO commented fields (group and so on)
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server_extra_user_account::dsl as srv_user;
            let changeset = (
                srv_user::desc.eq(new_desc.as_str()),
                // TODO auth key
                // srv_user::auth_key.eq(new_auth_key.as_str()),

                // srv_user::auth_key_filename.eq(new_auth_key_filename.as_str()),
                // never store Some("") for group, we want None then.
                // prj_poi::group_name.eq(new_group
                //     .as_ref()
                //     .map(|s| s.as_str())
                //     .filter(|s| !s.is_empty())),
                srv_user::username.eq(new_username.as_str()),
                srv_user::password.eq(new_password.as_str()),
                srv_user::server_id.eq(server_id),
            );
            let server_db_after_result = perform_insert_or_update!(
                sql_conn,
                server_user_id,
                srv_user::server_extra_user_account,
                srv_user::id,
                changeset,
                ServerExtraUserAccount,
            );
            sender.send_blocking(server_db_after_result).unwrap();
        }))
        .unwrap();
    receiver
}

fn add_group_edit_suffix(server_item1: &adw::PreferencesGroup, edit_closure: glib::RustClosure) {
    let edit_btn = gtk::Button::builder()
        .icon_name("document-edit-symbolic")
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .build();
    let delete_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk::Align::Center)
        .build();
    edit_btn.connect_closure("clicked", false, edit_closure);
    let suffix_box = gtk::Box::builder().css_classes(["toolbar"]).build();
    suffix_box.append(&edit_btn);
    suffix_box.append(&delete_btn);
    server_item1.set_header_suffix(Some(&suffix_box));
}

fn display_server_website(w: &ServerWebsite, vbox: &gtk::Box) {
    let (_, server_item1, _) = server_website_contents(w, WidgetMode::Show);
    vbox.append(&server_item1);

    add_group_edit_suffix(
        &server_item1,
        glib::closure_local!(@strong w as w1, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let (header, server_item, _) = server_website_contents(&w1, WidgetMode::Edit);
            item_box.append(&header.unwrap());
            item_box.append(&server_item);

            display_item_edit_dialog(&v, "Edit Website", item_box, 600, 600, DialogClamp::Yes);
        }),
    );
}

fn server_website_contents(
    website: &ServerWebsite,
    widget_mode: WidgetMode,
) -> (
    Option<ItemHeaderEdit>,
    adw::PreferencesGroup,
    ServerWebsiteViewEdit,
) {
    let item_header_edit = if widget_mode == WidgetMode::Edit {
        let website_item_header = ItemHeaderEdit::new(
            "globe",
            website.group_name.as_deref(),
            &[], // TODO list of groups get_server_group_names(),
            common::EnvOrEnvs::None,
        );
        website_item_header.set_title(website.desc.clone());
        Some(website_item_header)
    } else {
        None
    };

    let server_item1 = adw::PreferencesGroup::builder().build();

    if widget_mode == WidgetMode::Show {
        server_item1.set_title(&website.desc);
    }

    let server_website_view_edit = ServerWebsiteViewEdit::new();
    server_website_view_edit.set_url(website.url.to_string());
    server_website_view_edit.set_username(website.username.to_string());
    server_website_view_edit.set_password(website.password.to_string());
    server_website_view_edit.set_text(website.text.to_string());
    server_website_view_edit.prepare(widget_mode);
    server_item1.add(&server_website_view_edit);

    // TODO databases linked to website?

    (item_header_edit, server_item1, server_website_view_edit)
}

fn display_server_database(w: &ServerDatabase, vbox: &gtk::Box) {
    let (_, server_item1, _) = server_database_contents(w, WidgetMode::Show);
    vbox.append(&server_item1);

    add_group_edit_suffix(
        &server_item1,
        glib::closure_local!(@strong w as w1, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let (header, server_item, _) = server_database_contents(&w1, WidgetMode::Edit);
            item_box.append(&header.unwrap());
            item_box.append(&server_item);

            display_item_edit_dialog(&v, "Edit Database", item_box, 600, 600, DialogClamp::Yes);
        }),
    );
}

fn server_database_contents(
    database: &ServerDatabase,
    widget_mode: WidgetMode,
) -> (
    Option<ItemHeaderEdit>,
    adw::PreferencesGroup,
    ServerDatabaseViewEdit,
) {
    let item_header_edit = if widget_mode == WidgetMode::Edit {
        let database_item_header = ItemHeaderEdit::new(
            "globe",
            database.group_name.as_deref(),
            &[], // TODO list of groups get_server_group_names(),
            common::EnvOrEnvs::None,
        );
        database_item_header.set_title(database.desc.clone());
        Some(database_item_header)
    } else {
        None
    };

    let server_item1 = adw::PreferencesGroup::builder().build();

    if widget_mode == WidgetMode::Show {
        server_item1.set_title(&database.desc);
    }

    let server_database_view_edit = ServerDatabaseViewEdit::new();
    server_database_view_edit.set_name(database.name.to_string());
    server_database_view_edit.set_username(database.username.to_string());
    server_database_view_edit.set_password(database.password.to_string());
    server_database_view_edit.set_text(database.text.to_string());
    server_database_view_edit.prepare(widget_mode);
    server_item1.add(&server_database_view_edit);

    (item_header_edit, server_item1, server_database_view_edit)
}

fn display_server_poi(poi: &ServerPointOfInterest, vbox: &gtk::Box) {
    let (_, server_item1, _) = server_poi_contents(poi, WidgetMode::Show);
    vbox.append(&server_item1);

    add_group_edit_suffix(
        &server_item1,
        glib::closure_local!(@strong poi as p, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let (header, server_item, _) = server_poi_contents(&p, WidgetMode::Edit);
            item_box.append(&header.unwrap());
            item_box.append(&server_item);

            display_item_edit_dialog(&v, "Edit POI", item_box, 600, 600, DialogClamp::Yes);
        }),
    );
}

fn server_poi_contents(
    poi: &ServerPointOfInterest,
    widget_mode: WidgetMode,
) -> (
    Option<ItemHeaderEdit>,
    adw::PreferencesGroup,
    ServerPoiViewEdit,
) {
    let item_header_edit = if widget_mode == WidgetMode::Edit {
        let server_item_header = ItemHeaderEdit::new(
            interest_type_get_icon(poi.interest_type),
            poi.group_name.as_deref(),
            &[], // TODO list of groups get_server_group_names(),
            common::EnvOrEnvs::None,
        );
        server_item_header.set_title(poi.desc.clone());
        Some(server_item_header)
    } else {
        None
    };

    let desc = match poi.interest_type {
        InterestType::PoiLogFile => "Log file",
        InterestType::PoiConfigFile => "Config file",
        InterestType::PoiApplication => "Application",
        InterestType::PoiCommandToRun => "Command to run",
        InterestType::PoiBackupArchive => "Backup/Archive",
        InterestType::PoiCommandTerminal => "Command to run",
    };
    let server_item1 = adw::PreferencesGroup::builder().build();

    if widget_mode == WidgetMode::Show {
        server_item1.set_description(Some(desc));
        server_item1.set_title(&poi.desc);
    }

    let server_poi_view_edit = ServerPoiViewEdit::new();
    server_poi_view_edit.set_interest_type(poi.interest_type.to_string());
    server_poi_view_edit.set_path(poi.path.to_string());
    server_poi_view_edit.set_text(poi.text.to_string());
    server_poi_view_edit.set_run_on(poi.run_on.to_string());
    server_poi_view_edit.prepare(widget_mode);
    server_item1.add(&server_poi_view_edit);

    (item_header_edit, server_item1, server_poi_view_edit)
}

fn display_server_extra_user_account(user: &ServerExtraUserAccount, vbox: &gtk::Box) {
    let (_, server_item1, _) = server_extra_user_account_contents(user, WidgetMode::Show);
    vbox.append(&server_item1);

    add_group_edit_suffix(
        &server_item1,
        glib::closure_local!(@strong user as u, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let (header, server_item, _) = server_extra_user_account_contents(&u, WidgetMode::Edit);
            item_box.append(&header.unwrap());
            item_box.append(&server_item);

            display_item_edit_dialog(&v, "Edit User Account", item_box, 600, 600, DialogClamp::Yes);
        }),
    );
    // TODO auth key
}

fn server_extra_user_account_contents(
    user: &ServerExtraUserAccount,
    widget_mode: WidgetMode,
) -> (
    Option<ItemHeaderEdit>,
    adw::PreferencesGroup,
    ServerExtraUserAccountViewEdit,
) {
    let item_header_edit = if widget_mode == WidgetMode::Edit {
        let user_item_header = ItemHeaderEdit::new(
            "user",
            user.group_name.as_deref(),
            &[], // TODO list of groups get_server_group_names(),
            common::EnvOrEnvs::None,
        );
        user_item_header.set_title(user.desc.clone());
        Some(user_item_header)
    } else {
        None
    };

    let server_item1 = adw::PreferencesGroup::builder().build();

    if widget_mode == WidgetMode::Show {
        server_item1.set_title(&user.desc);
    }

    let server_user_view_edit = ServerExtraUserAccountViewEdit::new();
    server_user_view_edit.set_username(user.username.to_string());
    server_user_view_edit.set_password(user.password.to_string());
    server_user_view_edit.prepare(widget_mode);
    server_item1.add(&server_user_view_edit);

    (item_header_edit, server_item1, server_user_view_edit)
}

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

fn display_server_note(note: &ServerNote, vbox: &gtk::Box, focused_server_item_id: Option<i32>) {
    let server_item1 = server_note_contents_show(note, focused_server_item_id, vbox);
    // let (note_view, note_view_scrolled_window) =
    //     note::get_note_contents_widget(&note.contents, widget_mode);

    add_group_edit_suffix(
        &server_item1,
        glib::closure_local!(@strong note as n, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let (header_edit, note) = server_note_contents_edit(&n, &item_box);
            item_box.set_margin_start(30);
            item_box.set_margin_end(30);

            let (dlg, save_btn) = display_item_edit_dialog(&v, "Edit Note", item_box, 6000, 6000, DialogClamp::No);
            save_btn.connect_clicked(move |_| {
                let text_edit_b = note.imp().text_edit.borrow();
                let text_edit = text_edit_b.as_ref().unwrap();

                let buf = text_edit.buffer();
                let start_iter = buf.start_iter();
                let end_iter = buf.end_iter();
                let new_contents = text_edit.buffer().text(&start_iter, &end_iter, false);

                let app = gio::Application::default()
                    .and_downcast::<ProjectpadApplication>()
                    .unwrap();
                let db_sender = app.get_sql_channel();

                let receiver = save_server_note(db_sender.clone(), n.server_id, Some(n.id),
                    header_edit.title(), new_contents);
                let dlg = dlg.clone();
                glib::spawn_future_local(async move {
                    let project_note_after_result = receiver.recv().await.unwrap();
                    let window = app.imp().window.get().unwrap();
                    let win_binding = window.upgrade();
                    let win_binding_ref = win_binding.as_ref().unwrap();
                    let pi = &win_binding_ref.imp().project_item;
                    let pi_bin = &win_binding_ref.imp().project_item.imp().project_item;
                    load_and_display_server(pi_bin, db_sender, n.server_id, Some(n.id), pi);
                    dlg.close();
                });
            });
        }),
    );
}

pub fn save_server(
    server_id: Option<i32>,
    new_env_type: EnvironmentType,
    new_is_retired: bool,
    new_desc: String,
    new_address: String,
    new_username: String,
    new_password: String,
    new_text: String,
    new_server_type: ServerType,
    new_server_access_type: ServerAccessType,
) -> async_channel::Receiver<Result<Server, (String, Option<String>)>> {
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    let db_sender = app.get_sql_channel();
    let (sender, receiver) = async_channel::bounded(1);
    let project_id = app.project_id().unwrap();

    // TODO commented fields (group and so on)
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server::dsl as srv;
            let changeset = (
                srv::desc.eq(new_desc.as_str()),
                srv::is_retired.eq(new_is_retired),
                srv::ip.eq(new_address.as_str()),
                srv::text.eq(new_text.as_str()),
                // never store Some("") for group, we want None then.
                // srv::group_name.eq(new_group
                //     .as_ref()
                //     .map(|s| s.as_str())
                //     .filter(|s| !s.is_empty())),
                srv::username.eq(new_username.as_str()),
                srv::password.eq(new_password.as_str()),
                // srv::auth_key.eq(new_authkey.as_ref()),
                // srv::auth_key_filename.eq(new_authkey_filename.as_ref()),
                srv::server_type.eq(new_server_type),
                srv::access_type.eq(new_server_access_type),
                srv::environment.eq(new_env_type),
                srv::project_id.eq(project_id),
            );
            let server_after_result = perform_insert_or_update!(
                sql_conn,
                server_id,
                srv::server,
                srv::id,
                changeset,
                Server,
            );
            sender.send_blocking(server_after_result).unwrap();
        }))
        .unwrap();
    receiver
}

fn save_server_note(
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    server_note_id: Option<i32>,
    new_title: String,
    new_contents: GString,
) -> async_channel::Receiver<Result<ServerNote, (String, Option<String>)>> {
    let (sender, receiver) = async_channel::bounded(1);
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server_note::dsl as srv_note;
            let changeset = (
                srv_note::title.eq(&new_title),
                // never store Some("") for group, we want None then.
                // srv_note::group_name.eq(new_group
                //     .as_ref()
                //     .map(|s| s.as_str())
                //     .filter(|s| !s.is_empty())),
                srv_note::contents.eq(new_contents.as_str()),
                srv_note::server_id.eq(server_id),
            );
            let server_note_after_result = perform_insert_or_update!(
                sql_conn,
                server_note_id,
                srv_note::server_note,
                srv_note::id,
                changeset,
                ServerNote,
            );
            sender.send_blocking(server_note_after_result).unwrap();
        }))
        .unwrap();
    receiver
}

fn server_note_contents_show(
    note: &ServerNote,
    focused_server_item_id: Option<i32>,
    vbox: &gtk::Box,
) -> adw::PreferencesGroup {
    let contents_head = notes::note_markdown_to_quick_preview(&note.contents)
        .lines()
        .take(3)
        .collect_vec()
        .join("⏎");

    let note_view = Note::new();
    // TODO call in the other order, it crashes. could put edit_mode in the ctor, but
    // it feels even worse (would like not to rebuild the widget every time...)
    note_view.set_server_note_id(note.id);
    note_view.set_edit_mode(false);

    note_view.set_height_request(500);

    let server_item1 = adw::PreferencesGroup::builder()
        .description("Note")
        .title(&note.title)
        .build();
    vbox.append(&server_item1);

    let row = adw::ExpanderRow::builder()
        .title(truncate(&contents_head, 120))
        // .css_classes(["property"])
        .build();

    row.add_row(&note_view);

    if Some(note.id) == focused_server_item_id {
        row.set_expanded(true);
    }

    server_item1.add(&row);

    server_item1
}

fn server_note_contents_edit(note: &ServerNote, vbox: &gtk::Box) -> (ItemHeaderEdit, Note) {
    let project_item_header = ItemHeaderEdit::new(
        ProjectItemType::ProjectNote.get_icon(),
        note.group_name.as_deref(),
        &[], // TODO list of groups
        common::EnvOrEnvs::None,
    );
    project_item_header.set_title(note.title.clone());
    vbox.append(&project_item_header);

    let note_view = Note::new();
    // TODO call in the other order, it crashes. could put edit_mode in the ctor, but
    // it feels even worse (would like not to rebuild the widget every time...)
    note_view.set_server_note_id(note.id);
    note_view.set_edit_mode(true);

    note_view.set_height_request(500);

    vbox.append(&note_view);

    (project_item_header, note_view)
}
