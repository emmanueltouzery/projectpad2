use super::project_items_list::ProjectItem;
use super::project_poi_header::ProjectPoiHeader;
use super::project_search_header::ProjectSearchHeader;
use super::server_item_list_item::ServerItemListItem;
use super::server_poi_contents::ServerItem;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use itertools::Itertools;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase, ServerExtraUserAccount,
    ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;

macro_rules! group_by {
    ( $x:ident, $p:expr, $k:ident ) => {
        let mut $x = HashMap::new();
        for (key, group) in &$p.into_iter().group_by(|w| w.$k) {
            $x.insert(key, group.collect::<Vec<_>>());
        }
    };
}

pub struct SearchResult {
    pub projects: Vec<Project>,
    pub project_notes: Vec<ProjectNote>,
    pub project_pois: Vec<ProjectPointOfInterest>,
    pub server_links: Vec<ServerLink>,
    pub servers: Vec<Server>,
    pub server_databases: Vec<ServerDatabase>,
    pub server_extra_users: Vec<ServerExtraUserAccount>,
    pub server_notes: Vec<ServerNote>,
    pub server_pois: Vec<ServerPointOfInterest>,
    pub server_websites: Vec<ServerWebsite>,
}

#[derive(Clone, Debug)]
enum ProjectPadItem {
    Project(Project),
    ProjectNote(ProjectNote),
    ProjectPoi(ProjectPointOfInterest),
    ServerLink(ServerLink),
    Server(Server),
    ServerDatabase(ServerDatabase),
    ServerExtraUserAccount(ServerExtraUserAccount),
    ServerNote(ServerNote),
    ServerPoi(ServerPointOfInterest),
    ServerWebsite(ServerWebsite),
}

type SearchDisplay = Vec<ProjectPadItem>;

#[derive(Msg)]
pub enum Msg {
    FilterChanged(Option<String>),
    GotSearchResult(SearchResult),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    sender: relm::Sender<SearchResult>,
    search_display: SearchDisplay,
    project_components: Vec<relm::Component<ProjectSearchHeader>>,
    project_poi_components: Vec<relm::Component<ProjectPoiHeader>>,
    server_item_components: Vec<relm::Component<ServerItemListItem>>,
}

