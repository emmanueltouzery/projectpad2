use super::environment_list_item::EnvironmentListItem;
use super::project_poi_list_item::Model as PrjPoiItemModel;
use super::project_poi_list_item::ProjectPoiListItem;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::Project;
use projectpadsql::models::{EnvironmentType, ProjectNote, ProjectPointOfInterest, Server};
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

type ProjectItems = (Vec<Server>, Vec<ProjectNote>, Vec<ProjectPointOfInterest>);

#[derive(Msg)]
pub enum Msg {
    EventSelected,
    ActiveProjectChanged(Project),
    GotProjectPois(ProjectItems),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    relm: relm::Relm<ProjectItemsList>,
    project: Option<Project>,
    servers: Vec<Server>,
    project_notes: Vec<ProjectNote>,
    project_pois: Vec<ProjectPointOfInterest>,
    _channel: relm::Channel<ProjectItems>,
    sender: relm::Sender<ProjectItems>,
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
        let (channel, sender) = relm::Channel::new(move |prj_items: ProjectItems| {
            stream.emit(Msg::GotProjectPois(prj_items));
        });
        Model {
            relm: relm.clone(),
            project: None,
            servers: vec![],
            project_notes: vec![],
            project_pois: vec![],
            sender,
            _channel: channel,
            db_sender,
        }
    }

    fn fetch_project_items(&mut self) {
        let s = self.model.sender.clone();
        let cur_project_id = self.model.project.as_ref().map(|p| p.id);
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project_note::dsl as pnt;
                use projectpadsql::schema::project_point_of_interest::dsl as ppoi;
                use projectpadsql::schema::server::dsl as srv;
                let (servers, prj_notes, prj_pois) = match cur_project_id {
                    Some(pid) => {
                        let srvs = srv::server
                            .filter(srv::project_id.eq(pid))
                            .order(srv::desc.asc())
                            .load::<Server>(sql_conn)
                            .unwrap();
                        let prj_notes = pnt::project_note
                            .filter(pnt::project_id.eq(pid))
                            .order(pnt::title.asc())
                            .load::<ProjectNote>(sql_conn)
                            .unwrap();
                        let prj_pois = ppoi::project_point_of_interest
                            .filter(ppoi::project_id.eq(pid))
                            .order(ppoi::desc.asc())
                            .load::<ProjectPointOfInterest>(sql_conn)
                            .unwrap();
                        (srvs, prj_notes, prj_pois)
                    }
                    None => (vec![], vec![], vec![]),
                };

                s.send((servers, prj_notes, prj_pois)).unwrap();
            }))
            .unwrap();
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::EventSelected => {}
            Msg::ActiveProjectChanged(project) => {
                self.model.project = Some(project);
                self.fetch_project_items();
            }
            Msg::GotProjectPois(pois) => {
                if let Some(vadj) = self.scroll.get_vadjustment() {
                    vadj.set_value(0.0);
                }
                self.model.servers = pois.0;
                self.model.project_notes = pois.1;
                self.model.project_pois = pois.2;
                self.update_items_list();
            }
        }
    }

    fn add_items_list_environment(&mut self, env: EnvironmentType) {
        let mut servers = self
            .model
            .servers
            .iter()
            .filter(|p| p.environment == EnvironmentType::EnvProd)
            .peekable();
        let matches_env = |note: &&ProjectNote| match env {
            EnvironmentType::EnvProd => note.has_prod,
            EnvironmentType::EnvUat => note.has_uat,
            EnvironmentType::EnvStage => note.has_stage,
            EnvironmentType::EnvDevelopment => note.has_dev,
        };
        let mut project_notes = self
            .model
            .project_notes
            .iter()
            .filter(matches_env)
            .peekable();
        if servers.peek().is_some() || project_notes.peek().is_some() {
            let _child = self
                .project_items_list
                .add_widget::<EnvironmentListItem>(env);
            for prj_note in project_notes {
                let _child =
                    self.project_items_list
                        .add_widget::<ProjectPoiListItem>(PrjPoiItemModel {
                            text: prj_note.title.clone(),
                            secondary_desc: None,
                        });
            }
            for server in servers {
                let _child =
                    self.project_items_list
                        .add_widget::<ProjectPoiListItem>(PrjPoiItemModel {
                            text: server.desc.clone(),
                            secondary_desc: Some(server.username.clone()),
                        });
            }
        }
    }

    fn update_items_list(&mut self) {
        for child in self.project_items_list.get_children() {
            self.project_items_list.remove(&child);
        }
        if let Some(Project {
            has_prod,
            has_uat,
            has_stage,
            has_dev,
            ..
        }) = self.model.project
        {
            for prj_poi in &self.model.project_pois {
                let _child =
                    self.project_items_list
                        .add_widget::<ProjectPoiListItem>(PrjPoiItemModel {
                            text: prj_poi.desc.clone(),
                            secondary_desc: Some(prj_poi.text.clone()),
                        });
            }
            if has_prod {
                self.add_items_list_environment(EnvironmentType::EnvProd);
            }
            if has_uat {
                self.add_items_list_environment(EnvironmentType::EnvUat);
            }
            if has_stage {
                self.add_items_list_environment(EnvironmentType::EnvStage);
            }
            if has_dev {
                self.add_items_list_environment(EnvironmentType::EnvDevelopment);
            }
        }
    }

    view! {
        #[name="scroll"]
        gtk::ScrolledWindow {
            #[name="project_items_list"]
            gtk::ListBox {}
        }
    }
}
