use crate::{
    app::ProjectpadApplication,
    notes, perform_insert_or_update,
    sql_thread::SqlFunc,
    widgets::{
        project_item::{ProjectItem, WidgetMode},
        project_item_model::ProjectItemType,
        project_items::common::{display_item_edit_dialog, get_project_group_names, DialogClamp},
    },
};
use adw::prelude::*;
use diesel::prelude::*;
use glib::GString;
use gtk::subclass::prelude::*;
use itertools::Itertools;
use projectpadsql::models::{
    InterestType, RunOn, Server, ServerAccessType, ServerDatabase, ServerExtraUserAccount,
    ServerNote, ServerPointOfInterest, ServerType, ServerWebsite,
};
use std::{
    collections::{BTreeSet, HashMap},
    sync::mpsc,
    time::Duration,
};

use super::{
    common::{self, DetailsRow, SuffixAction},
    project_item_header_edit::ProjectItemHeaderEdit,
    project_item_header_view::ProjectItemHeaderView,
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

                let mut dbs = databases.into_iter().map(ServerItem::Database);
                servers.extend(&mut dbs);

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
    let (header_box, vbox) = server_contents(
        &channel_data.server,
        &channel_data.project_group_names,
        WidgetMode::Show,
    );
    let add_btn = gtk::MenuButton::builder()
        .icon_name("list-add-symbolic")
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .popover(&add_server_item_popover(IncludeAddGroup::Yes))
        .build();
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
                let (_, vbox) = server_contents(&s, &pgn_, WidgetMode::Edit);

                display_item_edit_dialog(&v, "Edit Server", vbox, 600, 600, DialogClamp::Yes);
            }),
        );

    add_server_items(&channel_data, server_item_id, &vbox, project_item);

    parent.set_child(Some(&vbox));
}

// TODO kill DetailsRow use in this file, move everything to ServerViewEdit ####
pub fn server_contents(
    server: &Server,
    project_group_names: &[String],
    widget_mode: WidgetMode,
) -> (gtk::Box, gtk::Box) {
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

    DetailsRow::new("Text", &server.text, SuffixAction::copy(&server.text), &[])
        .add(widget_mode, &server_item0);

    if widget_mode == WidgetMode::Edit {
        // server type
        let server_type_combo = adw::ComboRow::new();
        server_type_combo.set_title("Server Type");
        let server_type_model = gtk::StringList::new(&[
            "Application",
            "Database",
            "HTTP server or proxy",
            "Monitoring",
            "Reporting",
        ]);
        server_type_combo.set_model(Some(&server_type_model));
        server_type_combo.set_selected(match server.server_type {
            ServerType::SrvApplication => 0,
            ServerType::SrvDatabase => 1,
            ServerType::SrvHttpOrProxy => 2,
            ServerType::SrvMonitoring => 3,
            ServerType::SrvReporting => 4,
        });

        server_item0.add(&server_type_combo);

        // access type
        let access_type_combo = adw::ComboRow::new();
        access_type_combo.set_title("Access Type");
        let access_type_model =
            gtk::StringList::new(&["Remote Desktop (RDP)", "SSH", "SSH Tunnel", "Website"]);
        access_type_combo.set_model(Some(&access_type_model));
        access_type_combo.set_selected(match server.access_type {
            ServerAccessType::SrvAccessRdp => 0,
            ServerAccessType::SrvAccessSsh => 1,
            ServerAccessType::SrvAccessSshTunnel => 2,
            ServerAccessType::SrvAccessWww => 3,
        });

        server_item0.add(&access_type_combo);
    }

    let header_box = if widget_mode == WidgetMode::Edit {
        let project_item_header = ProjectItemHeaderEdit::new(
            ProjectItemType::Server,
            server.group_name.as_deref(),
            project_group_names,
            common::EnvOrEnvs::Env(server.environment),
        );
        project_item_header.set_title(server.desc.clone());
        vbox.append(&project_item_header);
        project_item_header.header_box()
    } else {
        let project_item_header = ProjectItemHeaderView::new(ProjectItemType::Server);
        project_item_header.set_title(server.desc.clone());
        vbox.append(&project_item_header);
        project_item_header.header_box()
    };

    let server_view_edit = ServerViewEdit::new();
    server_view_edit.set_ip(server.ip.clone());
    server_view_edit.set_username(server.username.clone());
    server_view_edit.set_access_type(server.access_type.to_string());
    server_view_edit.set_password(server.password.clone());
    server_view_edit.prepare(widget_mode);
    vbox.append(&server_view_edit);

    vbox.append(&server_item0);

    (header_box, vbox)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IncludeAddGroup {
    Yes,
    No,
}

fn add_server_item_popover(include_add_group: IncludeAddGroup) -> gtk::PopoverMenu {
    let add_poi_menu = gio::Menu::new();
    add_poi_menu.append(Some("Application"), None);
    add_poi_menu.append(Some("Backup/Archive"), None);
    add_poi_menu.append(Some("Command to run"), None);
    add_poi_menu.append(Some("Config file"), None);
    add_poi_menu.append(Some("Log file"), None);

    let add_menu = gio::Menu::new();
    add_menu.append(Some("Website"), None);
    add_menu.append_submenu(Some("Point of interest"), &add_poi_menu);
    add_menu.append(Some("Note"), None);
    if include_add_group == IncludeAddGroup::Yes {
        add_menu.append(Some("Group"), None);
    }
    gtk::PopoverMenu::builder().menu_model(&add_menu).build()
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
            // TODO remove fallback
            _ => {}
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

    let edit_btn = gtk::Button::builder()
        .icon_name("document-edit-symbolic")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .build();
    frame_header.append(&edit_btn);

    let add_btn = gtk::MenuButton::builder()
        .icon_name("list-add-symbolic")
        .css_classes(["flat"])
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .popover(&add_server_item_popover(IncludeAddGroup::Yes))
        .build();
    frame_header.append(&add_btn);

    frame_box.append(&frame_header);
    frame_box.append(&gtk::Separator::builder().build());
    frame.set_child(Some(&frame_box));
    (frame, frame_box)
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
    let server_item1 = server_website_contents(w, WidgetMode::Show, vbox);
    add_group_edit_suffix(
        &server_item1,
        glib::closure_local!(@strong w as w1, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            server_website_contents(&w1, WidgetMode::Edit, &item_box);

            display_item_edit_dialog(&v, "Edit Website", item_box, 600, 600, DialogClamp::Yes);
        }),
    );
}

