use crate::{
    notes,
    sql_thread::SqlFunc,
    widgets::project_item::{ProjectItem, ProjectItemEditMode, WidgetMode},
};
use adw::prelude::*;
use diesel::prelude::*;
use itertools::Itertools;
use projectpadsql::models::{
    InterestType, RunOn, Server, ServerAccessType, ServerDatabase, ServerExtraUserAccount,
    ServerNote, ServerPointOfInterest, ServerType, ServerWebsite,
};
use std::{
    collections::{BTreeSet, HashMap},
    rc::Rc,
    sync::mpsc,
    time::Duration,
};

use super::{
    common::{self, DetailsRow, SuffixAction},
    note,
};

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
}

pub fn load_and_display_server(
    parent: &adw::Bin,
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    server_item_id: Option<i32>,
    widget_mode: WidgetMode,
    project_item: &ProjectItem,
) {
    dbg!(&server_id);
    dbg!(&server_item_id);
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

            sender
                .send_blocking(ChannelData {
                    server,
                    server_items: grouped_items.into_iter().cloned().collect(),
                    group_start_indices,
                    databases_for_websites,
                    websites_for_databases,
                })
                .unwrap();
        }))
        .unwrap();

    let p = parent.clone();
    let mut pi = project_item.clone();
    glib::spawn_future_local(async move {
        let channel_data = receiver.recv().await.unwrap();
        display_server(&p, channel_data, server_item_id, widget_mode, &mut pi);
    });
}

