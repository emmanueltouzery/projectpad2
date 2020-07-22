use super::project_poi_list_item::Model as PrjPoiItemModel;
use super::project_poi_list_item::ProjectPoiListItem;
use crate::icons::*;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{
    EnvironmentType, InterestType, Project, ProjectNote, ProjectPointOfInterest, Server,
    ServerAccessType, ServerLink, ServerType,
};
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::collections::{BTreeSet, HashMap};
use std::sync::mpsc;

type ChannelData = (
    (Vec<ProjectItem>, HashMap<i32, String>),
    Option<EnvironmentType>,
    Option<ProjectItem>,
);

#[derive(Debug, Clone, Eq, PartialEq)]
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
    ProjectItemSelectedFromElsewhere((Project, Option<EnvironmentType>, Option<ProjectItem>)),
    FocusRow(gtk::ListBoxRow),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    relm: relm::Relm<ProjectItemsList>,
    project: Option<Project>,
    environment: EnvironmentType,
    project_items: Vec<ProjectItem>,
    project_item_groups_start_indexes: HashMap<i32, String>,
    _channel: relm::Channel<ChannelData>,
    sender: relm::Sender<ChannelData>,
}

#[widget]
impl Widget for ProjectItemsList {
    fn init_view(&mut self) {
        self.project_items_list
            .set_focus_vadjustment(&self.scroll.get_vadjustment().unwrap());
        self.update_items_list();
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |ch_data: ChannelData| {
            stream.emit(Msg::GotProjectItems(ch_data));
        });
        Model {
            relm: relm.clone(),
            project: None,
            environment: EnvironmentType::EnvProd,
            project_items: Vec::new(),
            project_item_groups_start_indexes: HashMap::new(),
            sender,
            _channel: channel,
            db_sender,
        }
    }

    fn add_items(
        items: &mut Vec<ProjectItem>,
        servers: &mut dyn Iterator<Item = Server>,
        lsrvs: &mut dyn Iterator<Item = ServerLink>,
        prj_notes: &mut dyn Iterator<Item = ProjectNote>,
        prj_pois: &mut dyn Iterator<Item = ProjectPointOfInterest>,
        group_name: Option<String>,
    ) {
        items.extend(
            servers
                .take_while(|s| s.group_name == group_name)
                .map(ProjectItem::Server),
        );
        items.extend(
            lsrvs
                .take_while(|s| s.group_name == group_name)
                .map(ProjectItem::ServerLink),
        );
        items.extend(
            prj_notes
                .take_while(|s| s.group_name == group_name)
                .map(ProjectItem::ProjectNote),
        );
        items.extend(
            prj_pois
                .take_while(|s| s.group_name == group_name)
                .map(ProjectItem::ProjectPointOfInterest),
        );
    }

    fn fetch_project_items_sql(
        sql_conn: &diesel::SqliteConnection,
        env: EnvironmentType,
        cur_project_id: Option<i32>,
    ) -> (
        Vec<Server>,
        Vec<ServerLink>,
        Vec<ProjectNote>,
        Vec<ProjectPointOfInterest>,
    ) {
        use projectpadsql::schema::project_note::dsl as pnt;
        use projectpadsql::schema::project_point_of_interest::dsl as ppoi;
        use projectpadsql::schema::server::dsl as srv;
        use projectpadsql::schema::server_link::dsl as lsrv;
        match cur_project_id {
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
                    EnvironmentType::EnvDevelopment => prj_query.filter(pnt::has_dev.eq(true)),
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
        }
    }

    fn fetch_project_items(
        &mut self,
        env_to_select: Option<EnvironmentType>,
        pi_to_select: Option<ProjectItem>,
    ) {
        let s = self.model.sender.clone();
        let cur_project_id = self.model.project.as_ref().map(|p| p.id);
        let env = self.model.environment;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                let (servers, lsrvs, prj_notes, prj_pois) =
                    Self::fetch_project_items_sql(sql_conn, env, cur_project_id);

                let mut group_names: BTreeSet<&String> = servers
                    .iter()
                    .filter_map(|s| s.group_name.as_ref())
                    .collect();
                group_names.extend(lsrvs.iter().filter_map(|s| s.group_name.as_ref()));
                group_names.extend(prj_notes.iter().filter_map(|s| s.group_name.as_ref()));
                group_names.extend(prj_pois.iter().filter_map(|s| s.group_name.as_ref()));
                let group_names: BTreeSet<String> =
                    group_names.iter().map(|s| s.to_string()).collect();

                let mut servers_iter = servers.into_iter();
                let mut lsrvs_iter = lsrvs.into_iter();
                let mut prj_notes_iter = prj_notes.into_iter();
                let mut prj_pois_iter = prj_pois.into_iter();

                let mut items = Vec::new();
                let mut group_start_indexes = HashMap::new();
                // this code relies on the sort order from the SQL query
                // to be the same as the one we process the results in.
                // notably we must have the nulls (no group) first.
                Self::add_items(
                    &mut items,
                    &mut servers_iter,
                    &mut lsrvs_iter,
                    &mut prj_notes_iter,
                    &mut prj_pois_iter,
                    None,
                );
                for group_name in group_names {
                    group_start_indexes.insert(items.len() as i32, group_name.clone());
                    Self::add_items(
                        &mut items,
                        &mut servers_iter,
                        &mut lsrvs_iter,
                        &mut prj_notes_iter,
                        &mut prj_pois_iter,
                        Some(group_name),
                    );
                }
                s.send((
                    (items, group_start_indexes),
                    env_to_select,
                    pi_to_select.as_ref().cloned(),
                ))
                .unwrap();
            }))
            .unwrap();
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ActiveProjectChanged(project) => {
                self.model.project = Some(project);
                self.fetch_project_items(None, None);
            }
            Msg::GotProjectItems((items, env, project_item)) => {
                println!("got items {:?} {:?}", env, project_item);
                if let Some(vadj) = self.scroll.get_vadjustment() {
                    vadj.set_value(0.0);
                }
                self.model.project_items = items.0;
                self.model.project_item_groups_start_indexes = items.1;
                self.update_items_list();
                let row_idx = self
                    .model
                    .project_items
                    .iter()
                    .position(|cur_pi| Some(cur_pi) == project_item.as_ref());
                println!("select position: {:?}", row_idx);
                let row = row_idx.and_then(|i| self.project_items_list.get_row_at_index(i as i32));
                self.project_items_list.select_row(row.as_ref());
                if let Some(r) = row {
                    // we need the timeout. We've just added the rows to the listbox,
                    // they are not realized yet. The scrolling doesn't work unless
                    // we allow the gtk thread to realize the list items
                    // https://discourse.gnome.org/t/listbox-programmatically-scroll-to-row/3844
                    relm::timeout(self.model.relm.stream(), 100, move || {
                        Msg::FocusRow(r.clone())
                    });
                }
            }
            Msg::ActiveEnvironmentChanged(env) => {
                self.model.environment = env;
                self.fetch_project_items(Some(env), None);
            }
            Msg::ProjectItemIndexSelected(row_idx) => {
                self.model.relm.stream().emit(Msg::ProjectItemSelected(
                    row_idx.and_then(|idx| self.model.project_items.get(idx).cloned()),
                ))
            }
            Msg::ProjectItemSelected(_) => {
                // meant for my parent
            }
            Msg::ProjectItemSelectedFromElsewhere((project, env, pi)) => {
                self.model.project = Some(project);
                if let Some(e) = env {
                    self.model.environment = e;
                }
                self.fetch_project_items(env, pi.clone());
            }
            Msg::FocusRow(r) => {
                r.grab_focus();
            }
        }
    }

    fn to_item_model(project_item: &ProjectItem) -> PrjPoiItemModel {
        match project_item {
            ProjectItem::Server(srv) => PrjPoiItemModel {
                text: srv.desc.clone(),
                secondary_desc: None,
                group_name: srv.group_name.as_ref().cloned(),
                icon: match (srv.server_type, srv.access_type) {
                    (ServerType::SrvDatabase, _) => Icon::DATABASE,
                    (ServerType::SrvReporting, _) => Icon::REPORTING,
                    (ServerType::SrvMonitoring, _) => Icon::MONITORING,
                    (ServerType::SrvHttpOrProxy, _) => Icon::HTTP,
                    (_, ServerAccessType::SrvAccessRdp) => Icon::WINDOWS,
                    (_, _) => Icon::SERVER,
                },
            },
            ProjectItem::ServerLink(link) => PrjPoiItemModel {
                text: link.desc.clone(),
                secondary_desc: None,
                group_name: link.group_name.as_ref().cloned(),
                icon: Icon::SERVER_LINK,
            },
            ProjectItem::ProjectNote(note) => PrjPoiItemModel {
                text: note.title.clone(),
                secondary_desc: None,
                group_name: note.group_name.as_ref().cloned(),
                icon: Icon::NOTE,
            },
            ProjectItem::ProjectPointOfInterest(poi) => PrjPoiItemModel {
                text: poi.desc.clone(),
                secondary_desc: Some(poi.text.clone()),
                group_name: poi.group_name.as_ref().cloned(),
                icon: match poi.interest_type {
                    InterestType::PoiLogFile => Icon::LOG_FILE,
                    InterestType::PoiConfigFile => Icon::CONFIG_FILE,
                    InterestType::PoiApplication => Icon::COG,
                    InterestType::PoiCommandToRun => Icon::TERMINAL,
                    InterestType::PoiCommandTerminal => Icon::TERMINAL,
                    InterestType::PoiBackupArchive => Icon::ARCHIVE,
                },
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
                if let Some(group_name) = indexes.get(&row.get_index()) {
                    let label = gtk::Label::new(Some(group_name));
                    label.get_style_context().add_class("project_item_header");
                    row.set_header(Some(&label));
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
