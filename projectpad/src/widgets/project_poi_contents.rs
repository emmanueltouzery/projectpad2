use super::project_items_list::ProjectItem;
use super::project_poi_item_list_item::ProjectPoiItemListItem;
use super::win::ProjectPoiItem;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{Server, ServerPointOfInterest, ServerWebsite};
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

type ItemTypes = (Vec<ServerWebsite>, Vec<ServerPointOfInterest>);

#[derive(Msg)]
pub enum Msg {
    ProjectItemSelected(Option<ProjectItem>),
    GotItems(ItemTypes),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    sender: relm::Sender<ItemTypes>,
    _channel: relm::Channel<ItemTypes>,
    cur_project_item: Option<ProjectItem>,
    server_wwws: Vec<ServerWebsite>,
    server_pois: Vec<ServerPointOfInterest>,
}

#[widget]
impl Widget for ProjectPoiContents {
    fn init_view(&mut self) {
        self.contents_list
            .get_style_context()
            .add_class("item_list");
        self.update_contents_list();
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |items: ItemTypes| {
            stream.emit(Msg::GotItems(items));
        });
        Model {
            _channel: channel,
            sender,
            db_sender,
            cur_project_item: None,
            server_wwws: vec![],
            server_pois: vec![],
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectItemSelected(pi) => {
                self.model.cur_project_item = pi;
                self.fetch_items();
            }
            Msg::GotItems(items) => {
                self.model.server_wwws = items.0;
                self.model.server_pois = items.1;
                self.update_contents_list();
            }
        }
    }

    fn update_contents_list(&mut self) {
        for child in self.contents_list.get_children() {
            self.contents_list.remove(&child);
        }
        for item in &self.model.server_wwws {
            let child = self
                .contents_list
                .add_widget::<ProjectPoiItemListItem>(ProjectPoiItem {
                    name: item.desc.clone(),
                });
        }
        for item in &self.model.server_pois {
            let child = self
                .contents_list
                .add_widget::<ProjectPoiItemListItem>(ProjectPoiItem {
                    name: item.desc.clone(),
                });
        }
    }

    fn fetch_items(&mut self) {
        let s = self.model.sender.clone();
        let cur_server_id = match self.model.cur_project_item {
            Some(ProjectItem::Server(Server { id, .. })) => Some(id),
            _ => None,
        };
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
                use projectpadsql::schema::server_website::dsl as srv_www;
                let items = match cur_server_id {
                    Some(sid) => {
                        let srv_wwws = srv_www::server_website
                            .filter(srv_www::server_id.eq(sid))
                            .order(srv_www::desc.asc())
                            .load::<ServerWebsite>(sql_conn)
                            .unwrap();
                        let srv_pois = srv_poi::server_point_of_interest
                            .filter(srv_poi::server_id.eq(sid))
                            .order(srv_poi::desc.asc())
                            .load::<ServerPointOfInterest>(sql_conn)
                            .unwrap();
                        (srv_wwws, srv_pois)
                    }
                    None => (vec![], vec![]),
                };
                s.send(items).unwrap();
            }))
            .unwrap();
    }

    view! {
        #[name="contents_list"]
        gtk::ListBox {
            selection_mode: gtk::SelectionMode::None,
        }
    }
}
