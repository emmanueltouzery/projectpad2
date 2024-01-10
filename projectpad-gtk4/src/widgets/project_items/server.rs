use crate::{notes, sql_thread::SqlFunc};
use adw::prelude::*;
use diesel::prelude::*;
use itertools::Itertools;
use projectpadsql::models::{
    InterestType, Server, ServerDatabase, ServerExtraUserAccount, ServerNote,
    ServerPointOfInterest, ServerWebsite,
};
use std::{
    collections::{BTreeSet, HashMap},
    sync::mpsc,
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
    edit_mode: bool, // TODO bools are crappy as parameters
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
        display_server(&p, channel_data, edit_mode);
    });
}

fn display_server(parent: &adw::Bin, channel_data: ChannelData, edit_mode: bool) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(10)
        .margin_top(10)
        .build();

    let header_box = gtk::Box::builder().spacing(10).build();

    let server_icon = gtk::Image::builder()
        .icon_name("server")
        .pixel_size(48)
        .build();
    header_box.append(&server_icon);

    let header_second_col = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .valign(gtk::Align::Center)
        .build();

    let server = gtk::Entry::builder()
        .text(&channel_data.server.desc)
        .halign(gtk::Align::Start)
        .css_classes(["title-1"])
        // .description("desc")
        .build();
    header_second_col.append(&server);

    header_box.append(&header_second_col);

    if edit_mode {
        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .halign(gtk::Align::End)
            .hexpand(true)
            .build();
        header_box.append(&delete_btn);
    }

    vbox.append(&header_box);

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

    let widget_mode = if edit_mode {
        WidgetMode::Edit
    } else {
        WidgetMode::Show
    };

    DetailsRow::new(
        "Address",
        &channel_data.server.ip,
        Some("edit-copy-symbolic"),
    )
    .add(widget_mode, &server_item0);

    DetailsRow::new(
        "Username",
        &channel_data.server.username,
        Some("edit-copy-symbolic"),
    )
    .add(widget_mode, &server_item0);

    DetailsRow::new_password(
        "Password",
        &channel_data.server.password,
        Some("edit-copy-symbolic"),
    )
    .add(widget_mode, &server_item0);
    DetailsRow::new(
        "Text",
        &channel_data.server.text,
        Some("edit-copy-symbolic"),
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
    if include_add_group == IncludeAddGroup::Yes {
        add_menu.append(Some("Group"), None);
    }
    gtk::PopoverMenu::builder().menu_model(&add_menu).build()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WidgetMode {
    Show,
    Edit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PasswordMode {
    PlainText,
    Password,
}

struct DetailsRow<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub suffix_icon: Option<&'static str>,
    pub password_mode: PasswordMode,
}

impl DetailsRow<'_> {
    fn new<'a>(
        title: &'a str,
        subtitle: &'a str,
        suffix_icon: Option<&'static str>,
    ) -> DetailsRow<'a> {
        DetailsRow {
            title,
            subtitle,
            suffix_icon,
            password_mode: PasswordMode::PlainText,
        }
    }

    fn new_password<'a>(
        title: &'a str,
        subtitle: &'a str,
        suffix_icon: Option<&'static str>,
    ) -> DetailsRow<'a> {
        DetailsRow {
            title,
            subtitle,
            suffix_icon,
            password_mode: PasswordMode::Password,
        }
    }

    fn add(&self, widget_mode: WidgetMode, group: &adw::PreferencesGroup) {
        match widget_mode {
            WidgetMode::Show => self.add_show(group),
            WidgetMode::Edit => self.add_edit(group),
        }
    }

    fn add_show(&self, group: &adw::PreferencesGroup) {
        if !self.subtitle.is_empty() {
            let subtitle = if self.password_mode == PasswordMode::PlainText {
                self.subtitle
            } else {
                "●●●●"
            };
            let e = adw::ActionRow::builder()
                .title(glib::markup_escape_text(self.title))
                .subtitle(glib::markup_escape_text(subtitle))
                // https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/boxed-lists.html#property-rows
                // When used together with the .property style class, AdwActionRow and
                // AdwExpanderRow deemphasize their title and emphasize their subtitle instead
                .css_classes(["property"])
                .build();
            if let Some(i) = self.suffix_icon {
                e.add_suffix(&gtk::Image::builder().icon_name(i).build());
            }
            group.add(&e);
        }
    }

    fn add_edit(&self, group: &adw::PreferencesGroup) {
        match self.password_mode {
            PasswordMode::PlainText => {
                let e = adw::EntryRow::builder()
                    .title(self.title)
                    .text(self.subtitle)
                    .build();
                if let Some(i) = self.suffix_icon {
                    e.add_suffix(&gtk::Image::builder().icon_name(i).build());
                }
                group.add(&e);
            }
            PasswordMode::Password => {
                let e = adw::PasswordEntryRow::builder()
                    .title(self.title)
                    .text(self.subtitle)
                    .build();
                if let Some(i) = self.suffix_icon {
                    e.add_suffix(&gtk::Image::builder().icon_name(i).build());
                }
                group.add(&e);
            }
        }
    }
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

