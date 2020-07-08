use super::project_items_list::ProjectItem;
use super::project_poi_header::ProjectPoiHeader;
use super::project_search_header::ProjectSearchHeader;
use super::server_item_list_item::ServerItemListItem;
use super::server_poi_contents::ServerItem;
use crate::sql_thread::SqlFunc;
use cairo::Context;
use diesel::prelude::*;
use gio::prelude::*;
use gio::subclass::prelude::*;
use glib::subclass;
use glib::subclass::prelude::*;
use glib::translate::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CellRendererState;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase, ServerExtraUserAccount,
    ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};
use std::collections::HashSet;
use std::sync::mpsc;

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

#[derive(Msg)]
pub enum Msg {
    FilterChanged(Option<String>),
    GotSearchResult(SearchResult),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    filter: Option<String>,
    sender: relm::Sender<SearchResult>,
    search_result: Option<SearchResult>,
    list_store: gtk::ListStore,
}

pub struct MyRendererPrivate {}

glib_wrapper! {
    pub struct MyRenderer(
        Object<subclass::simple::InstanceStruct<MyRendererPrivate>,
        subclass::simple::ClassStruct<MyRendererPrivate>,
        MyRendererClass>)
        @extends gtk::CellRenderer;

    match fn {
        get_type => || MyRendererPrivate::get_type().to_glib(),
    }
}

impl MyRenderer {
    pub fn new() -> Self {
        glib::Object::new(Self::static_type(), &[])
            .expect("Failed to create MyRenderer")
            .downcast()
            .expect("Created MyRenderer is of wrong type")
    }
}

// https://gitlab.gnome.org/GNOME/rhythmbox/-/blob/master/widgets/rb-cell-renderer-rating.c
// https://github.com/gtk-rs/examples/blob/master/src/bin/basic_subclass.rs
// https://people.gnome.org/~federico/blog/librsvg-gobject-in-rust.html
impl ObjectImpl for MyRendererPrivate {
    glib_object_impl!();

    fn set_property(&self, _obj: &glib::Object, _id: usize, _value: &glib::Value) {
        println!("set_property, {:?}, {:?}", _obj, _value.get::<i32>());
    }
}
impl ObjectSubclass for MyRendererPrivate {
    const NAME: &'static str = "MyRenderer";
    type ParentType = gtk::CellRenderer;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::glib_object_subclass!();

    fn class_init(klass: &mut Self::Class) {
        klass.install_properties(&[subclass::Property("project_id", |name| {
            glib::ParamSpec::int(
                name,
                "Project ID",
                "Project ID",
                0,
                i32::MAX,
                0,
                glib::ParamFlags::READWRITE,
            )
        })]);
    }

    fn new() -> Self {
        Self {}
    }
}
impl CellRendererImpl for MyRendererPrivate {
    fn render<P: IsA<gtk::Widget>>(
        &self,
        renderer: &gtk::CellRenderer,
        cr: &Context,
        widget: &P,
        background_area: &gtk::Rectangle,
        cell_area: &gtk::Rectangle,
        flags: CellRendererState,
    ) {
        // println!("render!!");
    }
}

