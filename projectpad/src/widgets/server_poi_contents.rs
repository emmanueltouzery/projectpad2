use super::server_item_list_item::Msg as ServerItemListItemMsg;
use super::server_item_list_item::ServerItemListItem;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{
    Server, ServerDatabase, ServerExtraUserAccount, ServerNote, ServerPointOfInterest,
    ServerWebsite,
};
use relm::{Component, ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::collections::{BTreeSet, HashMap};
use std::sync::mpsc;

type ChannelData = (Vec<ServerItem>, HashMap<i32, String>);

#[derive(Msg)]
pub enum Msg {
    ServerSelected(Option<Server>),
    GotItems(ChannelData),
    ViewNote(ServerNote),
    RefreshItems,
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
}

pub struct Model {
    relm: relm::Relm<ServerPoiContents>,
    db_sender: mpsc::Sender<SqlFunc>,
    sender: relm::Sender<ChannelData>,
    _channel: relm::Channel<ChannelData>,
    cur_project_item: Option<Server>,
    server_items: Vec<ServerItem>,
    server_item_groups_start_indexes: HashMap<i32, String>,
    _children_components: Vec<Component<ServerItemListItem>>,
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
            cur_project_item: None,
            server_items: vec![],
            server_item_groups_start_indexes: HashMap::new(),
            _children_components: vec![],
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ServerSelected(srv) => {
                self.model.cur_project_item = srv;
                self.fetch_items();
            }
            Msg::GotItems(items) => {
                self.model.server_items = items.0;
                self.model.server_item_groups_start_indexes = items.1;
                self.update_contents_list();
            }
            // ViewNote is meant for my parent
            Msg::ViewNote(_) => {}
            Msg::RefreshItems => {
                self.fetch_items();
            }
        }
    }

    fn update_contents_list(&mut self) {
        for child in self.contents_list.get_children() {
            self.contents_list.remove(&child);
        }
        let mut children_components = vec![];
        for item in &self.model.server_items {
            let component = self
                .contents_list
                .add_widget::<ServerItemListItem>((self.model.db_sender.clone(), item.clone()));
            relm::connect!(component@ServerItemListItemMsg::ViewNote(ref n), self.model.relm, Msg::ViewNote(n.clone()));
            relm::connect!(component@ServerItemListItemMsg::ServerPoiDeleted(_), self.model.relm, Msg::RefreshItems);
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
    }

    fn fetch_items(&mut self) {
        let s = self.model.sender.clone();
        let cur_server_id = self.model.cur_project_item.as_ref().map(|srv| srv.id);
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_database::dsl as srv_db;
                use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
                use projectpadsql::schema::server_note::dsl as srv_note;
                use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
                use projectpadsql::schema::server_website::dsl as srv_www;
                let items = match cur_server_id {
                    Some(sid) => {
                        let mut servers = srv_www::server_website
                            .filter(srv_www::server_id.eq(sid))
                            .order(srv_www::desc.asc())
                            .load::<ServerWebsite>(sql_conn)
                            .unwrap()
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
                        servers.extend(
                            &mut srv_db::server_database
                                .filter(srv_db::server_id.eq(sid))
                                .order(srv_db::desc.asc())
                                .load::<ServerDatabase>(sql_conn)
                                .unwrap()
                                .into_iter()
                                .map(ServerItem::Database),
                        );

                        servers
                    }
                    None => vec![],
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

                s.send((
                    grouped_items.into_iter().map(|i| i.clone()).collect(),
                    group_start_indexes,
                ))
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
