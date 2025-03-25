use crate::{
    app::ProjectpadApplication,
    notes, perform_insert_or_update,
    sql_thread::SqlFunc,
    sql_util,
    widgets::{
        project_item::{ProjectItem, WidgetMode},
        project_item_list::ProjectItemList,
        project_item_model::ProjectItemType,
        project_items::common::{
            confirm_delete, display_item_edit_dialog, run_sqlfunc_and_then, DialogClamp,
        },
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
        ServerExtraUserAccount, ServerLink, ServerNote, ServerPointOfInterest, ServerType,
        ServerWebsite,
    },
};
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    sync::mpsc,
    time::Duration,
};
use std::{path::Path, str::FromStr};

use super::{
    common::{self},
    item_header_edit::ItemHeaderEdit,
    project_poi::{project_item_header, DisplayHeaderMode},
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
    pub server: Server,
    pub server_items: Vec<ServerItem>,
    pub group_start_indices: HashMap<i32, String>,
    pub databases_for_websites: HashMap<i32, ServerDatabase>,
    pub websites_for_databases: HashMap<i32, Vec<ServerWebsite>>,
    pub project_group_names: Vec<String>,
    pub server_group_names: Vec<String>,
}

pub fn run_channel_data_query(sql_conn: &mut SqliteConnection, server_id: i32) -> ChannelData {
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
            .filter(srv_db::id.eq_any(server_websites.iter().filter_map(|w| w.server_database_id)))
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
        let mut dbs_as_server_items = databases.clone().into_iter().map(ServerItem::Database);
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
    let server_group_names = get_server_group_names(sql_conn, server.id);

    ChannelData {
        server,
        server_items: grouped_items.into_iter().cloned().collect(),
        group_start_indices,
        databases_for_websites,
        websites_for_databases,
        project_group_names,
        server_group_names,
    }
}

