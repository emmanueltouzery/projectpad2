use super::project_poi_list_item::ProjectPoiListItem;
use super::win::ProjectPoi;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::Project;
use projectpadsql::models::Server;
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    EventSelected,
    ActiveProjectChanged(i32),
    GotProjectPois(Vec<Server>),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    relm: relm::Relm<ProjectItemsList>,
    project_id: Option<i32>,
    project_pois: Vec<Server>,
    _channel: relm::Channel<Vec<Server>>,
    sender: relm::Sender<Vec<Server>>,
}

#[widget]
impl Widget for ProjectItemsList {
    fn init_view(&mut self) {
        self.update_items_list();
        relm::connect!(
            self.model.relm,
            self.project_items_list,
            connect_row_selected(_, _),
            Msg::EventSelected
        );
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |prjs: Vec<Server>| {
            println!("emitting {}", prjs.len());
            stream.emit(Msg::GotProjectPois(prjs));
        });
        Model {
            relm: relm.clone(),
            project_id: None,
            project_pois: vec![],
            sender,
            _channel: channel,
            db_sender,
        }
    }

    fn fetch_project_items(&mut self) {
        let s = self.model.sender.clone();
        let cur_project_id = self.model.project_id;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server::dsl::*;
                let prj_pois = match cur_project_id {
                    Some(pid) => server
                        .filter(project_id.eq(pid))
                        .order(desc.asc())
                        .load::<Server>(sql_conn)
                        .unwrap(),
                    None => vec![],
                };
                println!("loaded prjs pois: {}", prj_pois.len());
                s.send(prj_pois).unwrap();
            }))
            .unwrap();
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::EventSelected => {}
            Msg::ActiveProjectChanged(pid) => {
                println!("project changed!");
                self.model.project_id = Some(pid);
                self.fetch_project_items();
            }
            Msg::GotProjectPois(pois) => {
                self.model.project_pois = pois;
                self.update_items_list();
            }
        }
    }

    fn update_items_list(&mut self) {
        for child in self.project_items_list.get_children() {
            self.project_items_list.remove(&child);
        }
        for project_poi in &self.model.project_pois {
            let _child = self
                .project_items_list
                .add_widget::<ProjectPoiListItem>(project_poi.clone());
        }
    }

    view! {
        gtk::ScrolledWindow {
            #[name="project_items_list"]
            gtk::ListBox {}
        }
    }
}
