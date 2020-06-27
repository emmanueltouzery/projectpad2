use super::project_items_list::ProjectItem;
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
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    ServerSelected(Option<Server>),
    GotItems(Vec<ServerItem>),
}

#[derive(Clone)]
pub enum ServerItem {
    Website(ServerWebsite),
    PointOfInterest(ServerPointOfInterest),
    Note(ServerNote),
    ExtraUserAccount(ServerExtraUserAccount),
    Database(ServerDatabase),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    sender: relm::Sender<Vec<ServerItem>>,
    _channel: relm::Channel<Vec<ServerItem>>,
    cur_project_item: Option<Server>,
    server_items: Vec<ServerItem>,
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
        let (channel, sender) = relm::Channel::new(move |items: Vec<ServerItem>| {
            stream.emit(Msg::GotItems(items));
        });
        Model {
            _channel: channel,
            sender,
            db_sender,
            cur_project_item: None,
            server_items: vec![],
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
                self.model.server_items = items;
                self.update_contents_list();
            }
        }
    }

    fn update_contents_list(&mut self) {
        for child in self.contents_list.get_children() {
            self.contents_list.remove(&child);
        }
        let mut children_components = vec![];
        for item in &self.model.server_items {
            children_components.push(
                self.contents_list
                    .add_widget::<ServerItemListItem>(item.clone()),
            );
        }
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
                s.send(items).unwrap();
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