#[widget]
impl Widget for SearchView {
    fn init_view(&mut self) {
        self.fetch_search_results();
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |search_r: SearchResult| {
            stream.emit(Msg::GotSearchResult(search_r));
        });
        Model {
            db_sender,
            sender,
            search_display: vec![],
            project_components: vec![],
            project_poi_components: vec![],
            server_item_components: vec![],
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::FilterChanged(filter) => {
                println!("{:?}", filter);
                let indices = match filter {
                    None => HashSet::new(),
                    Some(f) => self.find_selected_indices(&f),
                };
                self.search_result_box
                    .set_filter_func(Some(Box::new(move |row| {
                        indices.contains(&row.get_index())
                    })));
                self.search_result_box.invalidate_filter();
            }
            Msg::GotSearchResult(search_result) => {
                self.refresh_display(search_result);
            }
        }
    }

    // careful there, we must bubble up the items. If a server note matches,
    // we must bring in also the parent server and its parent project.
    fn find_selected_indices(&self, filter: &str) -> HashSet<i32> {
        let mut row_ids = HashSet::new();
        let mut server_ids_indices = HashMap::new();
        let mut project_ids_indices = HashMap::new();
        let mut server_ids = HashSet::new();
        let matches = |contents: &str| contents.to_lowercase().contains(&filter.to_lowercase());
        for (pos, item) in self.model.search_display.iter().enumerate() {
            match item {
                ProjectPadItem::Project(p) => {
                    if matches(&p.name) {
                        row_ids.insert(p.id);
                    }
                    project_ids_indices.insert(p.id, pos as i32);
                }
                ProjectPadItem::ProjectNote(pn) => {
                    if matches(&pn.title) || matches(&pn.contents) {
                        row_ids.insert(pos as i32);
                        row_ids.insert(*project_ids_indices.get(&pn.project_id).unwrap());
                    }
                }
                ProjectPadItem::ServerPoi(poi) => {
                    if matches(&poi.desc) || matches(&poi.text) || matches(&poi.path) {
                        row_ids.insert(pos as i32);
                        server_ids.insert(poi.server_id);
                    }
                }
                ProjectPadItem::ServerDatabase(db) => {
                    if matches(&db.desc) || matches(&db.name) || matches(&db.text) {
                        row_ids.insert(pos as i32);
                        server_ids.insert(db.server_id);
                    }
                }
                ProjectPadItem::ServerWebsite(www) => {
                    if matches(&www.desc) || matches(&www.url) || matches(&www.text) {
                        row_ids.insert(pos as i32);
                        server_ids.insert(www.server_id);
                    }
                } // TODO db desc or name...
                ProjectPadItem::ProjectPoi(poi) => {
                    if matches(&poi.desc) || matches(&poi.text) || matches(&poi.path) {
                        row_ids.insert(pos as i32);
                        row_ids.insert(*project_ids_indices.get(&poi.project_id).unwrap());
                    }
                }
                ProjectPadItem::ServerLink(link) => {
                    if matches(&link.desc) {
                        row_ids.insert(pos as i32);
                        row_ids.insert(*project_ids_indices.get(&link.project_id).unwrap());
                    }
                }
                ProjectPadItem::Server(srv) => {
                    if matches(&srv.desc) || matches(&srv.ip) || matches(&srv.text) {
                        row_ids.insert(pos as i32);
                        row_ids.insert(*project_ids_indices.get(&srv.project_id).unwrap());
                    }
                    server_ids_indices.insert(srv.id, pos as i32);
                }
                ProjectPadItem::ServerExtraUserAccount(usr) => {
                    if matches(&usr.desc) {
                        row_ids.insert(pos as i32);
                        server_ids.insert(usr.server_id);
                    }
                }
                ProjectPadItem::ServerNote(note) => {
                    if matches(&note.title) || matches(&note.contents) {
                        row_ids.insert(pos as i32);
                        server_ids.insert(note.server_id);
                    }
                }
            }
        }
        for server_id in server_ids {
            let server_idx = *server_ids_indices.get(&server_id).unwrap();
            row_ids.insert(server_idx);
            let server = match &self.model.search_display[server_idx as usize] {
                ProjectPadItem::Server(srv) => srv,
                x => panic!("Expected a server, got {:?}", x),
            };
            row_ids.insert(*project_ids_indices.get(&server.project_id).unwrap());
        }
        row_ids
    }

    fn refresh_display(&mut self, search_result: SearchResult) {
        println!("refresh display");
        let mut search_display = vec![];
        self.model.project_components.clear();
        for child in self.search_result_box.get_children() {
            self.search_result_box.remove(&child);
        }

        // these group_bys speed up the lookups, not that the speed
        // was worrying me. another thing they give us is that we
        // can easily take ownership of the contents so that we
        // store the values in self.model.search_display without cloning.
        //
        // note that we rely on the fact that items are properly
        // sorted for the group_by! for instance server children
        // must be sorted by server_id.

        group_by!(websites_by_server, search_result.server_websites, server_id);
        group_by!(notes_by_server, search_result.server_notes, server_id);
        group_by!(users_by_server, search_result.server_extra_users, server_id);
        group_by!(dbs_by_server, search_result.server_databases, server_id);
        group_by!(pois_by_server, search_result.server_pois, server_id);
        group_by!(
            serverlinks_by_project,
            search_result.server_links,
            project_id
        );
        group_by!(
            projectnotes_by_project,
            search_result.project_notes,
            project_id
        );
        group_by!(
            projectpois_by_project,
            search_result.project_pois,
            project_id
        );
        group_by!(servers_by_project, search_result.servers, project_id);

        for project in search_result.projects {
            self.model.project_components.push(
                self.search_result_box
                    .add_widget::<ProjectSearchHeader>(project.clone()),
            );
            let project_id = project.id;
            search_display.push(ProjectPadItem::Project(project));
            for server in servers_by_project
                .remove(&project_id)
                .unwrap_or_else(|| vec![])
            {
                let server_id = server.id;
                self.model.project_poi_components.push(
                    self.search_result_box
                        .add_widget::<ProjectPoiHeader>(Some(ProjectItem::Server(server.clone()))),
                );
                search_display.push(ProjectPadItem::Server(server));

                for server_website in websites_by_server
                    .remove(&server_id)
                    .unwrap_or_else(|| vec![])
                {
                    self.model.server_item_components.push(
                        self.search_result_box.add_widget::<ServerItemListItem>(
                            ServerItem::Website(server_website.clone()),
                        ),
                    );
                    search_display.push(ProjectPadItem::ServerWebsite(server_website));
                }
                for server_note in notes_by_server.remove(&server_id).unwrap_or_else(|| vec![]) {
                    self.model.server_item_components.push(
                        self.search_result_box
                            .add_widget::<ServerItemListItem>(ServerItem::Note(
                                server_note.clone(),
                            )),
                    );
                    search_display.push(ProjectPadItem::ServerNote(server_note));
                }
                for server_user in users_by_server.remove(&server_id).unwrap_or_else(|| vec![]) {
                    self.model.server_item_components.push(
                        self.search_result_box.add_widget::<ServerItemListItem>(
                            ServerItem::ExtraUserAccount(server_user.clone()),
                        ),
                    );
                    search_display.push(ProjectPadItem::ServerExtraUserAccount(server_user));
                }
                for server_db in dbs_by_server.remove(&server_id).unwrap_or_else(|| vec![]) {
                    self.model.server_item_components.push(
                        self.search_result_box.add_widget::<ServerItemListItem>(
                            ServerItem::Database(server_db.clone()),
                        ),
                    );
                    search_display.push(ProjectPadItem::ServerDatabase(server_db));
                }
                for server_poi in pois_by_server.remove(&server_id).unwrap_or_else(|| vec![]) {
                    self.model.server_item_components.push(
                        self.search_result_box.add_widget::<ServerItemListItem>(
                            ServerItem::PointOfInterest(server_poi.clone()),
                        ),
                    );
                    search_display.push(ProjectPadItem::ServerPoi(server_poi));
                }
            }
            for server_link in serverlinks_by_project
                .remove(&project_id)
                .unwrap_or_else(|| vec![])
            {
                self.model.project_poi_components.push(
                    self.search_result_box.add_widget::<ProjectPoiHeader>(Some(
                        ProjectItem::ServerLink(server_link.clone()),
                    )),
                );
                search_display.push(ProjectPadItem::ServerLink(server_link));
            }
            for project_note in projectnotes_by_project
                .remove(&project_id)
                .unwrap_or_else(|| vec![])
            {
                self.model.project_poi_components.push(
                    self.search_result_box.add_widget::<ProjectPoiHeader>(Some(
                        ProjectItem::ProjectNote(project_note.clone()),
                    )),
                );
                search_display.push(ProjectPadItem::ProjectNote(project_note));
            }
            for project_poi in projectpois_by_project
                .remove(&project_id)
                .unwrap_or_else(|| vec![])
            {
                self.model.project_poi_components.push(
                    self.search_result_box.add_widget::<ProjectPoiHeader>(Some(
                        ProjectItem::ProjectPointOfInterest(project_poi.clone()),
                    )),
                );
                search_display.push(ProjectPadItem::ProjectPoi(project_poi));
            }
        }
        self.model.search_display = search_display; // TODO don't think i need a model member & a clone
        println!("refresh display done");
    }

    fn fetch_search_results(&self) {
        let s = self.model.sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                s.send(SearchResult {
                    projects: Self::filter_projects(sql_conn),
                    project_notes: Self::filter_project_notes(sql_conn),
                    project_pois: Self::filter_project_pois(sql_conn),
                    servers: Self::filter_servers(sql_conn),
                    server_notes: Self::filter_server_notes(sql_conn),
                    server_links: Self::filter_server_links(sql_conn),
                    server_pois: Self::filter_server_pois(sql_conn),
                    server_databases: Self::filter_server_databases(sql_conn),
                    server_extra_users: Self::filter_server_extra_users(sql_conn),
                    server_websites: Self::filter_server_websites(sql_conn)
                        .into_iter()
                        .map(|p| p.0)
                        .collect::<Vec<_>>(),
                })
                .unwrap();
            }))
            .unwrap();
    }

    fn filter_projects(db_conn: &SqliteConnection) -> Vec<Project> {
        use projectpadsql::schema::project::dsl::*;
        project.order(name.asc()).load::<Project>(db_conn).unwrap()
    }

    fn filter_project_pois(db_conn: &SqliteConnection) -> Vec<ProjectPointOfInterest> {
        use projectpadsql::schema::project_point_of_interest::dsl::*;
        project_point_of_interest
            .order(project_id.asc()) // we must order because we group_by later!
            .load::<ProjectPointOfInterest>(db_conn)
            .unwrap()
    }

    fn filter_project_notes(db_conn: &SqliteConnection) -> Vec<ProjectNote> {
        use projectpadsql::schema::project_note::dsl::*;
        project_note
            .order(project_id.asc()) // we must order because we group_by later!
            .load::<ProjectNote>(db_conn)
            .unwrap()
    }

    fn filter_server_notes(db_conn: &SqliteConnection) -> Vec<ServerNote> {
        use projectpadsql::schema::server_note::dsl::*;
        server_note
            .order(server_id.asc()) // we must order because we group_by later!
            .load::<ServerNote>(db_conn)
            .unwrap()
    }

    fn filter_server_links(db_conn: &SqliteConnection) -> Vec<ServerLink> {
        use projectpadsql::schema::server_link::dsl::*;
        server_link
            .order(project_id.asc()) // we must order because we group_by later!
            .load::<ServerLink>(db_conn)
            .unwrap()
    }

    fn filter_server_extra_users(db_conn: &SqliteConnection) -> Vec<ServerExtraUserAccount> {
        use projectpadsql::schema::server_extra_user_account::dsl::*;
        server_extra_user_account
            .order(server_id.asc()) // we must order because we group_by later!
            .load::<ServerExtraUserAccount>(db_conn)
            .unwrap()
    }

    fn filter_server_pois(db_conn: &SqliteConnection) -> Vec<ServerPointOfInterest> {
        use projectpadsql::schema::server_point_of_interest::dsl::*;
        server_point_of_interest
            .order(server_id.asc()) // we must order because we group_by later!
            .load::<ServerPointOfInterest>(db_conn)
            .unwrap()
    }

    fn filter_server_databases(db_conn: &SqliteConnection) -> Vec<ServerDatabase> {
        use projectpadsql::schema::server_database::dsl::*;
        server_database
            .order(server_id.asc()) // we must order because we group_by later!
            .load::<ServerDatabase>(db_conn)
            .unwrap()
    }

    fn filter_servers(db_conn: &SqliteConnection) -> Vec<Server> {
        use projectpadsql::schema::server::dsl::*;
        server
            .order(project_id.asc()) // we must order because we group_by later!
            .load::<Server>(db_conn)
            .unwrap()
    }

    fn filter_server_websites(
        db_conn: &SqliteConnection,
    ) -> Vec<(ServerWebsite, Option<ServerDatabase>)> {
        use projectpadsql::schema::server_database::dsl as db;
        use projectpadsql::schema::server_website::dsl::*;
        server_website
            .left_outer_join(db::server_database)
            .order(server_id.asc()) // we must order because we group_by later!
            .load::<(ServerWebsite, Option<ServerDatabase>)>(db_conn)
            .unwrap()
    }

    view! {
        gtk::ScrolledWindow {
            #[name="search_result_box"]
            gtk::ListBox {
            }
        }
    }
}