fn display_server_website(w: &ServerWebsite, widget_mode: WidgetMode, vbox: &gtk::Box) {
    let server_item1 = adw::PreferencesGroup::builder()
        .description("Website")
        .title(&w.desc)
        .build();
    DetailsRow::new("Address", &w.url, Some("web-browser-symbolic"))
        .add(widget_mode, &server_item1);

    DetailsRow::new("Username", &w.username, Some("edit-copy-symbolic"))
        .add(widget_mode, &server_item1);
    DetailsRow::new_password("Password", &w.password, Some("edit-copy-symbolic"))
        .add(widget_mode, &server_item1);
    DetailsRow::new("Text", &w.text, Some("edit-copy-symbolic")).add(widget_mode, &server_item1);

    if widget_mode == WidgetMode::Edit {
        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .build();
        server_item1.set_header_suffix(Some(&delete_btn));
    }

    vbox.append(&server_item1);
}

fn display_server_poi(poi: &ServerPointOfInterest, widget_mode: WidgetMode, vbox: &gtk::Box) {
    let server_item1 = adw::PreferencesGroup::builder()
        .description("Point of interest")
        .title(&poi.desc)
        .build();
    DetailsRow::new("Path", &poi.path, Some("edit-copy-symbolic")).add(widget_mode, &server_item1);
    let field_name = match poi.interest_type {
        InterestType::PoiCommandToRun | InterestType::PoiCommandTerminal => "Command",
        _ => "Text",
    };
    DetailsRow::new(field_name, &poi.text, Some("edit-copy-symbolic"))
        .add(widget_mode, &server_item1);

    if widget_mode == WidgetMode::Edit {
        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .build();
        server_item1.set_header_suffix(Some(&delete_btn));
    }

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

    let text_view = if widget_mode == WidgetMode::Show {
        gtk::TextView::builder()
            .buffer(
                &notes::note_markdown_to_text_buffer(
                    &note.contents,
                    &crate::notes::build_tag_table(),
                )
                .buffer,
            )
            .editable(false)
            .build()
            .upcast::<gtk::Widget>()
    } else {
        let buf = sourceview5::Buffer::with_language(
            &sourceview5::LanguageManager::default()
                .language("markdown")
                .unwrap(),
        );
        // https://stackoverflow.com/a/63351603/516188
        // TODO don't hardcode sourceview to dark mode
        // dbg!(&sourceview5::StyleSchemeManager::default().scheme_ids());
        buf.set_property(
            "style-scheme",
            sourceview5::StyleSchemeManager::default().scheme("Adwaita-dark"),
        );
        buf.set_text(&note.contents);
        let view = sourceview5::View::with_buffer(&buf);
        view.upcast::<gtk::Widget>()
    };

    let server_item1 = adw::PreferencesGroup::builder()
        .description("Note")
        .title(&note.title)
        .build();

    if widget_mode == WidgetMode::Edit {
        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .build();
        server_item1.set_header_suffix(Some(&delete_btn));
    }

    let scrolled_text_view = gtk::ScrolledWindow::builder()
        .child(&text_view)
        .height_request(500)
        .build();

    let row = adw::ExpanderRow::builder()
        .title(truncate(&contents_head, 120))
        // .css_classes(["property"])
        .build();

    row.add_row(&scrolled_text_view);

    server_item1.add(&row);

    vbox.append(&server_item1);
}
