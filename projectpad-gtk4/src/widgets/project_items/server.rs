use crate::sql_thread::SqlFunc;
use adw::prelude::*;
use diesel::prelude::*;
use itertools::Itertools;
use projectpadsql::models::{
    ServerDatabase, ServerExtraUserAccount, ServerNote, ServerPointOfInterest, ServerWebsite,
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
            use projectpadsql::schema::server_database::dsl as srv_db;
            use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
            use projectpadsql::schema::server_note::dsl as srv_note;
            use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
            use projectpadsql::schema::server_website::dsl as srv_www;
            let (items, databases_for_websites, websites_for_databases) = match server_id {
                Some(sid) => {
                    let server_websites = srv_www::server_website
                        .filter(srv_www::server_id.eq(sid))
                        .order(srv_www::desc.asc())
                        .load::<ServerWebsite>(sql_conn)
                        .unwrap();

                    let databases_for_websites =
                        srv_db::server_database
                            .filter(srv_db::id.eq_any(
                                server_websites.iter().filter_map(|w| w.server_database_id),
                            ))
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
                        .filter(
                            srv_www::server_database_id.eq_any(databases.iter().map(|db| db.id)),
                        )
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

                    (servers, databases_for_websites, websites_for_databases)
                }
                None => (vec![], HashMap::new(), HashMap::new()),
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
                    server_items: grouped_items.into_iter().cloned().collect(),
                    group_start_indexes,
                    databases_for_websites,
                    websites_for_databases,
                })
                .unwrap();
        }))
        .unwrap();

    glib::spawn_future_local(async move {
        let channel_data = receiver.recv().await.unwrap();
        dbg!(&channel_data);
    });

    display_server(parent, server_id.unwrap(), edit_mode);
}

fn display_server(parent: &adw::Bin, id: i32, edit_mode: bool) {
    if edit_mode {
        display_server_edit(parent);
    } else {
        display_server_show(parent);
    }
}

fn display_server_edit(parent: &adw::Bin) {
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
        .text("Server")
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
        .text("hostname")
        .build();
    server_item0.add(&address_ar);
    // server.add(&address_ar);

    let server_username_ar = adw::EntryRow::builder()
        .title("Username")
        .text("root")
        .build();
    // server.add(&server_username_ar);
    server_item0.add(&server_username_ar);

    vbox.append(&server_item0);

    let server_item1 = adw::PreferencesGroup::builder()
        .title("Website")
        .description("service1")
        .build();
    let website_ar = adw::EntryRow::builder()
        .title("Address")
        .text("https://service1.com")
        .build();
    server_item1.add(&website_ar);

    let username_ar = adw::EntryRow::builder()
        .title("Username")
        .text("admin")
        .build();
    server_item1.add(&username_ar);
    let password_ar = adw::PasswordEntryRow::builder()
        .title("Password")
        .text("pass")
        .build();
    server_item1.add(&password_ar);
    vbox.append(&server_item1);

    // lb.set_property("halign", gtk::Align::Fill);
    // parent.set_property("halign", gtk::Align::Fill);

    let add_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .hexpand(true)
        .build();
    vbox.append(&add_btn);

    parent.set_child(Some(&vbox));
}

fn display_server_show(parent: &adw::Bin) {
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
        .label("Server")
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
        .subtitle("hostname")
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
        .subtitle("root")
        .build();
    server_username_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("edit-copy-symbolic")
            .build(),
    );
    // server.add(&server_username_ar);
    server_item0.add(&server_username_ar);

    vbox.append(&server_item0);

    let server_item1 = adw::PreferencesGroup::builder()
        .title("Website")
        .description("service1")
        .build();
    let website_ar = adw::ActionRow::builder()
        .title("Address")
        .subtitle("https://service1.com")
        .build();
    website_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("web-browser-symbolic")
            .build(),
    );
    server_item1.add(&website_ar);

    let username_ar = adw::ActionRow::builder()
        .title("Username")
        .subtitle("admin")
        .build();
    username_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("edit-copy-symbolic")
            .build(),
    );
    server_item1.add(&username_ar);
    let password_ar = adw::ActionRow::builder()
        .title("Password")
        .subtitle("●●●●")
        .build();
    password_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("edit-copy-symbolic")
            .build(),
    );
    server_item1.add(&password_ar);
    vbox.append(&server_item1);

    // lb.set_property("halign", gtk::Align::Fill);
    // parent.set_property("halign", gtk::Align::Fill);

    parent.set_child(Some(&vbox));
}