fn server_website_contents(
    w: &ServerWebsite,
    widget_mode: WidgetMode,
    vbox: &gtk::Box,
) -> adw::PreferencesGroup {
    let server_item1 = adw::PreferencesGroup::builder().build();
    if widget_mode == WidgetMode::Show {
        server_item1.set_title(&w.desc);
        server_item1.set_description(Some("Website"));
    } else {
        server_item1.set_title("Website");
        DetailsRow::new("Description", &w.desc, None, &[]).add(widget_mode, &server_item1);
    }
    DetailsRow::new("Address", &w.url, Some(SuffixAction::link(&w.url)), &[])
        .add(widget_mode, &server_item1);

    DetailsRow::new(
        "Username",
        &w.username,
        SuffixAction::copy(&w.username),
        &[],
    )
    .add(widget_mode, &server_item1);
    DetailsRow::new_password("Password", &w.password, SuffixAction::copy(&w.password))
        .add(widget_mode, &server_item1);
    DetailsRow::new("Text", &w.text, SuffixAction::copy(&w.text), &[])
        .add(widget_mode, &server_item1);

    vbox.append(&server_item1);

    server_item1
}

fn display_server_poi(poi: &ServerPointOfInterest, vbox: &gtk::Box) {
    let server_item1 = server_poi_contents(poi, WidgetMode::Show, vbox);

    add_group_edit_suffix(
        &server_item1,
        glib::closure_local!(@strong poi as p, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            server_poi_contents(&p, WidgetMode::Edit, &item_box);

            display_item_edit_dialog(&v, "Edit POI", item_box, 600, 600, DialogClamp::Yes);
        }),
    );
}

