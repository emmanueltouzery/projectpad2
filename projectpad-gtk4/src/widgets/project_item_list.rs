use std::{
    collections::{BTreeSet, HashMap},
    sync::mpsc,
};

use diesel::prelude::*;
use glib::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use itertools::Itertools;
use projectpadsql::models::{
    EnvironmentType, ProjectNote, ProjectPointOfInterest, Server, ServerLink,
};

use crate::sql_thread::SqlFunc;

use super::{
    project_item_list_model::ProjectItemListModel,
    project_item_model::{ProjectItemModel, ProjectItemType},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ProjectItem {
    Server(Server),
    ServerLink(ServerLink),
    ProjectNote(ProjectNote),
    ProjectPointOfInterest(ProjectPointOfInterest),
}

// https://gtk-rs.org/gtk4-rs/stable/latest/book/todo_1.html
// https://gitlab.com/news-flash/news_flash_gtk/-/blob/master/src/article_list/models/article.rs?ref_type=heads
mod imp {
    use std::sync::OnceLock;

    use glib::subclass::Signal;
    use gtk::{
        subclass::{
            prelude::{BoxImpl, ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_item_list.ui"
    )]
    pub struct ProjectItemList {
        #[template_child]
        pub project_item_list: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectItemList {
        const NAME: &'static str = "ProjectItemList";
        type ParentType = gtk::Box;
        type Type = super::ProjectItemList;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProjectItemList {
        fn constructed(&self) {
            self.obj().init_list();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("activate-item")
                    // item id + project_item_type
                    .param_types([i32::static_type(), u8::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for ProjectItemList {}

    impl BoxImpl for ProjectItemList {}
}

glib::wrapper! {
    pub struct ProjectItemList(ObjectSubclass<imp::ProjectItemList>)
        @extends gtk::Widget, gtk::Box;
}

impl ProjectItemList {
    pub fn init_list(&self) {
        self.imp().project_item_list.set_factory(Some(
            &gtk::BuilderListItemFactory::from_resource(
                Some(&gtk::BuilderRustScope::new()),
                "/com/github/emmanueltouzery/projectpad2/src/widgets/project_item_row.ui",
            ),
        ));
        self.imp().project_item_list.set_header_factory(Some(
            &gtk::BuilderListItemFactory::from_resource(
                Some(&gtk::BuilderRustScope::new()),
                "/com/github/emmanueltouzery/projectpad2/src/widgets/project_item_header_row.ui",
            ),
        ));
    }

    pub fn set_project_items(
        &self,
        project_items: &[ProjectItem],
        group_start_indices: HashMap<i32, String>,
        selected_item: Option<i32>,
    ) {
        let mut list_store = ProjectItemListModel::new();
        list_store.set_group_start_indices(project_items.len(), group_start_indices);
        let mut idx = 0;
        let mut selected_index = None;

        for project_item in project_items {
            let item_model = Self::get_item_model(project_item);
            list_store.append(&item_model);
            if selected_item == Some(item_model.property("id")) {
                selected_index = Some(idx);
            }
            idx += 1;
        }
        if selected_item.is_none() && !project_items.is_empty() {
            // None == select first item (if any)
            selected_index = Some(0);

            if let Some(first_item) = list_store.item(0) {
                self.emit_by_name::<()>(
                    "activate-item",
                    &[
                        &first_item.property_value("id"),
                        &first_item.property_value("project-item-type"),
                    ],
                );
            }
        }
        if let Some(s_model) = self.imp().project_item_list.model() {
            let _sel_model = s_model.downcast::<gtk::SingleSelection>().unwrap();
            _sel_model.set_model(Some(&list_store));
        } else {
            let selection_model = gtk::SingleSelection::new(Some(list_store));
            self.imp()
                .project_item_list
                .set_model(Some(&selection_model));

            self.imp()
            .project_item_list
            .model()
            .unwrap()
            .connect_selection_changed(glib::clone!(@weak self as s => move |sel_model, _idx, _items_count| {
                let idx = sel_model.downcast_ref::<gtk::SingleSelection>().unwrap().selected();
                let model = sel_model
                    .item(idx)
                    .unwrap();
                s.emit_by_name::<()>("activate-item", &[
                    &model.property_value("id"),
                    &model.property_value("project-item-type"),
                ])
            }));
        }
        if let Some(idx) = selected_index {
            self.imp()
                .project_item_list
                .scroll_to(idx, gtk::ListScrollFlags::SELECT, None);
        }
    }

    fn get_item_model(project_item: &ProjectItem) -> ProjectItemModel {
        match project_item {
            ProjectItem::Server(srv) => ProjectItemModel::new(
                srv.id,
                ProjectItemType::Server,
                srv.desc.clone(),
                srv.environment,
                srv.group_name.clone()
            ),
            //     markup: if srv.is_retired {
            //         format!("<i>{}</i>", glib::markup_escape_text(&srv.desc))
            //     } else {
            //         glib::markup_escape_text(&srv.desc).to_string()
            //     },
            //     group_name: srv.group_name.as_ref().cloned(),
            //     icon: match (srv.server_type, srv.access_type) {
            //         (ServerType::SrvDatabase, _) => Icon::DATABASE,
            //         (ServerType::SrvReporting, _) => Icon::REPORTING,
            //         (ServerType::SrvMonitoring, _) => Icon::MONITORING,
            //         (ServerType::SrvHttpOrProxy, _) => Icon::HTTP,
            //         (_, ServerAccessType::SrvAccessRdp) => Icon::WINDOWS,
            //         (_, _) => Icon::SERVER,
            //     },
            // },
            ProjectItem::ServerLink(link) => ProjectItemModel::new(
                link.id,
                ProjectItemType::ServerLink,
                link.desc.clone(),
                link.environment,
                link.group_name.clone()
            ),
            //     markup: glib::markup_escape_text(&link.desc).to_string(),
            //     group_name: link.group_name.as_ref().cloned(),
            //     icon: Icon::SERVER_LINK,
            // },
            ProjectItem::ProjectNote(note) => ProjectItemModel::new(
                note.id,
                ProjectItemType::ProjectNote,
                note.title.clone(),
                EnvironmentType::EnvProd, // TODO has_prod, has...
                note.group_name.clone()
            ),
            //     markup: glib::markup_escape_text(&note.title).to_string(),
            //     group_name: note.group_name.as_ref().cloned(),
            //     icon: Icon::NOTE,
            // },
            ProjectItem::ProjectPointOfInterest(poi) => ProjectItemModel::new(
                poi.id, ProjectItemType::ProjectPointOfInterest, poi.desc.clone(), EnvironmentType::EnvProd, poi.group_name.clone()) // TODO env
                // markup: glib::markup_escape_text(&poi.desc).to_string(),
                // group_name: poi.group_name.as_ref().cloned(),
                // icon: match poi.interest_type {
                //     InterestType::PoiLogFile => Icon::LOG_FILE,
                //     InterestType::PoiConfigFile => Icon::CONFIG_FILE,
                //     InterestType::PoiApplication => Icon::COG,
                //     InterestType::PoiCommandToRun => Icon::TERMINAL,
                //     InterestType::PoiCommandTerminal => Icon::TERMINAL,
                //     InterestType::PoiBackupArchive => Icon::ARCHIVE,
                // },
            }
    }

    pub fn fetch_project_items(
        &mut self,
        db_sender: &mpsc::Sender<SqlFunc>,
        project_id: i32,
        selected_item: Option<i32>,
    ) {
        let (sender, receiver) = async_channel::bounded(1);
        db_sender
            .send(SqlFunc::new(move |sql_conn| {
                let (servers, lsrvs, prj_notes, prj_pois) =
                    Self::fetch_project_items_sql(sql_conn, Some(project_id));

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
                let mut group_start_indices = HashMap::new();
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
                    group_start_indices.insert(items.len() as i32, group_name.clone());
                    Self::add_items(
                        &mut items,
                        &mut servers_iter,
                        &mut lsrvs_iter,
                        &mut prj_notes_iter,
                        &mut prj_pois_iter,
                        Some(group_name),
                    );
                }
                sender.send_blocking((items, group_start_indices)).unwrap();
            }))
            .unwrap();
        let mut s = self.clone();
        glib::spawn_future_local(async move {
            let (items, group_start_indices) = receiver.recv().await.unwrap();
            s.set_project_items(&items, group_start_indices, selected_item);
        });
    }

    fn add_items(
        items: &mut Vec<ProjectItem>,
        servers: &mut (impl Iterator<Item = Server> + Clone),
        lsrvs: &mut (impl Iterator<Item = ServerLink> + Clone),
        prj_notes: &mut (impl Iterator<Item = ProjectNote> + Clone),
        prj_pois: &mut (impl Iterator<Item = ProjectPointOfInterest> + Clone),
        group_name: Option<String>,
    ) {
        items.extend(
            servers
                .take_while_ref(|s| s.group_name == group_name)
                .map(ProjectItem::Server),
        );
        items.extend(
            lsrvs
                .take_while_ref(|s| s.group_name == group_name)
                .map(ProjectItem::ServerLink),
        );
        items.extend(
            prj_notes
                .take_while_ref(|s| s.group_name == group_name)
                .map(ProjectItem::ProjectNote),
        );
        items.extend(
            prj_pois
                .take_while_ref(|s| s.group_name == group_name)
                .map(ProjectItem::ProjectPointOfInterest),
        );
    }

    fn fetch_project_items_sql(
        sql_conn: &mut diesel::SqliteConnection,
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
                    .filter(
                        srv::project_id.eq(pid), /*.and(srv::environment.eq(env))*/
                    )
                    .order((srv::group_name.asc(), srv::desc.asc()))
                    .load::<Server>(sql_conn)
                    .unwrap();
                let lsrvs = lsrv::server_link
                    .filter(
                        lsrv::project_id.eq(pid), /*.and(lsrv::environment.eq(env))*/
                    )
                    .order((lsrv::group_name.asc(), lsrv::desc.asc()))
                    .load::<ServerLink>(sql_conn)
                    .unwrap();
                let mut prj_query = pnt::project_note
                    .filter(pnt::project_id.eq(pid))
                    .into_boxed();
                // prj_query = match env {
                //     EnvironmentType::EnvProd => prj_query.filter(pnt::has_prod.eq(true)),
                //     EnvironmentType::EnvUat => prj_query.filter(pnt::has_uat.eq(true)),
                //     EnvironmentType::EnvStage => prj_query.filter(pnt::has_stage.eq(true)),
                //     EnvironmentType::EnvDevelopment => prj_query.filter(pnt::has_dev.eq(true)),
                // };
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
}
