use crate::{notes, sql_thread::SqlFunc, widgets::project_item::WidgetMode};
use adw::prelude::*;
use diesel::prelude::*;
use itertools::Itertools;
use projectpadsql::models::{
    InterestType, Server, ServerDatabase, ServerExtraUserAccount, ServerNote,
    ServerPointOfInterest, ServerWebsite,
};
use std::{
    collections::{BTreeSet, HashMap},
    rc::Rc,
    sync::mpsc,
};

use super::{
    common::{self, copy_to_clipboard, DetailsRow, SuffixAction},
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
    server_id: Option<i32>,
    widget_mode: WidgetMode,
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
            let sid = server_id.unwrap();
            let (server, items, databases_for_websites, websites_for_databases) = {
                let server = srv::server
                    .filter(srv::id.eq(sid))
                    .first::<Server>(sql_conn)
                    .unwrap();

                let server_websites = srv_www::server_website
                    .filter(srv_www::server_id.eq(sid))
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
                        .filter(srv_poi::server_id.eq(sid))
                        .order(srv_poi::desc.asc())
                        .load::<ServerPointOfInterest>(sql_conn)
                        .unwrap()
                        .into_iter()
                        .map(ServerItem::PointOfInterest),
                );
                servers.extend(
                    srv_note::server_note
                        .filter(srv_note::server_id.eq(sid))
                        .order(srv_note::title.asc())
                        .load::<ServerNote>(sql_conn)
                        .unwrap()
                        .into_iter()
                        .map(ServerItem::Note),
                );
                servers.extend(
                    &mut srv_usr::server_extra_user_account
                        .filter(srv_usr::server_id.eq(sid))
                        .order(srv_usr::desc.asc())
                        .load::<ServerExtraUserAccount>(sql_conn)
                        .unwrap()
                        .into_iter()
                        .map(ServerItem::ExtraUserAccount),
                );

                let databases = srv_db::server_database
                    .filter(srv_db::server_id.eq(sid))
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
    glib::spawn_future_local(async move {
        let channel_data = receiver.recv().await.unwrap();
        display_server(&p, channel_data, widget_mode);
    });
}

fn display_server(parent: &adw::Bin, channel_data: ChannelData, widget_mode: WidgetMode) {
    let vbox = common::get_contents_box_with_header(&channel_data.server.desc, widget_mode);

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

    DetailsRow::new(
        "Address",
        &channel_data.server.ip,
        SuffixAction::copy(&channel_data.server.ip),
    )
    .add(widget_mode, &server_item0);

    DetailsRow::new(
        "Username",
        &channel_data.server.username,
        SuffixAction::copy(&channel_data.server.username),
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
    )
    .add(widget_mode, &server_item0);

    vbox.append(&server_item0);

    add_server_items(&channel_data, widget_mode, &vbox);

    if widget_mode == WidgetMode::Edit {
        // let (frame, frame_box) = group_frame("", WidgetMode::Edit);
        // finish_server_item_group(&frame_box, WidgetMode::Edit);
        // vbox.append(&frame);

        let add_btn = gtk::MenuButton::builder()
            .icon_name("list-add-symbolic")
            .hexpand(true)
            .popover(&add_server_item_popover(IncludeAddGroup::Yes))
            .build();
        vbox.append(&add_btn);
    }
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

fn add_server_items(channel_data: &ChannelData, widget_mode: WidgetMode, vbox: &gtk::Box) {
    let mut cur_parent = vbox.clone();
    let mut cur_group_name = None::<&str>;

    for server_item in channel_data.server_items.iter() {
        let group_name = server_item.group_name();
        if group_name != cur_group_name {
            if let Some(grp) = group_name {
                let (frame, frame_box) = group_frame(grp, widget_mode);
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
    }
    if cur_group_name.is_some() {
        finish_server_item_group(&cur_parent, widget_mode);
    }
}

fn group_frame(group_name: &str, widget_mode: WidgetMode) -> (gtk::Frame, gtk::Box) {
    let frame = gtk::Frame::builder().build();
    let frame_box = gtk::Box::builder()
        .css_classes(["card", "frame-group"])
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .build();
    if widget_mode == WidgetMode::Show {
        frame_box.append(
            &gtk::Label::builder()
                .css_classes(["heading"])
                .halign(gtk::Align::Start)
                .label(group_name)
                .build(),
        );
    } else {
        frame_box.append(
            &gtk::Entry::builder()
                .css_classes(["heading"])
                .halign(gtk::Align::Start)
                .text(group_name)
                .build(),
        );
    }
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

fn add_group_edit_suffix(server_item1: &adw::PreferencesGroup, title: &str) {
    let delete_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .build();
    let edit_desc_entry = gtk::Entry::builder()
        .text(title)
        .valign(gtk::Align::Center)
        .build();

    let suffix_box = gtk::Box::builder().spacing(15).build();
    suffix_box.append(&edit_desc_entry);
    suffix_box.append(&delete_btn);
    server_item1.set_header_suffix(Some(&suffix_box));
}

fn display_server_website(w: &ServerWebsite, widget_mode: WidgetMode, vbox: &gtk::Box) {
    let server_item1 = adw::PreferencesGroup::builder()
        .description("Website")
        .title(&w.desc)
        .build();
    let url = w.url.clone();
    DetailsRow::new(
        "Address",
        &w.url,
        Some(SuffixAction {
            icon: "external-link-alt-symbolic",
            action: Rc::new(Box::new(move || {
                gtk::UriLauncher::new(&url).launch(
                    None::<&gtk::Window>,
                    None::<&gio::Cancellable>,
                    |_| {},
                );
            })),
        }),
    )
    .add(widget_mode, &server_item1);

    DetailsRow::new("Username", &w.username, SuffixAction::copy(&w.username))
        .add(widget_mode, &server_item1);
    DetailsRow::new_password("Password", &w.password, SuffixAction::copy(&w.password))
        .add(widget_mode, &server_item1);
    DetailsRow::new("Text", &w.text, SuffixAction::copy(&w.text)).add(widget_mode, &server_item1);

    if widget_mode == WidgetMode::Edit {
        add_group_edit_suffix(&server_item1, &w.desc);
    }

    vbox.append(&server_item1);
}

fn display_server_poi(poi: &ServerPointOfInterest, widget_mode: WidgetMode, vbox: &gtk::Box) {
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
    DetailsRow::new("Path", &poi.path, SuffixAction::copy(&poi.path))
        .add(widget_mode, &server_item1);
    let field_name = match poi.interest_type {
        InterestType::PoiCommandToRun | InterestType::PoiCommandTerminal => "Command",
        _ => "Text",
    };
    DetailsRow::new(field_name, &poi.text, SuffixAction::copy(&poi.text))
        .add(widget_mode, &server_item1);

    if widget_mode == WidgetMode::Edit {
        add_group_edit_suffix(&server_item1, &poi.desc);
    }

    vbox.append(&server_item1);
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
    )
    .add(widget_mode, &server_item1);
    DetailsRow::new_password(
        "Password",
        &user.password,
        SuffixAction::copy(&user.password),
    )
    .add(widget_mode, &server_item1);

    if widget_mode == WidgetMode::Edit {
        add_group_edit_suffix(&server_item1, &user.username);
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
        add_group_edit_suffix(&server_item1, &note.title);
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
