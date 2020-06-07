use super::project_poi_list_item::Model as PrjPoiItemModel;
use super::project_poi_list_item::ProjectPoiListItem;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use itertools::Itertools;
use projectpadsql::models::Project;
use projectpadsql::models::{
    EnvironmentType, ProjectNote, ProjectPointOfInterest, Server, ServerLink,
};
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::mpsc;

type ChannelData = (Vec<ProjectItem>, HashSet<i32>);

#[derive(Debug, Clone)]
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
    GotProjectItems(ChannelData),
    ProjectItemIndexSelected(Option<usize>),
    ProjectItemSelected(Option<ProjectItem>),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    relm: relm::Relm<ProjectItemsList>,
    project: Option<Project>,
    environment: EnvironmentType,
    project_items: Vec<ProjectItem>,
    project_item_groups_start_indexes: HashSet<i32>,
    _channel: relm::Channel<ChannelData>,
    sender: relm::Sender<ChannelData>,
}

#[widget]
impl Widget for ProjectItemsList {
    fn init_view(&mut self) {
        self.update_items_list();
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |prj_items: ChannelData| {
            stream.emit(Msg::GotProjectItems(prj_items));
        });
        Model {
            relm: relm.clone(),
            project: None,
            environment: EnvironmentType::EnvProd,
            project_items: Vec::new(),
            project_item_groups_start_indexes: HashSet::new(),
            sender,
            _channel: channel,
            db_sender,
        }
    }

    fn add_items(
        items: &mut Vec<ProjectItem>,
        servers_by_group: &mut HashMap<Option<String>, Vec<Server>>,
        lsrvs_by_group: &mut HashMap<Option<String>, Vec<ServerLink>>,
        prj_notes_by_group: &mut HashMap<Option<String>, Vec<ProjectNote>>,
        prj_pois_by_group: &mut HashMap<Option<String>, Vec<ProjectPointOfInterest>>,
        group_name: Option<String>,
    ) {
        if let Some(servers) = servers_by_group.remove(&group_name) {
            items.extend(servers.into_iter().map(ProjectItem::Server));
        }
        if let Some(lsrvs) = lsrvs_by_group.remove(&group_name) {
            items.extend(lsrvs.into_iter().map(ProjectItem::ServerLink));
        }
        if let Some(notes) = prj_notes_by_group.remove(&group_name) {
            items.extend(notes.into_iter().map(ProjectItem::ProjectNote));
        }
        if let Some(pois) = prj_pois_by_group.remove(&group_name) {
            items.extend(pois.into_iter().map(ProjectItem::ProjectPointOfInterest));
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
                            .order((srv::group_name.asc(), srv::desc.asc()))
                            .load::<Server>(sql_conn)
                            .unwrap();
                        let lsrvs = lsrv::server_link
                            .filter(lsrv::project_id.eq(pid).and(lsrv::environment.eq(env)))
                            .order((lsrv::group_name.asc(), lsrv::desc.asc()))
                            .load::<ServerLink>(sql_conn)
                            .unwrap();
                        let mut prj_query = pnt::project_note
                            .filter(pnt::project_id.eq(pid))
                            .into_boxed();
                        prj_query = match env {
                            EnvironmentType::EnvProd => prj_query.filter(pnt::has_prod.eq(true)),
                            EnvironmentType::EnvUat => prj_query.filter(pnt::has_uat.eq(true)),
                            EnvironmentType::EnvStage => prj_query.filter(pnt::has_stage.eq(true)),
                            EnvironmentType::EnvDevelopment => {
                                prj_query.filter(pnt::has_dev.eq(true))
                            }
                        };
                        let prj_notes = prj_query
                            .order((pnt::group_name.asc(), pnt::title.asc()))
                            .load::<ProjectNote>(sql_conn)
                            .unwrap();
                        let prj_pois = ppoi::project_point_of_interest
                            .filter(ppoi::project_id.eq(pid))
                            .order((ppoi::group_name.asc(), ppoi::desc.asc()))
                            .load::<ProjectPointOfInterest>(sql_conn)
                            .unwrap();
                        (srvs, lsrvs, prj_notes, prj_pois)
                    }
                    None => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
                };

                let mut group_names: BTreeSet<String> =
                    servers.iter().filter_map(|s| s.group_name).collect();
                group_names.extend(lsrvs.iter().filter_map(|s| s.group_name));
                group_names.extend(prj_notes.iter().filter_map(|s| s.group_name));
                group_names.extend(prj_pois.iter().filter_map(|s| s.group_name));

                let mut servers_by_group_name = HashMap::new();
                for (key, group) in &servers.into_iter().group_by(|s| s.group_name) {
                    servers_by_group_name.insert(key, group.collect());
                }
                let mut lsrvs_by_group_name = HashMap::new();
                for (key, group) in &lsrvs.into_iter().group_by(|s| s.group_name) {
                    lsrvs_by_group_name.insert(key, group.collect());
                }
                let mut notes_by_group_name = HashMap::new();
                for (key, group) in &prj_notes.into_iter().group_by(|s| s.group_name) {
                    notes_by_group_name.insert(key, group.collect());
                }
                let mut pois_by_group_name = HashMap::new();
                for (key, group) in &prj_pois.into_iter().group_by(|s| s.group_name) {
                    pois_by_group_name.insert(key, group.collect());
                }

                let mut items = Vec::new();
                let mut group_start_indexes = HashSet::new();
                for group_name in group_names {
                    group_start_indexes.insert(items.len() as i32);
                    Self::add_items(
                        &mut items,
                        &mut servers_by_group_name,
                        &mut lsrvs_by_group_name,
                        &mut notes_by_group_name,
                        &mut pois_by_group_name,
                        Some(group_name),
                    );
                }
                Self::add_items(
                    &mut items,
                    &mut servers_by_group_name,
                    &mut lsrvs_by_group_name,
                    &mut notes_by_group_name,
                    &mut pois_by_group_name,
                    None,
                );

                s.send((items, group_start_indexes)).unwrap();
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
                self.model.project_items = items.0;
                self.model.project_item_groups_start_indexes = items.1;
                self.update_items_list();
            }
            Msg::ActiveEnvironmentChanged(env) => {
                self.model.environment = env;
                self.fetch_project_items();
            }
            Msg::ProjectItemIndexSelected(row_idx) => {
                self.model.relm.stream().emit(Msg::ProjectItemSelected(
                    row_idx.and_then(|idx| self.model.project_items.get(idx).cloned()),
                ))
            }
            Msg::ProjectItemSelected(_) => {
                // meant for my parent
            }
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
        let indexes = self.model.project_item_groups_start_indexes.clone();
        self.project_items_list
            .set_header_func(Some(Box::new(move |row, h| {
                if indexes.contains(&row.get_index()) {
                    row.set_header(Some(&gtk::Label::new(Some("hi"))));
                } else {
                    row.set_header::<gtk::ListBoxRow>(None)
                }
            })));
    }

    view! {
        #[name="scroll"]
        gtk::ScrolledWindow {
            #[name="project_items_list"]
            gtk::ListBox {
                row_selected(_, row) =>
                    Msg::ProjectItemIndexSelected(row.map(|r| r.get_index() as usize))
            }
        }
    }
}
