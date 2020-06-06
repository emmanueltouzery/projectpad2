use super::project_poi_list_item::Model as PrjPoiItemModel;
use super::project_poi_list_item::ProjectPoiListItem;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::Project;
use projectpadsql::models::{
    EnvironmentType, ProjectNote, ProjectPointOfInterest, Server, ServerLink,
};
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Debug)]
pub enum ProjectItem {
    Server(Server),
    ServerLink(ServerLink),
    ProjectNote(ProjectNote),
    ProjectPointOfInterest(ProjectPointOfInterest),
}

#[derive(Msg)]
pub enum Msg {
    ActiveProjectChanged(Project),
    ActiveEnvironmentChanged(EnvironmentType),
    GotProjectItems(Vec<ProjectItem>),
    ProjectItemSelected(Option<usize>),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    relm: relm::Relm<ProjectItemsList>,
    project: Option<Project>,
    environment: EnvironmentType,
    project_items: Vec<ProjectItem>,
    _channel: relm::Channel<Vec<ProjectItem>>,
    sender: relm::Sender<Vec<ProjectItem>>,
}

#[widget]
impl Widget for ProjectItemsList {
    fn init_view(&mut self) {
        self.update_items_list();
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |prj_items: Vec<ProjectItem>| {
            stream.emit(Msg::GotProjectItems(prj_items));
        });
        Model {
            relm: relm.clone(),
            project: None,
            environment: EnvironmentType::EnvProd,
            project_items: vec![],
            sender,
            _channel: channel,
            db_sender,
        }
    }

    fn fetch_project_items(&mut self) {
        let s = self.model.sender.clone();
        let cur_project_id = self.model.project.as_ref().map(|p| p.id);
        let env = self.model.environment;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project_note::dsl as pnt;
                use projectpadsql::schema::project_point_of_interest::dsl as ppoi;
                use projectpadsql::schema::server::dsl as srv;
                use projectpadsql::schema::server_link::dsl as lsrv;
                let (servers, lsrvs, prj_notes, prj_pois) = match cur_project_id {
                    Some(pid) => {
                        let srvs = srv::server
                            .filter(srv::project_id.eq(pid).and(srv::environment.eq(env)))
                            .order(srv::desc.asc())
                            .load::<Server>(sql_conn)
                            .unwrap();
                        let lsrvs = lsrv::server_link
                            .filter(lsrv::project_id.eq(pid).and(lsrv::environment.eq(env)))
                            .order(lsrv::desc.asc())
                            .load::<ServerLink>(sql_conn)
                            .unwrap();
                        let mut prj_query = pnt::project_note
                            .filter(pnt::project_id.eq(pid))
                            .into_boxed();
                        match env {
                            EnvironmentType::EnvProd => {
                                prj_query = prj_query.filter(pnt::has_prod.eq(true))
                            }
                            EnvironmentType::EnvUat => {
                                prj_query = prj_query.filter(pnt::has_uat.eq(true))
                            }
                            EnvironmentType::EnvStage => {
                                prj_query = prj_query.filter(pnt::has_stage.eq(true))
                            }
                            EnvironmentType::EnvDevelopment => {
                                prj_query = prj_query.filter(pnt::has_dev.eq(true))
                            }
                        };
                        let prj_notes = prj_query
                            .order(pnt::title.asc())
                            .load::<ProjectNote>(sql_conn)
                            .unwrap();
                        let prj_pois = ppoi::project_point_of_interest
                            .filter(ppoi::project_id.eq(pid))
                            .order(ppoi::desc.asc())
                            .load::<ProjectPointOfInterest>(sql_conn)
                            .unwrap();
                        (srvs, lsrvs, prj_notes, prj_pois)
                    }
                    None => (vec![], vec![], vec![], vec![]),
                };

                let mut items: Vec<_> = servers.into_iter().map(ProjectItem::Server).collect();
                items.extend(&mut lsrvs.into_iter().map(ProjectItem::ServerLink));
                items.extend(&mut prj_notes.into_iter().map(ProjectItem::ProjectNote));
                items.extend(
                    &mut prj_pois
                        .into_iter()
                        .map(ProjectItem::ProjectPointOfInterest),
                );

                s.send(items).unwrap();
            }))
            .unwrap();
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ActiveProjectChanged(project) => {
                self.model.project = Some(project);
                self.fetch_project_items();
            }
            Msg::GotProjectItems(items) => {
                if let Some(vadj) = self.scroll.get_vadjustment() {
                    vadj.set_value(0.0);
                }
                self.model.project_items = items;
                self.update_items_list();
            }
            Msg::ActiveEnvironmentChanged(env) => {
                self.model.environment = env;
                // TODO gtk actually supports listbox filters...
                self.update_items_list();
            }
            Msg::ProjectItemSelected(row_idx) => println!(
                "selected {:?}",
                row_idx.map(|idx| self.model.project_items.get(idx))
            ),
        }
    }

    fn to_item_model(project_item: &ProjectItem) -> PrjPoiItemModel {
        match project_item {
            ProjectItem::Server(srv) => PrjPoiItemModel {
                text: srv.desc.clone(),
                secondary_desc: Some(srv.username.clone()),
                group_name: srv.group_name.as_ref().cloned(),
            },
            ProjectItem::ServerLink(link) => PrjPoiItemModel {
                text: link.desc.clone(),
                secondary_desc: None,
                group_name: link.group_name.as_ref().cloned(),
            },
            ProjectItem::ProjectNote(note) => PrjPoiItemModel {
                text: note.title.clone(),
                secondary_desc: None,
                group_name: note.group_name.as_ref().cloned(),
            },
            ProjectItem::ProjectPointOfInterest(poi) => PrjPoiItemModel {
                text: poi.desc.clone(),
                secondary_desc: Some(poi.text.clone()),
                group_name: poi.group_name.as_ref().cloned(),
            },
        }
    }

    fn update_items_list(&mut self) {
        for child in self.project_items_list.get_children() {
            self.project_items_list.remove(&child);
        }
        for project_item in &self.model.project_items {
            let _child = self
                .project_items_list
                .add_widget::<ProjectPoiListItem>(Self::to_item_model(project_item));
        }
    }

    view! {
        #[name="scroll"]
        gtk::ScrolledWindow {
            #[name="project_items_list"]
            gtk::ListBox {
                row_selected(_, row) => Msg::ProjectItemSelected(row.map(|r| r.get_index() as usize))
            }
        }
    }
}
