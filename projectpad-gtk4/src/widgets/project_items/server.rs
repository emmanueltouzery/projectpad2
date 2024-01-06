use crate::sql_thread::SqlFunc;
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
    group_start_indexes: HashMap<i32, String>,
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
            let mut group_start_indexes = HashMap::new();

            let mut grouped_items = vec![];
            grouped_items.extend(items.iter().filter(|i| i.group_name() == None));
            for group_name in &group_names {
                group_start_indexes.insert(grouped_items.len() as i32, group_name.to_string());
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
                    group_start_indexes,
                    databases_for_websites,
                    websites_for_databases,
                })
                .unwrap();
        }))
        .unwrap();

    let p = parent.clone();
    glib::spawn_future_local(async move {
        let channel_data = receiver.recv().await.unwrap();
        if edit_mode {
            display_server_edit(&p, channel_data);
        } else {
            display_server_show(&p, channel_data);
        }
    });
}

fn display_server_edit(parent: &adw::Bin, channel_data: ChannelData) {
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

    let delete_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .halign(gtk::Align::End)
        .hexpand(true)
        .build();
    header_box.append(&delete_btn);

    vbox.append(&header_box);

    // let server_ar = adw::EntryRow::builder().title("Server name").build();
    // server_ar.add_suffix(
    //     &gtk::Button::builder()
    //         .icon_name("open-menu-symbolic")
    //         .has_frame(false)
    //         .valign(gtk::Align::Center)
    //         .build(),
    // );
    // server.add(&server_ar);

    let server_item0 = adw::PreferencesGroup::builder().build();

    let address_ar = adw::EntryRow::builder()
        .title("Address")
        .text(&channel_data.server.ip)
        .build();
    server_item0.add(&address_ar);
    // server.add(&address_ar);

    let server_username_ar = adw::EntryRow::builder()
        .title("Username")
        .text(&channel_data.server.username)
        .build();
    // server.add(&server_username_ar);
    server_item0.add(&server_username_ar);

    vbox.append(&server_item0);

    add_server_items(&channel_data, WidgetMode::Edit, &vbox);

    // lb.set_property("halign", gtk::Align::Fill);
    // parent.set_property("halign", gtk::Align::Fill);

    let add_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .hexpand(true)
        .build();
    vbox.append(&add_btn);

    parent.set_child(Some(&vbox));
}

fn display_server_show(parent: &adw::Bin, channel_data: ChannelData) {
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

    let server = gtk::Label::builder()
        .label(&channel_data.server.desc)
        .halign(gtk::Align::Start)
        .css_classes(["title-1"])
        // .description("desc")
        .build();
    header_second_col.append(&server);

    header_box.append(&header_second_col);

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

    let address_ar = adw::ActionRow::builder()
        .title("Address")
        .subtitle(&channel_data.server.ip)
        .build();
    address_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("edit-copy-symbolic")
            .build(),
    );
    server_item0.add(&address_ar);
    // server.add(&address_ar);

    let server_username_ar = adw::ActionRow::builder()
        .title("Username")
        .subtitle(&channel_data.server.username)
        .build();
    server_username_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("edit-copy-symbolic")
            .build(),
    );
    // server.add(&server_username_ar);
    server_item0.add(&server_username_ar);

    vbox.append(&server_item0);

    add_server_items(&channel_data, WidgetMode::Show, &vbox);

    // lb.set_property("halign", gtk::Align::Fill);
    // parent.set_property("halign", gtk::Align::Fill);

    parent.set_child(Some(&vbox));
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

    fn build(&self, widget_mode: WidgetMode) -> gtk::Widget {
        match widget_mode {
            WidgetMode::Show => self.build_show(),
            WidgetMode::Edit => self.build_edit(),
        }
    }

    fn build_show(&self) -> gtk::Widget {
        let subtitle = if self.password_mode == PasswordMode::PlainText {
            self.subtitle
        } else {
            "●●●●"
        };
        let e = adw::ActionRow::builder()
            .title(self.title)
            .subtitle(subtitle)
            .build();
        if let Some(i) = self.suffix_icon {
            e.add_suffix(&gtk::Image::builder().icon_name(i).build());
        }
        e.upcast::<gtk::Widget>()
    }

    fn build_edit(&self) -> gtk::Widget {
        match self.password_mode {
            PasswordMode::PlainText => {
                let e = adw::EntryRow::builder()
                    .title(self.title)
                    .text(self.subtitle)
                    .build();
                if let Some(i) = self.suffix_icon {
                    e.add_suffix(&gtk::Image::builder().icon_name(i).build());
                }
                e.upcast::<gtk::Widget>()
            }
            PasswordMode::Password => {
                let e = adw::PasswordEntryRow::builder()
                    .title(self.title)
                    .text(self.subtitle)
                    .build();
                if let Some(i) = self.suffix_icon {
                    e.add_suffix(&gtk::Image::builder().icon_name(i).build());
                }
                e.upcast::<gtk::Widget>()
            }
        }
    }
}

fn add_server_items(channel_data: &ChannelData, widget_mode: WidgetMode, vbox: &gtk::Box) {
    for server_item in channel_data.server_items.iter() {
        match server_item {
            ServerItem::Website(w) => display_server_website(w, widget_mode, &vbox),
            ServerItem::PointOfInterest(poi) => display_server_poi(poi, widget_mode, &vbox),
            _ => {}
        }
    }
}

fn display_server_website(w: &ServerWebsite, widget_mode: WidgetMode, vbox: &gtk::Box) {
    let server_item1 = adw::PreferencesGroup::builder()
        .title("Website")
        .description(&w.desc)
        .build();
    let website_ar =
        DetailsRow::new("Address", &w.url, Some("web-browser-symbolic")).build(widget_mode);
    server_item1.add(&website_ar);

    if !w.username.is_empty() {
        let username_ar =
            DetailsRow::new("Username", &w.username, Some("edit-copy-symbolic")).build(widget_mode);
        server_item1.add(&username_ar);
    }
    if !w.password.is_empty() {
        let password_ar =
            DetailsRow::new_password("Password", &w.password, Some("edit-copy-symbolic"))
                .build(widget_mode);
        server_item1.add(&password_ar);
    }
    vbox.append(&server_item1);
}

fn display_server_poi(poi: &ServerPointOfInterest, widget_mode: WidgetMode, vbox: &gtk::Box) {
    let server_item1 = adw::PreferencesGroup::builder()
        .title("Point of interest")
        .description(&poi.desc)
        .build();
    if !poi.path.is_empty() {
        let path =
            DetailsRow::new("Path", &poi.path, Some("edit-copy-symbolic")).build(widget_mode);
        server_item1.add(&path);
    }
    if !poi.text.is_empty() {
        let field_name = match poi.interest_type {
            InterestType::PoiCommandToRun => "Command",
            _ => "Text",
        };
        let text =
            DetailsRow::new(field_name, &poi.text, Some("edit-copy-symbolic")).build(widget_mode);
        server_item1.add(&text);
    }

    vbox.append(&server_item1);
}