fn display_server(
    parent: &adw::Bin,
    channel_data: ChannelData,
    server_item_id: Option<i32>,
    widget_mode: WidgetMode,
    project_item: &ProjectItem,
) {
    let vbox = common::get_contents_box_with_header(
        &channel_data.server.desc,
        channel_data.server.group_name.as_deref(),
        common::EnvOrEnvs::Env(channel_data.server.environment),
        widget_mode,
    );

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

    let address_suffix_www = [SuffixAction::link(&channel_data.server.ip)];
    DetailsRow::new(
        "Address",
        &channel_data.server.ip,
        SuffixAction::copy(&channel_data.server.ip),
        if channel_data.server.access_type == ServerAccessType::SrvAccessWww {
            &address_suffix_www
        } else {
            &[]
        },
    )
    .add(widget_mode, &server_item0);

    DetailsRow::new(
        "Username",
        &channel_data.server.username,
        SuffixAction::copy(&channel_data.server.username),
        &[],
    )
    .add(widget_mode, &server_item0);

    DetailsRow::new_password(
        "Password",
        &channel_data.server.password,
        SuffixAction::copy(&channel_data.server.password),
    )
    .add(widget_mode, &server_item0);
    DetailsRow::new(
        "Text",
        &channel_data.server.text,
        SuffixAction::copy(&channel_data.server.text),
        &[],
    )
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
        server_type_combo.set_selected(match channel_data.server.server_type {
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
        access_type_combo.set_selected(match channel_data.server.access_type {
            ServerAccessType::SrvAccessRdp => 0,
            ServerAccessType::SrvAccessSsh => 1,
            ServerAccessType::SrvAccessSshTunnel => 2,
            ServerAccessType::SrvAccessWww => 3,
        });

        server_item0.add(&access_type_combo);
    }

    vbox.append(&server_item0);

    add_server_items(
        &channel_data,
        server_item_id,
        widget_mode,
        &vbox,
        project_item,
    );

    parent.set_child(Some(&vbox));
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
    widget_mode: WidgetMode,
    vbox: &gtk::Box,
    project_item: &ProjectItem,
) {
    let mut cur_parent = vbox.clone();
    let mut cur_group_name = None::<&str>;
    dbg!(focused_server_item_id);
    let mut started_groups = false;

    for server_item in channel_data.server_items.iter() {
        let group_name = server_item.group_name();

        if !started_groups && group_name.is_some() && widget_mode == WidgetMode::Edit {
            // let (frame, frame_box) = group_frame("", WidgetMode::Edit);
            // finish_server_item_group(&frame_box, WidgetMode::Edit);
            // vbox.append(&frame);

            let add_btn = gtk::MenuButton::builder()
                .icon_name("list-add-symbolic")
                .hexpand(true)
                .popover(&add_server_item_popover(IncludeAddGroup::Yes))
                .build();
            vbox.append(&add_btn);

            started_groups = true;
        }
        if group_name != cur_group_name {
            if let Some(grp) = group_name {
                let (frame, frame_box) = group_frame(grp, widget_mode, project_item);
                cur_parent = frame_box;
                vbox.append(&frame);
                cur_group_name = group_name;
            }
        }
        if server_item.group_name().is_none() {
            if cur_group_name.is_some() {
                finish_server_item_group(&cur_parent, widget_mode);
            }
            cur_parent = vbox.clone();
        }
        match server_item {
            ServerItem::Website(w) => display_server_website(w, widget_mode, &cur_parent),
            ServerItem::PointOfInterest(poi) => display_server_poi(poi, widget_mode, &cur_parent),
            ServerItem::Note(n) => display_server_note(n, widget_mode, &cur_parent),
            ServerItem::ExtraUserAccount(u) => {
                display_server_extra_user_account(u, widget_mode, &cur_parent)
            }
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
    if cur_group_name.is_some() {
        finish_server_item_group(&cur_parent, widget_mode);
    }
}

fn group_frame(
    group_name: &str,
    widget_mode: WidgetMode,
    project_item: &ProjectItem,
) -> (gtk::Frame, gtk::Box) {
    let frame = gtk::Frame::builder().build();
    let frame_box = gtk::Box::builder()
        .css_classes(["card", "frame-group"])
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .build();
    let frame_header = gtk::Box::builder().build();
    if widget_mode == WidgetMode::Show {
        frame_header.append(
            &gtk::Label::builder()
                .css_classes(["heading"])
                .halign(gtk::Align::Start)
                .hexpand(true)
                .label(group_name)
                .build(),
        );
    } else {
        frame_header.append(
            &gtk::Entry::builder()
                .css_classes(["heading"])
                .halign(gtk::Align::Start)
                .hexpand(true)
                .text(group_name)
                .build(),
        );
    }

    let icon_name = match *project_item.edit_mode_items() {
        ProjectItemEditMode::Group(ref g) if g == group_name => "view-reveal-symbolic",
        _ => "document-edit-symbolic",
    };

    let edit_btn = gtk::Button::builder()
        .css_classes(["suggested-action"])
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .icon_name(icon_name)
        .build();

    let gn = group_name.to_string();
    let pi = project_item.clone();
    edit_btn.connect_clicked(move |_| {
        pi.set_edit_mode_items(ProjectItemEditMode::Group(gn.clone()));
    });
    frame_header.append(&edit_btn);

    frame_box.append(&frame_header);
    frame_box.append(&gtk::Separator::builder().build());
    frame.set_child(Some(&frame_box));
    (frame, frame_box)
}

fn finish_server_item_group(cur_parent: &gtk::Box, widget_mode: WidgetMode) {
    if widget_mode == WidgetMode::Edit {
        cur_parent.append(
            &gtk::MenuButton::builder()
                .icon_name("list-add-symbolic")
                .popover(&add_server_item_popover(IncludeAddGroup::No))
                .build(),
        );
    }
}

fn add_group_edit_suffix(server_item1: &adw::PreferencesGroup, edit_closure: glib::RustClosure) {
    let delete_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .css_classes(["destructive-action"])
        .valign(gtk::Align::Center)
        .build();
    let edit_btn = gtk::Button::builder()
        .icon_name("document-edit-symbolic")
        .css_classes(["suggested-action"])
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .build();
    edit_btn.connect_closure("clicked", false, edit_closure);
    let suffix_box = gtk::Box::builder().spacing(15).build();
    suffix_box.append(&delete_btn);
    suffix_box.append(&edit_btn);
    server_item1.set_header_suffix(Some(&suffix_box));
}

fn display_item_edit_dialog(v: &gtk::Box, item_box: gtk::Box) {
    let cbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let header_bar = adw::HeaderBar::builder().build();
    cbox.append(&header_bar);
    cbox.append(
        &adw::Clamp::builder()
            .margin_top(10)
            .child(&item_box)
            .build(),
    );
    let dialog = adw::Dialog::builder()
        .title("Edit Server Website")
        .content_width(600)
        .content_height(400)
        .child(&cbox)
        .build();
    dialog.present(v);
}

fn display_server_website(w: &ServerWebsite, widget_mode: WidgetMode, vbox: &gtk::Box) {
    let server_item1 = server_website_contents(w, WidgetMode::Show, vbox);
    if widget_mode == WidgetMode::Edit {
        add_group_edit_suffix(
            &server_item1,
            glib::closure_local!(@strong w as w1, @strong vbox as v => move |_b: gtk::Button| {
                let item_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .build();
                server_website_contents(&w1, WidgetMode::Edit, &item_box);

                display_item_edit_dialog(&v, item_box);
            }),
        );
    }
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

fn display_server_poi(poi: &ServerPointOfInterest, widget_mode: WidgetMode, vbox: &gtk::Box) {
    let server_item1 = server_poi_contents(poi, WidgetMode::Show, vbox);

    if widget_mode == WidgetMode::Edit {
        add_group_edit_suffix(
            &server_item1,
            glib::closure_local!(@strong poi as p, @strong vbox as v => move |_b: gtk::Button| {
                let item_box = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .build();
                server_poi_contents(&p, WidgetMode::Edit, &item_box);

                display_item_edit_dialog(&v, item_box);
            }),
        );
    }
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

fn display_server_extra_user_account(
    user: &ServerExtraUserAccount,
    widget_mode: WidgetMode,
    vbox: &gtk::Box,
) {
    let server_item1 = adw::PreferencesGroup::builder()
        .description("Extra user")
        // .title(&poi.desc)
        .build();
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

    if widget_mode == WidgetMode::Edit {
        add_group_edit_suffix(
            &server_item1,
            glib::closure_local!(|_b: gtk::Button| {
                println!("editing server www");
            }),
        );
    }
    // TODO auth key

    vbox.append(&server_item1);
}

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

fn display_server_note(note: &ServerNote, widget_mode: WidgetMode, vbox: &gtk::Box) {
    let contents_head = notes::note_markdown_to_quick_preview(&note.contents)
        .lines()
        .take(3)
        .collect_vec()
        .join("⏎");

    // let (note_view, note_view_scrolled_window) =
    //     note::get_note_contents_widget(&note.contents, widget_mode);

    let note_view = note::Note::new();
    // TODO call in the other order, it crashes. could put edit_mode in the ctor, but
    // it feels even worse (would like not to rebuild the widget every time...)
    dbg!(note.id);
    note_view.set_server_note_id(note.id);
    note_view.set_edit_mode(widget_mode.get_edit_mode());

    let server_item1 = adw::PreferencesGroup::builder()
        .description("Note")
        .title(&note.title)
        .build();

    if widget_mode == WidgetMode::Edit {
        add_group_edit_suffix(
            &server_item1,
            glib::closure_local!(|_b: gtk::Button| {
                println!("editing server www");
            }),
        );
    }

    note_view.set_height_request(500);

    let row = adw::ExpanderRow::builder()
        .title(truncate(&contents_head, 120))
        // .css_classes(["property"])
        .build();

    row.add_row(&note_view);

    server_item1.add(&row);

    vbox.append(&server_item1);
}
