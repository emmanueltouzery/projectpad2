use super::server_item_list_item::Msg as ServerItemListItemMsg;
use super::server_item_list_item::ServerItemListItem;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use itertools::Itertools;
use projectpadsql::models::{
    Server, ServerDatabase, ServerExtraUserAccount, ServerLink, ServerNote, ServerPointOfInterest,
    ServerWebsite,
};
use relm::{Component, ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::collections::{BTreeSet, HashMap};
use std::sync::mpsc;

pub struct ChannelData {
    server_items: Vec<ServerItem>,
    group_start_indexes: HashMap<i32, String>,
    databases_for_websites: HashMap<i32, ServerDatabase>,
    websites_for_databases: HashMap<i32, Vec<ServerWebsite>>,
}

#[derive(Msg)]
pub enum Msg {
    ServerSelected(Option<Server>),
    ServerLinkSelected(ServerLink),
    GotItems(ChannelData),
    ViewNote(ServerNote),
    RefreshItems,
    RequestDisplayServerItem(ServerItem),
    ShowInfoBar(String),
    ScrollTo(ScrollTarget),
    OpenSingleWebsiteLink,
}

#[derive(Clone, Debug)]
pub enum ServerItem {
    Website(ServerWebsite),
    PointOfInterest(ServerPointOfInterest),
    Note(ServerNote),
    ExtraUserAccount(ServerExtraUserAccount),
    Database(ServerDatabase),
}

impl ServerItem {
    fn group_name(&self) -> Option<&str> {
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

#[derive(Clone)]
pub enum ScrollTarget {
    ServerItem(ServerItem),
    GroupName(String),
}

pub struct Model {
    relm: relm::Relm<ServerPoiContents>,
    db_sender: mpsc::Sender<SqlFunc>,
    sender: relm::Sender<ChannelData>,
    _channel: relm::Channel<ChannelData>,
    cur_server_id: Option<i32>,
    server_items: Vec<ServerItem>,
    server_item_groups_start_indexes: HashMap<i32, String>,
    databases_for_websites: HashMap<i32, ServerDatabase>,
    websites_for_databases: HashMap<i32, Vec<ServerWebsite>>,
    _children_components: Vec<Component<ServerItemListItem>>,
    scroll_to_item_request: Option<ScrollTarget>,
}

#[widget]
impl Widget for ServerPoiContents {
    fn init_view(&mut self) {
        self.contents_list
            .get_style_context()
            .add_class("item_list");
        self.contents_scroll
            .get_style_context()
            .add_class("scrollgradient");
        self.update_contents_list();
        self.contents_list
            .set_focus_vadjustment(&self.contents_scroll.get_vadjustment().unwrap());
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |items: ChannelData| {
            stream.emit(Msg::GotItems(items));
        });
        Model {
            relm: relm.clone(),
            _channel: channel,
            sender,
            db_sender,
            cur_server_id: None,
            server_items: vec![],
            server_item_groups_start_indexes: HashMap::new(),
            databases_for_websites: HashMap::new(),
            websites_for_databases: HashMap::new(),
            _children_components: vec![],
            scroll_to_item_request: None,
        }
    }

    fn get_find_serveritem_cb(si: &ServerItem) -> Box<dyn Fn(&ServerItem) -> bool> {
        match si {
            ServerItem::Database(db) => {
                let id = db.id;
                Box::new(move |item2| {
                    matches!(item2,
                        ServerItem::Database(db2) if db2.id == id
                    )
                })
            }
            ServerItem::ExtraUserAccount(us) => {
                let id = us.id;
                Box::new(move |item2| {
                    matches!(item2,
                        ServerItem::ExtraUserAccount(us2) if us2.id == id
                    )
                })
            }
            ServerItem::Note(n) => {
                let id = n.id;
                Box::new(move |item2| {
                    matches!(item2,
                        ServerItem::Note(n2) if n2.id == id
                    )
                })
            }
            ServerItem::PointOfInterest(p) => {
                let id = p.id;
                Box::new(move |item2| {
                    matches!(item2,
                        ServerItem::PointOfInterest(p2) if p2.id == id
                    )
                })
            }
            ServerItem::Website(w) => {
                let id = w.id;
                Box::new(move |item2| {
                    matches!(item2,
                        ServerItem::Website(w2) if w2.id == id
                    )
                })
            }
        }
    }

    fn scroll_to(&mut self, st: ScrollTarget) {
        match st {
            ScrollTarget::ServerItem(si) => self.scroll_to_server_item(si),
            ScrollTarget::GroupName(gn) => {
                let mut found = false;
                if let Some((idx, _)) = self
                    .model
                    .server_item_groups_start_indexes
                    .iter()
                    .find(|(_cur_idx, cur_grp)| **cur_grp == gn)
                {
                    if let Some(row) = self.contents_list.get_row_at_index(*idx as i32) {
                        if let Some(vscroll) = self.contents_scroll.get_vadjustment() {
                            if let Some((_x, y)) =
                                row.translate_coordinates(&self.contents_list, 0, 0)
                            {
                                vscroll.set_value(y.into());
                            }
                        }
                        // self.contents_list.select_row(Some(&row));
                        // row.grab_focus();
                        found = true;
                    }
                }
                if !found {
                    // didn't find the item, maybe we didn't load that list
                    // yet, save the request and we'll check next time we'll load
                    self.model.scroll_to_item_request = Some(ScrollTarget::GroupName(gn));
                }
            }
        }
    }

    fn scroll_to_server_item(&mut self, si: ServerItem) {
        let find_cb = Self::get_find_serveritem_cb(&si);
        let midx = self.model.server_items.iter().position(find_cb);
        if let Some(idx) = midx {
            if let Some(row) = self.contents_list.get_row_at_index(idx as i32) {
                if let Some(vscroll) = self.contents_scroll.get_vadjustment() {
                    if let Some((_x, y)) = row.translate_coordinates(&self.contents_list, 0, 0) {
                        vscroll.set_value(y.into());
                    }
                }
                // self.contents_list.select_row(Some(&row));
                // row.grab_focus();
            }
        } else {
            // didn't find the item, maybe we didn't load that list
            // yet, save the request and we'll check next time we'll load
            self.model.scroll_to_item_request = Some(ScrollTarget::ServerItem(si));
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ServerSelected(srv) => {
                self.model.cur_server_id = srv.map(|s| s.id);
                self.fetch_items();
            }
            Msg::ServerLinkSelected(srv_l) => {
                self.model.cur_server_id = Some(srv_l.linked_server_id);
                self.model.scroll_to_item_request = srv_l
                    .linked_group_name
                    .map(|gn| ScrollTarget::GroupName(gn));
                self.fetch_items();
            }
            Msg::GotItems(items) => {
                self.model.server_items = items.server_items;
                self.model.server_item_groups_start_indexes = items.group_start_indexes;
                self.model.databases_for_websites = items.databases_for_websites;
                self.model.websites_for_databases = items.websites_for_databases;
                self.update_contents_list();
                // do we have a pending request to scroll to a certain item?
                if let Some(st) = self.model.scroll_to_item_request.take() {
                    let relm = self.model.relm.clone();
                    // must request the scroll through a gtk idle callback,
                    // because the list was just populated and row items are
                    // not ready to be interacted with
                    glib::idle_add_local(move || {
                        relm.stream().emit(Msg::ScrollTo(st.clone()));
                        glib::Continue(false)
                    });
                }
            }
            Msg::RequestDisplayServerItem(_) => {}
            Msg::RefreshItems => {
                self.fetch_items();
            }
            Msg::ScrollTo(st) => {
                self.scroll_to(st);
            }
            Msg::OpenSingleWebsiteLink => {
                let websites_with_urls: Vec<_> = self
                    .model
                    .server_items
                    .iter()
                    .filter_map(|si| match si {
                        ServerItem::Website(w) if !w.url.is_empty() => Some(w),
                        _ => None,
                    })
                    .take(2)
                    .collect();
                if websites_with_urls.len() == 1 {
                    if let Result::Err(e) = gtk::show_uri_on_window(
                        None::<&gtk::Window>,
                        &websites_with_urls.get(0).unwrap().url,
                        0,
                    ) {
                        eprintln!("Error opening link: {}", e);
                    }
                }
            }
            // ViewNote is meant for my parent
            Msg::ViewNote(_) => {}
            // meant for my parent
            Msg::ShowInfoBar(_) => {}
        }
    }

    fn database_for_item(&self, server_item: &ServerItem) -> Option<ServerDatabase> {
        match server_item {
            ServerItem::Website(srv_w) => srv_w
                .server_database_id
                .and_then(|db_id| self.model.databases_for_websites.get(&db_id))
                .cloned(),
            _ => None,
        }
    }

    fn websites_for_item(&self, server_item: &ServerItem) -> Vec<ServerWebsite> {
        match server_item {
            ServerItem::Database(db) => self
                .model
                .websites_for_databases
                .get(&db.id)
                .cloned()
                .unwrap_or_else(Vec::new),
            _ => vec![],
        }
    }

    fn update_contents_list(&mut self) {
        for child in self.contents_list.get_children() {
            self.contents_list.remove(&child);
        }
        let mut children_components = vec![];
        for item in &self.model.server_items {
            let component = self.contents_list.add_widget::<ServerItemListItem>((
                self.model.db_sender.clone(),
                item.clone(),
                self.database_for_item(&item),
                self.websites_for_item(&item),
            ));
            relm::connect!(
                component@ServerItemListItemMsg::ViewNote(ref n),
                self.model.relm, Msg::ViewNote(n.clone()));
            relm::connect!(
                component@ServerItemListItemMsg::ServerItemDeleted(_),
                self.model.relm, Msg::RefreshItems);
            relm::connect!(
                component@ServerItemListItemMsg::RequestDisplayServerItem(ref server_item),
                           self.model.relm, Msg::RequestDisplayServerItem(server_item.clone()));
            relm::connect!(
                component@ServerItemListItemMsg::ShowInfoBar(ref msg),
                           self.model.relm, Msg::ShowInfoBar(msg.clone()));
            children_components.push(component);
        }
        let indexes = self.model.server_item_groups_start_indexes.clone();
        self.contents_list
            .set_header_func(Some(Box::new(move |row, _h| {
                if let Some(group_name) = indexes.get(&row.get_index()) {
                    let label = gtk::LabelBuilder::new()
                        .label(group_name)
                        .xalign(0.0)
                        .build();
                    label.get_style_context().add_class("server_item_header");
                    row.set_header(Some(&label));
                } else {
                    row.set_header::<gtk::ListBoxRow>(None)
                }
            })));
        // need to keep the component alive else the event handling dies
        self.model._children_components = children_components;

        if self.model.cur_server_id.is_some() && self.model.server_items.is_empty() {
            // show a little intro text
            let vbox = gtk::BoxBuilder::new()
                .orientation(gtk::Orientation::Vertical)
                .build();
            let intro_label = gtk::LabelBuilder::new()
                .label("<big>This server doesn't have any items</big>")
                .xalign(0.0)
                .margin(10)
                .use_markup(true)
                .build();
            vbox.add(&intro_label);
            let details = gtk::ExpanderBuilder::new()
                .label("Server item types")
                .build();
            details.add(
                &gtk::LabelBuilder::new()
                    .label("You can add multiple item types to a server. To do that, use the gear icon \
                            next to the server name. Let's review all the available item types:\n\n\
                            • <u>Point of interest</u> - a command to run or a relevant file or folder \
                            located on that server;\n\n\
                            • <u>Website</u> - a service that's reachable over the network that lives \
                            on that server. Doesn't have to be specifically a website, but often is. \
                            A rabbitmq server, for instance, could possibly fit that description too. \
                            A 'website' has a an address, a port, possibly a username and password, and can be \
                            tied to a database;\n\n\
                            • <u>Database</u> - a database that lives on that server. It can have \
                            a default username/password tied to it, and websites can be tied to it;\n\n\
                            • <u>Extra user</u> - a pair of username and password, or username and \
                            authentication key, somehow tied to this server. It could be a user \
                            allowing to access the server itself, or it could be tied to a website \
                            on this server, or a database on that server... You should make it clear \
                            through the 'text' field and possibly by grouping together related \
                            server items through the 'group' field. We say 'extra' user because the \
                            basic user is the one that is directly tied to the server itself;\n\n\
                            • <u>Server note</u> - Notes are markdown-formatted text containing \
                            free-form text. Server notes are tied to a specific server. You can \
                            tie them to a more specific category within the server by using the \
                            'group' field.")
                    .xalign(0.0)
                    .margin(10)
                    .use_markup(true)
                    .wrap(true)
                    .build(),
            );
            vbox.add(&details);
            vbox.show_all();
            self.contents_list.add(&vbox);
        }

        for child in self.contents_list.get_children() {
            // don't want the row background color to change when we hover
            // it with the mouse (activatable), or the focus dotted lines
            // around the rows to be drawn, for aesthetic reasons.
            let row = child.dynamic_cast::<gtk::ListBoxRow>().unwrap();
            row.set_activatable(false);
            row.set_can_focus(false);
        }
    }

    fn fetch_items(&mut self) {
        let s = self.model.sender.clone();
        let cur_server_id = self.model.cur_server_id;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_database::dsl as srv_db;
                use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
                use projectpadsql::schema::server_note::dsl as srv_note;
                use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
                use projectpadsql::schema::server_website::dsl as srv_www;
                let (items, databases_for_websites, websites_for_databases) = match cur_server_id {
                    Some(sid) => {
                        let server_websites = srv_www::server_website
                            .filter(srv_www::server_id.eq(sid))
                            .order(srv_www::desc.asc())
                            .load::<ServerWebsite>(sql_conn)
                            .unwrap();

                        let databases_for_websites = srv_db::server_database
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
                                srv_www::server_database_id
                                    .eq_any(databases.iter().map(|db| db.id)),
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

                let group_names: BTreeSet<&str> =
                    items.iter().filter_map(|i| i.group_name()).collect();
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

                s.send(ChannelData {
                    server_items: grouped_items.into_iter().cloned().collect(),
                    group_start_indexes,
                    databases_for_websites,
                    websites_for_databases,
                })
                .unwrap();
            }))
            .unwrap();
    }

    view! {
        #[name="contents_scroll"]
        gtk::ScrolledWindow {
            #[name="contents_list"]
            gtk::ListBox {
                selection_mode: gtk::SelectionMode::None,
            }
        }
    }
}