#[widget]
impl Widget for SearchView {
    fn init_view(&mut self) {
        self.search_result_treeview
            .set_model(Some(&self.model.list_store));
        let col = gtk::TreeViewColumn::new();
        col.set_expand(true);
        let renderer = MyRenderer::new();
        col.pack_start(&renderer, true);
        col.add_attribute(&renderer, "project_id", 0);
        self.search_result_treeview.append_column(&col);
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |search_r: SearchResult| {
            stream.emit(Msg::GotSearchResult(search_r));
        });
        Model {
            filter: None,
            db_sender,
            sender,
            search_result: None,
            list_store: gtk::ListStore::new(&[glib::Type::I32]),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::FilterChanged(filter) => {
                self.model.filter = filter;
                self.fetch_search_results();
            }
            Msg::GotSearchResult(search_result) => {
                self.model.search_result = Some(search_result);
                self.refresh_display();
            }
        }
    }

    fn refresh_display(&self) {
        self.model.list_store.clear();
        if let Some(search_result) = &self.model.search_result {
            for project in &search_result.projects {
                let row = self.model.list_store.append();
                self.model
                    .list_store
                    .set_value(&row, 0, &project.id.to_value());
                // self.search_result_box
                //     .add_widget::<ProjectSearchHeader>(project.clone());
                // for server in search_result
                //     .servers
                //     .iter()
                //     .filter(|s| s.project_id == project.id)
                // {
                //     self.search_result_box
                //         .add_widget::<ProjectPoiHeader>(Some(ProjectItem::Server(server.clone())));

                //     for server_website in search_result
                //         .server_websites
                //         .iter()
                //         .filter(|sw| sw.server_id == server.id)
                //     {
                //         self.search_result_box.add_widget::<ServerItemListItem>(
                //             ServerItem::Website(server_website.clone()),
                //         );
                //     }
                //     for server_note in search_result
                //         .server_notes
                //         .iter()
                //         .filter(|sn| sn.server_id == server.id)
                //     {
                //         self.search_result_box
                //             .add_widget::<ServerItemListItem>(ServerItem::Note(
                //                 server_note.clone(),
                //             ));
                //     }
                //     for server_user in search_result
                //         .server_extra_users
                //         .iter()
                //         .filter(|su| su.server_id == server.id)
                //     {
                //         self.search_result_box.add_widget::<ServerItemListItem>(
                //             ServerItem::ExtraUserAccount(server_user.clone()),
                //         );
                //     }
                //     for server_db in search_result
                //         .server_databases
                //         .iter()
                //         .filter(|sd| sd.server_id == server.id)
                //     {
                //         self.search_result_box.add_widget::<ServerItemListItem>(
                //             ServerItem::Database(server_db.clone()),
                //         );
                //     }
                //     for server_poi in search_result
                //         .server_pois
                //         .iter()
                //         .filter(|sp| sp.server_id == server.id)
                //     {
                //         self.search_result_box.add_widget::<ServerItemListItem>(
                //             ServerItem::PointOfInterest(server_poi.clone()),
                //         );
                //     }
                // }
                // for server_link in search_result
                //     .server_links
                //     .iter()
                //     .filter(|s| s.project_id == project.id)
                // {
                //     self.search_result_box.add_widget::<ProjectPoiHeader>(Some(
                //         ProjectItem::ServerLink(server_link.clone()),
                //     ));
                // }
                // for project_note in search_result
                //     .project_notes
                //     .iter()
                //     .filter(|s| s.project_id == project.id)
                // {
                //     self.search_result_box.add_widget::<ProjectPoiHeader>(Some(
                //         ProjectItem::ProjectNote(project_note.clone()),
                //     ));
                // }
                // for project_poi in search_result
                //     .project_pois
                //     .iter()
                //     .filter(|s| s.project_id == project.id)
                // {
                //     self.search_result_box.add_widget::<ProjectPoiHeader>(Some(
                //         ProjectItem::ProjectPointOfInterest(project_poi.clone()),
                //     ));
                // }
            }
        }
    }

    fn fetch_search_results(&self) {
        match &self.model.filter {
            None => self
                .model
                .sender
                .send(SearchResult {
                    projects: vec![],
                    project_notes: vec![],
                    project_pois: vec![],
                    servers: vec![],
                    server_databases: vec![],
                    server_extra_users: vec![],
                    server_links: vec![],
                    server_notes: vec![],
                    server_pois: vec![],
                    server_websites: vec![],
                })
                .unwrap(),
            Some(filter) => {
                let s = self.model.sender.clone();
                let f = format!("%{}%", filter.replace('%', "\\%"));
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        // find all the leaves...
                        let servers = Self::filter_servers(sql_conn, &f);
                        let prjs = Self::filter_projects(sql_conn, &f);
                        let project_pois = Self::filter_project_pois(sql_conn, &f);
                        let project_notes = Self::filter_project_notes(sql_conn, &f);
                        let server_notes = Self::filter_server_notes(sql_conn, &f);
                        let server_links = Self::filter_server_links(sql_conn, &f);
                        let server_pois = Self::filter_server_pois(sql_conn, &f);
                        let server_databases = Self::filter_server_databases(sql_conn, &f);
                        let server_extra_users = Self::filter_server_extra_users(sql_conn, &f);
                        let server_websites = Self::filter_server_websites(sql_conn, &f)
                            .into_iter()
                            .map(|p| p.0)
                            .collect::<Vec<_>>();

                        // bubble up to the toplevel...
                        let mut all_server_ids =
                            servers.iter().map(|s| s.id).collect::<HashSet<_>>();
                        all_server_ids.extend(server_websites.iter().map(|sw| sw.server_id));
                        all_server_ids.extend(server_notes.iter().map(|sn| sn.server_id));
                        all_server_ids.extend(server_links.iter().map(|sl| sl.linked_server_id));
                        all_server_ids.extend(server_extra_users.iter().map(|sl| sl.server_id));
                        all_server_ids.extend(server_pois.iter().map(|sl| sl.server_id));
                        all_server_ids.extend(server_databases.iter().map(|sl| sl.server_id));
                        let all_servers = Self::load_servers_by_id(sql_conn, &all_server_ids);

                        let mut all_project_ids = all_servers
                            .iter()
                            .map(|s| s.project_id)
                            .collect::<HashSet<_>>();
                        all_project_ids.extend(prjs.iter().map(|p| p.id));
                        all_project_ids.extend(project_pois.iter().map(|ppoi| ppoi.project_id));
                        all_project_ids.extend(project_notes.iter().map(|pn| pn.project_id));
                        let all_projects = Self::load_projects_by_id(sql_conn, &all_project_ids);
                        s.send(SearchResult {
                            projects: all_projects,
                            project_notes,
                            project_pois,
                            servers: all_servers,
                            server_notes,
                            server_links,
                            server_pois,
                            server_databases,
                            server_extra_users,
                            server_websites,
                        })
                        .unwrap();
                    }))
                    .unwrap();
            }
        }
    }

    fn load_projects_by_id(db_conn: &SqliteConnection, ids: &HashSet<i32>) -> Vec<Project> {
        use projectpadsql::schema::project::dsl::*;
        project
            .filter(id.eq_any(ids))
            .order(name.asc())
            .load::<Project>(db_conn)
            .unwrap()
    }

    fn load_servers_by_id(db_conn: &SqliteConnection, ids: &HashSet<i32>) -> Vec<Server> {
        use projectpadsql::schema::server::dsl::*;
        server
            .filter(id.eq_any(ids))
            .load::<Server>(db_conn)
            .unwrap()
    }

    fn filter_projects(db_conn: &SqliteConnection, filter: &str) -> Vec<Project> {
        use projectpadsql::schema::project::dsl::*;
        project
            .filter(name.like(filter).escape('\\'))
            .load::<Project>(db_conn)
            .unwrap()
    }

    fn filter_project_pois(
        db_conn: &SqliteConnection,
        filter: &str,
    ) -> Vec<ProjectPointOfInterest> {
        use projectpadsql::schema::project_point_of_interest::dsl::*;
        project_point_of_interest
            .filter(
                desc.like(filter)
                    .escape('\\')
                    .or(text.like(filter).escape('\\'))
                    .or(path.like(filter).escape('\\')),
            )
            .load::<ProjectPointOfInterest>(db_conn)
            .unwrap()
    }

    fn filter_project_notes(db_conn: &SqliteConnection, filter: &str) -> Vec<ProjectNote> {
        use projectpadsql::schema::project_note::dsl::*;
        project_note
            .filter(
                title
                    .like(filter)
                    .escape('\\')
                    .or(contents.like(filter).escape('\\')),
            )
            .load::<ProjectNote>(db_conn)
            .unwrap()
    }

    fn filter_server_notes(db_conn: &SqliteConnection, filter: &str) -> Vec<ServerNote> {
        use projectpadsql::schema::server_note::dsl::*;
        server_note
            .filter(
                title
                    .like(filter)
                    .escape('\\')
                    .or(contents.like(filter).escape('\\')),
            )
            .load::<ServerNote>(db_conn)
            .unwrap()
    }

    fn filter_server_links(db_conn: &SqliteConnection, filter: &str) -> Vec<ServerLink> {
        use projectpadsql::schema::server_link::dsl::*;
        server_link
            .filter(desc.like(filter).escape('\\'))
            .load::<ServerLink>(db_conn)
            .unwrap()
    }

    fn filter_server_extra_users(
        db_conn: &SqliteConnection,
        filter: &str,
    ) -> Vec<ServerExtraUserAccount> {
        use projectpadsql::schema::server_extra_user_account::dsl::*;
        server_extra_user_account
            .filter(desc.like(filter).escape('\\'))
            .load::<ServerExtraUserAccount>(db_conn)
            .unwrap()
    }

    fn filter_server_pois(db_conn: &SqliteConnection, filter: &str) -> Vec<ServerPointOfInterest> {
        use projectpadsql::schema::server_point_of_interest::dsl::*;
        server_point_of_interest
            .filter(
                desc.like(filter)
                    .escape('\\')
                    .or(path.like(filter).escape('\\'))
                    .or(text.like(filter).escape('\\')),
            )
            .load::<ServerPointOfInterest>(db_conn)
            .unwrap()
    }

    fn filter_server_databases(db_conn: &SqliteConnection, filter: &str) -> Vec<ServerDatabase> {
        use projectpadsql::schema::server_database::dsl::*;
        server_database
            .filter(
                desc.like(filter)
                    .escape('\\')
                    .or(name.like(filter).escape('\\'))
                    .or(text.like(filter).escape('\\')),
            )
            .load::<ServerDatabase>(db_conn)
            .unwrap()
    }

    fn filter_servers(db_conn: &SqliteConnection, filter: &str) -> Vec<Server> {
        use projectpadsql::schema::server::dsl::*;
        server
            .filter(
                desc.like(filter)
                    .escape('\\')
                    .or(ip.like(filter).escape('\\'))
                    .or(text.like(filter).escape('\\')),
            )
            .load::<Server>(db_conn)
            .unwrap()
    }

    fn filter_server_websites(
        db_conn: &SqliteConnection,
        filter: &str,
    ) -> Vec<(ServerWebsite, Option<ServerDatabase>)> {
        use projectpadsql::schema::server_database::dsl as db;
        use projectpadsql::schema::server_website::dsl::*;
        server_website
            .left_outer_join(db::server_database)
            .filter(
                desc.like(filter)
                    .escape('\\')
                    .or(url.like(filter).escape('\\'))
                    .or(text.like(filter).escape('\\'))
                    .or(db::desc.like(filter).escape('\\'))
                    .or(db::name.like(filter).escape('\\')),
            )
            .load::<(ServerWebsite, Option<ServerDatabase>)>(db_conn)
            .unwrap()
    }

    view! {
        gtk::ScrolledWindow {
            #[name="search_result_treeview"]
            gtk::TreeView {
            }
        }
    }
}