pub fn load_and_display_server(
    parent: &adw::Bin,
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    server_item_id: Option<i32>,
    project_item: &ProjectItem,
) {
    let p = parent.clone();
    let pi = project_item.clone();
    let (sender, receiver) = async_channel::bounded(1);
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            sender
                .send_blocking(run_channel_data_query(sql_conn, server_id))
                .unwrap();
        }))
        .unwrap();
    glib::spawn_future_local(async move {
        let channel_data = receiver.recv().await.unwrap();
        display_server(&p, channel_data, server_item_id, &pi);
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
    let server_group_names = channel_data.server_group_names.clone();
    add_btn.connect_clicked(move |_| {
        display_add_server_item_dialog(server_id, server_group_names.clone())
    });
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

    let project_id = channel_data.server.project_id;
    let server_id = channel_data.server.id;
    let server_name = channel_data.server.desc.clone();
    delete_btn.connect_closure(
        "clicked",
        false,
        glib::closure_local!(move |_b: gtk::Button| {
            confirm_delete(
                "Delete Server",
                &format!(
                    "Do you want to delete '{}'? This action cannot be undone.",
                    server_name
                ),
                Box::new(move || {
                    delete_server(project_id, server_id);
                }),
            )
        }),
    );

    let pgn = channel_data.project_group_names.clone();
    edit_btn.connect_closure("clicked", false,
            glib::closure_local!(@strong channel_data.server as s, @strong pgn as pgn_, @strong vbox as v => move |_b: gtk::Button| {
                let (_, header_edit, vbox, server_view_edit) = server_contents(&s, &pgn_, WidgetMode::Edit);

                let (dlg, save_btn) = display_item_edit_dialog(&v, "Edit Server", vbox, 600, 600, DialogClamp::Yes);
                let he = header_edit.unwrap().clone();
                let old_auth_key = s.auth_key.clone();
                let old_auth_key_filename = s.auth_key_filename.clone();
                save_btn.connect_clicked(move|_| {
                    let receiver = save_server(
                        Some(s.id),
                        he.group_name(),
                        he.single_env(),
                        server_view_edit.is_retired(),
                        he.title(),
                        server_view_edit.ip(),
                        server_view_edit.username(),
                        server_view_edit.password(),
                        server_view_edit.text(),
                        ServerType::from_str(&server_view_edit.property::<String>("server_type"))
                        .unwrap(),
                        ServerAccessType::from_str(&server_view_edit.property::<String>("access_type"))
                        .unwrap(),
                        old_auth_key.as_deref(),
                        old_auth_key_filename.as_deref(),
                        server_view_edit.auth_key_filename()
                    );

                    let app = gio::Application::default()
                        .and_downcast::<ProjectpadApplication>()
                        .unwrap();
                    let dlg = dlg.clone();
                    glib::spawn_future_local(async move {
                        let server_after_result = receiver.recv().await.unwrap();
                        if let Err((title, msg)) = server_after_result {
                            let dialog = adw::AlertDialog::new(Some(&title), msg.as_deref());
                            dialog.add_responses(&[("close", "_Close")]);
                            dialog.set_default_response(Some("close"));
                            let app = gio::Application::default()
                                .and_downcast::<ProjectpadApplication>()
                                .unwrap();
                            dialog.present(&app.active_window().unwrap());
                        }
                        let window = app.imp().window.get().unwrap();
                        let win_binding = window.upgrade();
                        let win_binding_ref = win_binding.as_ref().unwrap();
                        // let pi = &win_binding_ref.imp().project_item;
                        // let pi_bin = &win_binding_ref.imp().project_item.imp().project_item;
                        ProjectItemList::display_project_item(None, s.id, ProjectItemType::Server);
                        // load_and_display_server(pi_bin, db_sender, s.id, None, pi);
                        dlg.close();
                    });
                });
            }),
        );

    add_server_items(&channel_data, true, server_item_id, &vbox, project_item);

    parent.set_child(Some(&vbox));
}

fn delete_server(project_id: i32, server_id: i32) {
    run_sqlfunc_and_then(
        Box::new(move |sql_conn| {
            use projectpadsql::schema::server::dsl as srv;
            use projectpadsql::schema::server_database::dsl as db;
            use projectpadsql::schema::server_link::dsl as srv_link;
            use projectpadsql::schema::server_website::dsl as srvw;

            // we cannot delete a server if a database under it
            // is being used elsewhere
            let dependent_websites = srvw::server_website
                .inner_join(db::server_database)
                .filter(db::server_id.eq(server_id))
                .load::<(ServerWebsite, ServerDatabase)>(sql_conn)
                .unwrap();
            let dependent_serverlinks = srv_link::server_link
                .filter(srv_link::linked_server_id.eq(server_id))
                .load::<ServerLink>(sql_conn)
                .unwrap();
            if !dependent_websites.is_empty() {
                Err((
                    "Cannot delete server",
                    Some(format!(
                        "databases {} on that server are used by websites {}",
                        itertools::join(dependent_websites.iter().map(|(_, d)| &d.name), ", "),
                        itertools::join(dependent_websites.iter().map(|(w, _)| &w.desc), ", ")
                    )),
                ))
            } else if !dependent_serverlinks.is_empty() {
                Err((
                    "Cannot delete server",
                    Some(format!(
                        "server links {} are tied to this server",
                        itertools::join(dependent_serverlinks.iter().map(|l| &l.desc), ", "),
                    )),
                ))
            } else {
                sql_util::delete_row(sql_conn, srv::server, server_id)
                // .map(|_| ProjectItem::Server(server.clone()))
            }
        }),
        Box::new(move |res| match res {
            Ok(_) => {
                ProjectItemList::display_project(project_id);
            }
            Err((msg, details)) => {
                let dialog = adw::AlertDialog::new(Some(msg), details.as_deref());
                dialog.add_responses(&[("close", "_Close")]);
                dialog.set_default_response(Some("close"));
                let app = gio::Application::default()
                    .and_downcast::<ProjectpadApplication>()
                    .unwrap();
                dialog.present(&app.active_window().unwrap());
            }
        }),
    );
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

    let (project_item_header_edit, header_box) = project_item_header(
        &vbox,
        &server.desc,
        server.group_name.as_deref(),
        ProjectItemType::Server,
        common::EnvOrEnvs::Env(server.environment),
        project_group_names,
        widget_mode,
        DisplayHeaderMode::Yes,
    );

    let server_view_edit = server_view_edit_contents(server, widget_mode);
    vbox.append(&server_view_edit);

    vbox.append(&server_item0);

    (header_box, project_item_header_edit, vbox, server_view_edit)
}

pub fn server_view_edit_contents(server: &Server, widget_mode: WidgetMode) -> ServerViewEdit {
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

    if let Some(auth_key) = server.auth_key.as_ref() {
        let auth_key_owned = auth_key.to_vec();
        connect_save_auth_key(&server_view_edit, auth_key_owned);
    }

    server_view_edit.prepare(widget_mode);
    server_view_edit
}

fn connect_save_auth_key<T: ObjectExt>(obj: &T, auth_key: Vec<u8>) {
    obj.connect_closure(
            "save-auth-key-to-disk",
            false,
            glib::closure_local!(@strong auth_key as contents => move |_: T, path: String| {
                if let Err(e) = std::fs::write(path, &contents) {
                    let dialog = adw::AlertDialog::new(Some("Error saving auth key"), Some(&format!("{e}")));
                    dialog.add_responses(&[("close", "_Close")]);
                    dialog.set_default_response(Some("close"));
                    let app = gio::Application::default()
                        .and_downcast::<ProjectpadApplication>()
                        .unwrap();
                    dialog.present(&app.active_window().unwrap());
                }
            }),
        );
}

pub fn add_server_items(
    channel_data: &ChannelData,
    read_write: bool,
    focused_server_item_id: Option<i32>,
    vbox: &gtk::Box,
    project_item: &ProjectItem,
) {
    let mut cur_parent = vbox.clone();
    let mut cur_group_name = None::<&str>;

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
            ServerItem::Website(w) => display_server_website(
                channel_data.server.id,
                read_write,
                channel_data.server_group_names.clone(),
                w,
                &cur_parent,
            ),
            ServerItem::PointOfInterest(poi) => display_server_poi(
                channel_data.server.id,
                read_write,
                channel_data.server_group_names.clone(),
                poi,
                &cur_parent,
            ),
            ServerItem::Note(n) => display_server_note(
                n,
                read_write,
                channel_data.server_group_names.clone(),
                &cur_parent,
                focused_server_item_id,
            ),
            ServerItem::ExtraUserAccount(u) => display_server_extra_user_account(
                channel_data.server.id,
                read_write,
                channel_data.server_group_names.clone(),
                u,
                &cur_parent,
            ),
            ServerItem::Database(u) => display_server_database(
                channel_data.server.id,
                read_write,
                channel_data.server_group_names.clone(),
                u,
                &cur_parent,
            ),
        }
        if Some(server_item.get_id()) == focused_server_item_id {
            let me = cur_parent.last_child().unwrap().clone();
            let v = vbox.clone();

            let pi = project_item.clone();
            glib::spawn_future_local(async move {
                // TODO crappy but doesn't work without the wait..
                // try glib::idle_add_local instead
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

fn display_add_server_item_dialog(server_id: i32, server_group_names: Vec<String>) {
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

    let dlg = dialog.clone();
    cancel_btn.connect_clicked(move |_btn: &gtk::Button| {
        dlg.close();
    });

    let s = stack.clone();
    let dlg = dialog.clone();
    let (header_edit, server_contents_child, server_view_edit) = server_poi_contents(
        &server_group_names,
        &ServerPointOfInterest::default(),
        WidgetMode::Edit,
    );
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
    let (header_edit, server_contents_child, server_view_edit) = server_website_contents(
        &server_group_names,
        &ServerWebsite::default(),
        WidgetMode::Edit,
    );
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
    let (header_edit, server_contents_child, server_view_edit) = server_database_contents(
        &server_group_names,
        &ServerDatabase::default(),
        WidgetMode::Edit,
    );
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
    let (header_edit, server_contents_child, server_view_edit) = server_extra_user_account_contents(
        &server_group_names,
        &ServerExtraUserAccount::default(),
        WidgetMode::Edit,
    );
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

    let s = stack.clone();
    let dlg = dialog.clone();
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let (header_edit, server_view_edit) =
        server_note_contents_edit(&ServerNote::default(), &server_group_names, &vbox);
    let hb = header_bar.clone();
    let he = header_edit.clone();
    note_btn.connect_clicked(move |_| {
        prepare_add_server_note_dlg(
            server_id,
            &dlg,
            &s,
            &hb,
            &he,
            &server_view_edit,
            // &server_contents_child,
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
    server_poi_connect_save(&save_btn, dlg, he, server_poi_view_edit, server_id, None);
    hb.pack_end(&save_btn);
}

fn server_poi_connect_save(
    save_btn: &gtk::Button,
    dlg: &adw::Dialog,
    he: &ItemHeaderEdit,
    server_poi_view_edit: &ServerPoiViewEdit,
    server_id: i32,
    server_poi_id: Option<i32>,
) {
    let d = dlg.clone();
    let server_poi_view_edit = server_poi_view_edit.clone();
    let he = he.clone();
    save_btn.connect_clicked(move |_| {
        let receiver = save_server_poi(
            server_id,
            server_poi_id,
            he.group_name(),
            he.title(),
            server_poi_view_edit.path(),
            server_poi_view_edit.text(),
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
                    None,
                    server_poi.server_id,
                    ProjectItemType::Server,
                );
            }
        });
    });
}

pub fn save_server_poi(
    server_id: i32,
    server_poi_id: Option<i32>,
    new_group_name: String,
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

    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
            let changeset = (
                srv_poi::desc.eq(new_desc.as_str()),
                srv_poi::path.eq(new_path.as_str()),
                srv_poi::text.eq(new_text.as_str()),
                // never store Some("") for group, we want None then.
                srv_poi::group_name.eq(Some(&new_group_name).filter(|s| !s.is_empty())),
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
    server_website_connect_save(
        &save_btn,
        dlg,
        he,
        server_website_view_edit,
        server_id,
        None,
    );
    hb.pack_end(&save_btn);
}

fn server_website_connect_save(
    save_btn: &gtk::Button,
    dlg: &adw::Dialog,
    he: &ItemHeaderEdit,
    server_website_view_edit: &ServerWebsiteViewEdit,
    server_id: i32,
    server_www_id: Option<i32>,
) {
    let d = dlg.clone();
    let server_poi_view_edit = server_website_view_edit.clone();
    let he = he.clone();
    save_btn.connect_clicked(move |_| {
        let receiver = save_server_website(
            server_id,
            server_www_id,
            he.group_name(),
            he.title(),
            server_poi_view_edit.url(),
            server_poi_view_edit.text(),
            server_poi_view_edit.username(),
            server_poi_view_edit.password(),
        );
        let d = d.clone();
        glib::spawn_future_local(async move {
            let server_poi_after_result = receiver.recv().await.unwrap();
            d.close();

            if let Ok(server_poi) = server_poi_after_result {
                ProjectItemList::display_project_item(
                    None,
                    server_poi.server_id,
                    ProjectItemType::Server,
                );
            }
        });
    });
}

pub fn save_server_website(
    server_id: i32,
    server_www_id: Option<i32>,
    new_group_name: String,
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

    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server_website::dsl as srv_www;
            let changeset = (
                srv_www::desc.eq(new_desc.as_str()),
                srv_www::url.eq(new_url.as_str()),
                srv_www::text.eq(new_text.as_str()),
                // never store Some("") for group, we want None then.
                srv_www::group_name.eq(Some(&new_group_name).filter(|s| !s.is_empty())),
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
    server_database_connect_save(
        &save_btn,
        dlg,
        he,
        server_database_view_edit,
        server_id,
        None,
    );
    hb.pack_end(&save_btn);
}

fn server_database_connect_save(
    save_btn: &gtk::Button,
    dlg: &adw::Dialog,
    he: &ItemHeaderEdit,
    server_database_view_edit: &ServerDatabaseViewEdit,
    server_id: i32,
    server_db_id: Option<i32>,
) {
    let d = dlg.clone();
    let server_db_view_edit = server_database_view_edit.clone();
    let he = he.clone();
    save_btn.connect_clicked(move |_| {
        let receiver = save_server_database(
            server_id,
            server_db_id,
            he.group_name(),
            he.title(),
            server_db_view_edit.name(),
            server_db_view_edit.text(),
            server_db_view_edit.username(),
            server_db_view_edit.password(),
        );
        let d = d.clone();
        glib::spawn_future_local(async move {
            let server_poi_after_result = receiver.recv().await.unwrap();
            d.close();

            if let Ok(server_poi) = server_poi_after_result {
                ProjectItemList::display_project_item(
                    None,
                    server_poi.server_id,
                    ProjectItemType::Server,
                );
            }
        });
    });
}

pub fn save_server_database(
    server_id: i32,
    server_db_id: Option<i32>,
    new_group_name: String,
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

    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            use projectpadsql::schema::server_database::dsl as srv_db;
            let changeset = (
                srv_db::desc.eq(new_desc.as_str()),
                srv_db::name.eq(new_name.as_str()),
                srv_db::text.eq(new_text.as_str()),
                // never store Some("") for group, we want None then.
                srv_db::group_name.eq(Some(&new_group_name).filter(|s| !s.is_empty())),
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
    server_extra_user_account_connect_save(
        &save_btn,
        dlg,
        he,
        server_user_view_edit,
        server_id,
        None,
        None,
        None,
    );
    hb.pack_end(&save_btn);
}

fn server_extra_user_account_connect_save(
    save_btn: &gtk::Button,
    dlg: &adw::Dialog,
    he: &ItemHeaderEdit,
    server_user_view_edit: &ServerExtraUserAccountViewEdit,
    server_id: i32,
    server_user_id: Option<i32>,
    old_auth_key: Option<Vec<u8>>,
    old_auth_key_filename: Option<String>,
) {
    let d = dlg.clone();
    let server_db_view_edit = server_user_view_edit.clone();
    let he = he.clone();
    save_btn.connect_clicked(move |_| {
        let receiver = save_server_extra_user_account(
            server_id,
            server_user_id,
            he.group_name(),
            he.title(),
            old_auth_key.as_deref(),
            old_auth_key_filename.as_deref(),
            server_db_view_edit.auth_key_filename(),
            server_db_view_edit.username(),
            server_db_view_edit.password(),
        );
        let d = d.clone();
        glib::spawn_future_local(async move {
            let server_poi_after_result = receiver.recv().await.unwrap();
            d.close();

            if let Ok(server_poi) = server_poi_after_result {
                ProjectItemList::display_project_item(
                    None,
                    server_poi.server_id,
                    ProjectItemType::Server,
                );
            }
        });
    });
}

fn prepare_add_server_note_dlg(
    server_id: i32,
    dlg: &adw::Dialog,
    s: &gtk::Stack,
    hb: &adw::HeaderBar,
    he: &ItemHeaderEdit,
    server_note: &Note,
    // server_contents_child: &adw::PreferencesGroup,
) {
    dlg.set_title("Add Server note");
    dlg.set_content_width(6000);
    dlg.set_content_height(6000);
    let contents = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_start(30)
        .margin_end(30)
        .build();
    contents.append(he);
    contents.append(server_note);
    // contents.append(server_contents_child);
    s.add_named(&contents, Some("second"));
    s.set_visible_child_name("second");

    let save_btn = gtk::Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let d = dlg.clone();
    let server_note = server_note.clone();
    let he = he.clone();
    save_btn.connect_clicked(move |_| {
        let app = gio::Application::default()
            .and_downcast::<ProjectpadApplication>()
            .unwrap();
        let db_sender = app.get_sql_channel();
        let receiver = save_server_note(
            db_sender,
            server_id,
            he.group_name(),
            None,
            he.title(),
            server_note.get_contents_text(),
        );
        let d = d.clone();
        glib::spawn_future_local(async move {
            let server_poi_after_result = receiver.recv().await.unwrap();
            d.close();

            if let Ok(server_poi) = server_poi_after_result {
                ProjectItemList::display_project_item(
                    None,
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
    new_group_name: String,
    new_desc: String,
    old_auth_key: Option<&[u8]>,
    old_auth_key_filename: Option<&str>,
    new_auth_key_filename: String,
    new_username: String,
    new_password: String,
) -> async_channel::Receiver<Result<ServerExtraUserAccount, (String, Option<String>)>> {
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    let db_sender = app.get_sql_channel();
    let (sender, receiver) = async_channel::bounded(1);

    let old_auth_key_filename_owned_str = old_auth_key_filename.unwrap_or("").to_owned();
    let old_auth_key_owned = old_auth_key.map(|k| k.to_vec());
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            let (sql_auth_key_filename, sql_auth_key_contents) = save_auth_key_get_new_vals(
                &new_auth_key_filename,
                &old_auth_key_filename_owned_str,
                old_auth_key_owned.clone(),
            );
            match sql_auth_key_contents {
                Ok(auth_key_contents) => {
                    use projectpadsql::schema::server_extra_user_account::dsl as srv_user;
                    let changeset = (
                        srv_user::desc.eq(new_desc.as_str()),
                        srv_user::auth_key.eq(auth_key_contents.as_ref()),
                        srv_user::auth_key_filename.eq(sql_auth_key_filename.as_ref()),
                        // never store Some("") for group, we want None then.
                        srv_user::group_name.eq(Some(&new_group_name).filter(|s| !s.is_empty())),
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
                }
                Err(e) => {
                    sender
                        .send_blocking(Err((
                            "Error reading the auth key file".to_string(),
                            Some(e.to_string()),
                        )))
                        .unwrap();
                }
            }
        }))
        .unwrap();
    receiver
}

fn add_group_edit_suffix(
    server_item1: &adw::PreferencesGroup,
    sensitive: bool,
    edit_closure: glib::RustClosure,
) -> gtk::Button {
    let edit_btn = gtk::Button::builder()
        .icon_name("document-edit-symbolic")
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .sensitive(sensitive)
        .build();
    let delete_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk::Align::Center)
        .sensitive(sensitive)
        .build();
    edit_btn.connect_closure("clicked", false, edit_closure);
    let suffix_box = gtk::Box::builder().css_classes(["toolbar"]).build();
    suffix_box.append(&edit_btn);
    suffix_box.append(&delete_btn);
    server_item1.set_header_suffix(Some(&suffix_box));
    delete_btn
}

fn display_server_website(
    server_id: i32,
    read_write: bool,
    server_group_names: Vec<String>,
    w: &ServerWebsite,
    vbox: &gtk::Box,
) {
    let (_, server_item1, _) = server_website_contents(&server_group_names, w, WidgetMode::Show);
    vbox.append(&server_item1);

    let www_id = w.id;
    let delete_btn = add_group_edit_suffix(
        &server_item1,
        read_write,
        glib::closure_local!(@strong w as w1, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let (header, server_item, server_website_view_edit) = server_website_contents(&server_group_names, &w1, WidgetMode::Edit);
            item_box.append(&header.clone().unwrap());
            item_box.append(&server_item);

            let (dlg, save_btn) = display_item_edit_dialog(&v, "Edit Website", item_box, 600, 600, DialogClamp::Yes);
            server_website_connect_save(&save_btn, &dlg, header.as_ref().unwrap(), &server_website_view_edit, server_id, Some(www_id));
        }),
    );

    let www_name = &w.desc;
    let www_id = w.id;
    delete_btn.connect_closure(
        "clicked",
        false,
        glib::closure_local!(@strong www_name as www_n, @strong www_id as w_id, @strong server_id as sid => move |_b: gtk::Button| {
            confirm_delete(
                "Delete Server Website",
                &format!(
                    "Do you want to delete '{}'? This action cannot be undone.",
                    www_n
                ),
                Box::new(move || {
                    run_sqlfunc_and_then(
                        Box::new(move |sql_conn| {
                            use projectpadsql::schema::server_website::dsl as srv_www;
                            sql_util::delete_row(sql_conn, srv_www::server_website, w_id)
                                .unwrap();
                        }),
                        Box::new(move |_| {
                            ProjectItemList::display_project_item(None, sid, ProjectItemType::Server);
                        }),
                    );
                }),
            )
        }),
    );
}

fn server_website_contents(
    server_group_names: &[String],
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
            server_group_names,
            common::EnvOrEnvs::None,
        );
        website_item_header.set_title(website.desc.clone());
        Some(website_item_header)
    } else {
        None
    };

    let server_item1 = adw::PreferencesGroup::builder().build();

    if widget_mode == WidgetMode::Show {
        server_item1.set_title(&glib::markup_escape_text(&website.desc));
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

fn display_server_database(
    server_id: i32,
    read_write: bool,
    server_group_names: Vec<String>,
    db: &ServerDatabase,
    vbox: &gtk::Box,
) {
    let (_, server_item1, _) = server_database_contents(&server_group_names, db, WidgetMode::Show);
    vbox.append(&server_item1);

    let db_id = db.id;
    let delete_btn = add_group_edit_suffix(
        &server_item1,
        read_write,
        glib::closure_local!(@strong db as w1, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let (header, server_item, server_database_view_edit) = server_database_contents(&server_group_names, &w1, WidgetMode::Edit);
            item_box.append(&header.clone().unwrap());
            item_box.append(&server_item);

            let (dlg, save_btn) = display_item_edit_dialog(&v, "Edit Database", item_box, 600, 600, DialogClamp::Yes);
            server_database_connect_save(&save_btn, &dlg, header.as_ref().unwrap(), &server_database_view_edit, server_id, Some(db_id));
        }),
    );

    let db_name = &db.desc;
    let db_id = db.id;
    delete_btn.connect_closure(
        "clicked",
        false,
        glib::closure_local!(@strong db_name as db_n, @strong db_id as w_id, @strong server_id as sid => move |_b: gtk::Button| {
            confirm_delete(
                "Delete Server Database",
                &format!(
                    "Do you want to delete '{}'? This action cannot be undone.",
                    db_n
                ),
                Box::new(move || {
                    run_sqlfunc_and_then(
                        Box::new(move |sql_conn| {
                            use projectpadsql::schema::server_database::dsl as srv_db;
                            sql_util::delete_row(sql_conn, srv_db::server_database, w_id)
                                .unwrap();
                        }),
                        Box::new(move |_| {
                            ProjectItemList::display_project_item(None, sid, ProjectItemType::Server);
                        }),
                    );
                }),
            )
        }),
    );
}

fn server_database_contents(
    server_group_names: &[String],
    database: &ServerDatabase,
    widget_mode: WidgetMode,
) -> (
    Option<ItemHeaderEdit>,
    adw::PreferencesGroup,
    ServerDatabaseViewEdit,
) {
    let item_header_edit = if widget_mode == WidgetMode::Edit {
        let database_item_header = ItemHeaderEdit::new(
            "database",
            database.group_name.as_deref(),
            server_group_names,
            common::EnvOrEnvs::None,
        );
        database_item_header.set_title(database.desc.clone());
        Some(database_item_header)
    } else {
        None
    };

    let server_item1 = adw::PreferencesGroup::builder().build();

    if widget_mode == WidgetMode::Show {
        server_item1.set_title(&glib::markup_escape_text(&database.desc));
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

fn display_server_poi(
    server_id: i32,
    read_write: bool,
    server_group_names: Vec<String>,
    poi: &ServerPointOfInterest,
    vbox: &gtk::Box,
) {
    let (_, server_item1, _) = server_poi_contents(&server_group_names, poi, WidgetMode::Show);
    vbox.append(&server_item1);

    let poi_id = poi.id;
    let delete_btn = add_group_edit_suffix(
        &server_item1,
        read_write,
        glib::closure_local!(@strong poi as p, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let (header, server_item, server_poi_view_edit) = server_poi_contents(&server_group_names, &p, WidgetMode::Edit);
            item_box.append(&header.clone().unwrap());
            item_box.append(&server_item);

            let (dlg, save_btn) = display_item_edit_dialog(&v, "Edit POI", item_box, 600, 600, DialogClamp::Yes);
            server_poi_connect_save(&save_btn, &dlg, header.as_ref().unwrap(), &server_poi_view_edit, server_id, Some(poi_id));
        }),
    );

    let poi_name = &poi.desc;
    let poi_id = poi.id;
    delete_btn.connect_closure(
        "clicked",
        false,
        glib::closure_local!(@strong poi_name as poi_n, @strong poi_id as w_id, @strong server_id as sid => move |_b: gtk::Button| {
            confirm_delete(
                "Delete Server POI",
                &format!(
                    "Do you want to delete '{}'? This action cannot be undone.",
                    poi_n
                ),
                Box::new(move || {
                    run_sqlfunc_and_then(
                        Box::new(move |sql_conn| {
                            use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
                            sql_util::delete_row(sql_conn, srv_poi::server_point_of_interest, w_id)
                                .unwrap();
                        }),
                        Box::new(move |_| {
                            ProjectItemList::display_project_item(None, sid, ProjectItemType::Server);
                        }),
                    );
                }),
            )
        }),
    );
}

fn server_poi_contents(
    server_group_names: &[String],
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
            server_group_names,
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
        server_item1.set_title(&glib::markup_escape_text(&poi.desc));
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

fn display_server_extra_user_account(
    server_id: i32,
    read_write: bool,
    server_group_names: Vec<String>,
    user: &ServerExtraUserAccount,
    vbox: &gtk::Box,
) {
    let (_, server_item1, _) =
        server_extra_user_account_contents(&server_group_names, user, WidgetMode::Show);
    vbox.append(&server_item1);

    let user_id = user.id;
    let delete_btn = add_group_edit_suffix(
        &server_item1,
        read_write,
        glib::closure_local!(@strong user as u, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let (header, server_item, server_extra_user_view_edit) = server_extra_user_account_contents(&server_group_names, &u, WidgetMode::Edit);
            item_box.append(&header.clone().unwrap());
            item_box.append(&server_item);

            let (dlg, save_btn) = display_item_edit_dialog(&v, "Edit User Account", item_box, 600, 600, DialogClamp::Yes);
            let old_auth_key = u.auth_key.clone();
            let old_auth_key_filename = u.auth_key_filename.clone();
            server_extra_user_account_connect_save(
                &save_btn, &dlg, header.as_ref().unwrap(), &server_extra_user_view_edit,
                server_id, Some(user_id), old_auth_key, old_auth_key_filename);
        }),
    );

    let user_name = &user.desc;
    let user_id = user.id;
    delete_btn.connect_closure(
        "clicked",
        false,
        glib::closure_local!(@strong user_name as user_n, @strong user_id as w_id, @strong server_id as sid => move |_b: gtk::Button| {
            confirm_delete(
                "Delete Server Extra User Account",
                &format!(
                    "Do you want to delete '{}'? This action cannot be undone.",
                    user_n
                ),
                Box::new(move || {
                    run_sqlfunc_and_then(
                        Box::new(move |sql_conn| {
                            use projectpadsql::schema::server_extra_user_account::dsl as srv_user;
                            sql_util::delete_row(sql_conn, srv_user::server_extra_user_account, w_id)
                                .unwrap();
                        }),
                        Box::new(move |_| {
                            ProjectItemList::display_project_item(None, sid, ProjectItemType::Server);
                        }),
                    );
                }),
            )
        }),
    );
}

fn server_extra_user_account_contents(
    server_group_names: &[String],
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
            server_group_names,
            common::EnvOrEnvs::None,
        );
        user_item_header.set_title(user.desc.clone());
        Some(user_item_header)
    } else {
        None
    };

    let server_item1 = adw::PreferencesGroup::builder().build();

    if widget_mode == WidgetMode::Show {
        server_item1.set_title(&glib::markup_escape_text(&user.desc));
    }

    let server_user_view_edit = ServerExtraUserAccountViewEdit::new();
    server_user_view_edit.set_username(user.username.to_string());
    server_user_view_edit.set_password(user.password.to_string());
    server_user_view_edit.set_auth_key_filename(
        user.auth_key_filename
            .clone()
            .unwrap_or_else(|| "".to_string()),
    );

    if let Some(auth_key) = user.auth_key.as_ref() {
        let auth_key_owned = auth_key.to_vec();
        connect_save_auth_key(&server_user_view_edit, auth_key_owned);
    }

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

fn display_server_note(
    note: &ServerNote,
    read_write: bool,
    server_group_names: Vec<String>,
    vbox: &gtk::Box,
    focused_server_item_id: Option<i32>,
) {
    let server_item1 = server_note_contents_show(note, focused_server_item_id, vbox);
    // let (note_view, note_view_scrolled_window) =
    //     note::get_note_contents_widget(&note.contents, widget_mode);

    let delete_btn = add_group_edit_suffix(
        &server_item1,
        read_write,
        glib::closure_local!(@strong note as n, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let (header_edit, note) = server_note_contents_edit(&n, &server_group_names, &item_box);
            item_box.set_margin_start(30);
            item_box.set_margin_end(30);

            let (dlg, save_btn) = display_item_edit_dialog(&v, "Edit Note", item_box, 6000, 6000, DialogClamp::No);
            save_btn.connect_clicked(move |_| {
                let new_contents = note.get_contents_text();

                let app = gio::Application::default()
                    .and_downcast::<ProjectpadApplication>()
                    .unwrap();
                let db_sender = app.get_sql_channel();

                let receiver = save_server_note(db_sender.clone(), n.server_id, header_edit.group_name(), Some(n.id),
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

    let note_name = &note.title;
    let note_id = note.id;
    let server_id = note.server_id;
    delete_btn.connect_closure(
        "clicked",
        false,
        glib::closure_local!(@strong note_name as note_n, @strong note_id as w_id, @strong server_id as sid => move |_b: gtk::Button| {
            confirm_delete(
                "Delete Server Note",
                &format!(
                    "Do you want to delete '{}'? This action cannot be undone.",
                    note_n
                ),
                Box::new(move || {
                    run_sqlfunc_and_then(
                        Box::new(move |sql_conn| {
                            use projectpadsql::schema::server_note::dsl as srv_note;
                            sql_util::delete_row(sql_conn, srv_note::server_note, w_id)
                                .unwrap();
                        }),
                        Box::new(move |_| {
                            ProjectItemList::display_project_item(None, sid, ProjectItemType::Server);
                        }),
                    );
                }),
            )
        }),
    );
}

fn save_auth_key_get_new_vals<'a>(
    new_auth_key_filename: &str,
    old_auth_key_filename_owned_str: &'a str,
    old_auth_key_owned: Option<Vec<u8>>,
) -> (Option<Cow<'a, str>>, std::io::Result<Option<Vec<u8>>>) {
    if new_auth_key_filename != old_auth_key_filename_owned_str {
        if new_auth_key_filename == "" {
            (None, Ok(None))
        } else {
            (
                Path::new(&new_auth_key_filename)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| Cow::Owned(s.to_string())),
                std::fs::read(&new_auth_key_filename).map(|v| Some(v)),
            )
        }
    } else {
        (
            Some(Cow::Borrowed(&old_auth_key_filename_owned_str)),
            Ok(old_auth_key_owned.clone()),
        )
    }
}

pub fn save_server(
    server_id: Option<i32>,
    new_group_name: String,
    new_env_type: EnvironmentType,
    new_is_retired: bool,
    new_desc: String,
    new_address: String,
    new_username: String,
    new_password: String,
    new_text: String,
    new_server_type: ServerType,
    new_server_access_type: ServerAccessType,
    old_auth_key: Option<&[u8]>,
    old_auth_key_filename: Option<&str>,
    new_auth_key_filename: String,
) -> async_channel::Receiver<Result<Server, (String, Option<String>)>> {
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    let db_sender = app.get_sql_channel();
    let (sender, receiver) = async_channel::bounded(1);
    let project_id = app.project_id().unwrap();

    let old_auth_key_filename_owned_str = old_auth_key_filename.unwrap_or("").to_owned();
    let old_auth_key_owned = old_auth_key.map(|k| k.to_vec());
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            let (sql_auth_key_filename, sql_auth_key_contents) = save_auth_key_get_new_vals(
                &new_auth_key_filename,
                &old_auth_key_filename_owned_str,
                old_auth_key_owned.clone(),
            );
            match sql_auth_key_contents {
                Ok(auth_key_contents) => {
                    use projectpadsql::schema::server::dsl as srv;
                    let changeset = (
                        srv::desc.eq(new_desc.as_str()),
                        srv::is_retired.eq(new_is_retired),
                        srv::ip.eq(new_address.as_str()),
                        srv::text.eq(new_text.as_str()),
                        // never store Some("") for group, we want None then.
                        srv::group_name.eq(Some(&new_group_name).filter(|s| !s.is_empty())),
                        srv::username.eq(new_username.as_str()),
                        srv::password.eq(new_password.as_str()),
                        srv::auth_key.eq(auth_key_contents.as_ref()),
                        srv::auth_key_filename.eq(sql_auth_key_filename.as_ref()),
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
                }
                Err(e) => {
                    sender
                        .send_blocking(Err((
                            "Error reading the auth key file".to_string(),
                            Some(e.to_string()),
                        )))
                        .unwrap();
                }
            }
        }))
        .unwrap();
    receiver
}

fn save_server_note(
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    new_group_name: String,
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
                srv_note::group_name.eq(Some(&new_group_name).filter(|s| !s.is_empty())),
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

fn server_note_contents_edit(
    note: &ServerNote,
    server_group_names: &[String],
    vbox: &gtk::Box,
) -> (ItemHeaderEdit, Note) {
    let project_item_header = ItemHeaderEdit::new(
        ProjectItemType::ProjectNote.get_icon(),
        note.group_name.as_deref(),
        server_group_names,
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
