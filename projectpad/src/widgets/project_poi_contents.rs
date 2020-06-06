use super::project_items_list::ProjectItem;
use super::project_poi_item_list_item::ProjectPoiItemListItem;
use super::win::{ProjectPoi, ProjectPoiItem};
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{Server, ServerWebsite};
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    ProjectItemSelected(Option<ProjectItem>),
    GotItems(Vec<ServerWebsite>),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    sender: relm::Sender<Vec<ServerWebsite>>,
    cur_project_item: Option<ProjectItem>,
    items: Vec<ServerWebsite>,
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
        let (channel, sender) = relm::Channel::new(move |items: Vec<ServerWebsite>| {
            stream.emit(Msg::GotItems(items));
        });
        Model {
            sender,
            db_sender,
            cur_project_item: None,
            items: vec![],
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectItemSelected(pi) => {
                self.model.cur_project_item = pi;
                self.fetch_items();
            }
            Msg::GotItems(items) => {
                self.model.items = items;
                self.update_contents_list();
            }
        }
    }

    fn update_contents_list(&mut self) {
        for child in self.contents_list.get_children() {
            self.contents_list.remove(&child);
        }
        for item in &self.model.items {
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
                use projectpadsql::schema::server_website::dsl as srv_www;
                let items = match cur_server_id {
                    Some(sid) => srv_www::server_website
                        .filter(srv_www::server_id.eq(sid))
                        .order(srv_www::desc.asc())
                        .load::<ServerWebsite>(sql_conn)
                        .unwrap(),
                    None => vec![],
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