fn server_poi_contents(
    poi: &ServerPointOfInterest,
    widget_mode: WidgetMode,
    vbox: &gtk::Box,
) -> adw::PreferencesGroup {
    let desc = match poi.interest_type {
        InterestType::PoiLogFile => "Log file",
        InterestType::PoiConfigFile => "Config file",
        InterestType::PoiApplication => "Application",
        InterestType::PoiCommandToRun => "Command to run",
        InterestType::PoiBackupArchive => "Backup/Archive",
        InterestType::PoiCommandTerminal => "Command to run",
    };
    let server_item1 = adw::PreferencesGroup::builder()
        .description(desc)
        .title(&poi.desc)
        .build();
    DetailsRow::new("Description", &poi.desc, None, &[]).add(widget_mode, &server_item1);
    DetailsRow::new("Path", &poi.path, SuffixAction::copy(&poi.path), &[])
        .add(widget_mode, &server_item1);
    let field_name = match poi.interest_type {
        InterestType::PoiCommandToRun | InterestType::PoiCommandTerminal => "Command",
        _ => "Text",
    };
    DetailsRow::new(field_name, &poi.text, SuffixAction::copy(&poi.text), &[])
        .add(widget_mode, &server_item1);

    vbox.append(&server_item1);

    if widget_mode == WidgetMode::Edit {
        // run on
        let run_on_combo = adw::ComboRow::new();
        run_on_combo.set_title("Run on");
        let run_on_model = gtk::StringList::new(&["Client", "Server"]);
        run_on_combo.set_model(Some(&run_on_model));
        run_on_combo.set_selected(match poi.run_on {
            RunOn::RunOnClient => 0,
            RunOn::RunOnServer => 1,
        });

        server_item1.add(&run_on_combo);

        let interest_type_combo = adw::ComboRow::new();
        interest_type_combo.set_title("Interest Type");
        let interest_type_model = gtk::StringList::new(&[
            "Application",
            "Backup/archive",
            "Command to run",
            "Command to run (terminal)",
            "Config file",
            "Log file",
        ]);
        interest_type_combo.set_model(Some(&interest_type_model));
        interest_type_combo.set_selected(match poi.interest_type {
            InterestType::PoiApplication => 0,
            InterestType::PoiBackupArchive => 1,
            InterestType::PoiCommandToRun => 2,
            InterestType::PoiCommandTerminal => 3,
            InterestType::PoiConfigFile => 4,
            InterestType::PoiLogFile => 5,
        });
        server_item1.add(&interest_type_combo);
    }

    server_item1
}

fn display_server_extra_user_account(user: &ServerExtraUserAccount, vbox: &gtk::Box) {
    let server_item1 = server_extra_user_account_contents(user, WidgetMode::Show, vbox);

    add_group_edit_suffix(
        &server_item1,
        glib::closure_local!(@strong user as u, @strong vbox as v => move |_b: gtk::Button| {
            let item_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .build();
            server_extra_user_account_contents(&u, WidgetMode::Edit, &item_box);

            display_item_edit_dialog(&v, "Edit User Account", item_box, 600, 600, DialogClamp::Yes);
        }),
    );
    // TODO auth key
}

fn server_extra_user_account_contents(
    user: &ServerExtraUserAccount,
    widget_mode: WidgetMode,
    vbox: &gtk::Box,
) -> adw::PreferencesGroup {
    let server_item1 = adw::PreferencesGroup::builder()
        .description("Extra user")
        // .title(&poi.desc)
        .build();
    DetailsRow::new("Description", &user.desc, None, &[]).add(widget_mode, &server_item1);
    DetailsRow::new(
        "Username",
        &user.username,
        SuffixAction::copy(&user.username),
        &[],
    )
    .add(widget_mode, &server_item1);
    DetailsRow::new_password(
        "Password",
        &user.password,
        SuffixAction::copy(&user.password),
    )
    .add(widget_mode, &server_item1);

    vbox.append(&server_item1);

    server_item1
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

fn server_note_contents_edit(note: &ServerNote, vbox: &gtk::Box) -> (ProjectItemHeaderEdit, Note) {
    // TODO this is not a PROJECT note. rename ProjectItemHeaderEdit, and make it take an icon
    // instead of a ProjectItemType. Although I'm not sure yet what i'll do about groups
    let project_item_header = ProjectItemHeaderEdit::new(
        ProjectItemType::ProjectNote,
        None,
        &[],
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
