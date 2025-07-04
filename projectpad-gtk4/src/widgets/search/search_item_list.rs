use glib::*;
use gtk::prelude::Cast;
use gtk::prelude::ObjectExt;
use gtk::prelude::StaticType;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use itertools::Itertools;
use projectpadsql::models::EnvironmentType;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase, ServerExtraUserAccount,
    ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};

use crate::search_engine::MatchConfidence;
use crate::search_engine::SearchResult;
use crate::widgets::project_items::project_poi;
use crate::widgets::project_items::server;

use super::search_item_list_model::SearchItemListModel;
use super::search_item_model::{SearchItemModel, SearchItemType};

mod imp {
    use std::{cell::RefCell, rc::Rc, sync::OnceLock};

    use glib::subclass::Signal;
    use gtk::{
        subclass::{
            prelude::{BoxImpl, ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    use super::*;

    #[derive(Properties, Debug, Default, CompositeTemplate)]
    #[properties(wrapper_type = super::SearchItemList)]
    #[template(
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/search/search_item_list.ui"
    )]
    pub struct SearchItemList {
        #[template_child]
        pub search_item_list: TemplateChild<gtk::ListView>,

        #[property(get, set)]
        single_click_activate: Rc<RefCell<bool>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchItemList {
        const NAME: &'static str = "SearchItemList";
        type ParentType = gtk::Box;
        type Type = super::SearchItemList;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SearchItemList {
        fn constructed(&self) {
            self.obj().init_list();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("select-item")
                        // project id + item id + search_item_type + optionally server item id
                        .param_types([
                            i32::static_type(),
                            i32::static_type(),
                            u8::static_type(),
                            i32::static_type(),
                        ])
                        .build(),
                    Signal::builder("activate-item")
                        // project id + item id + search_item_type + optionally server item id
                        .param_types([
                            i32::static_type(),
                            i32::static_type(),
                            u8::static_type(),
                            i32::static_type(),
                        ])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for SearchItemList {}

    impl BoxImpl for SearchItemList {}
}

glib::wrapper! {
    pub struct SearchItemList(ObjectSubclass<imp::SearchItemList>)
        @extends gtk::Widget, gtk::Box;
}

impl SearchItemList {
    pub fn init_list(&self) {
        self.bind_property(
            "single-click-activate",
            &self.imp().search_item_list.get(),
            "single-click-activate",
        )
        .sync_create()
        .build();

        self.imp()
            .search_item_list
            .set_factory(Some(&gtk::BuilderListItemFactory::from_resource(
                Some(&gtk::BuilderRustScope::new()),
                "/com/github/emmanueltouzery/projectpad2/src/widgets/search/search_item_row.ui",
            )));
        self.imp()
            .search_item_list
            .set_header_factory(Some(&gtk::BuilderListItemFactory::from_resource(
            Some(&gtk::BuilderRustScope::new()),
            "/com/github/emmanueltouzery/projectpad2/src/widgets/search/search_item_header_row.ui",
        )));
        self.imp().search_item_list.connect_activate(clone!(
            #[strong(rename_to = this)]
            self,
            move |list, item_idx| {
                let gtk_model: gtk::SingleSelection = list.model().unwrap().downcast().unwrap();
                let model: SearchItemListModel = gtk_model.model().unwrap().downcast().unwrap();
                let (project_id, item_id, item_type, sub_id) =
                    model.get_search_item(item_idx).unwrap();
                this.emit_by_name::<()>(
                    "activate-item",
                    &[&project_id, &item_id, &item_type, &sub_id],
                );
            }
        ));
    }

    pub fn set_search_items(
        &mut self,
        search_result: SearchResult,
        selection: Option<(SearchItemType, i32)>,
    ) {
        let mut list_store = SearchItemListModel::new();
        for project in &search_result.projects {
            list_store.append(&Self::get_project_model(project));

            let mut search_item_models_and_conf = vec![];
            self.project_display_servers(
                project,
                &search_result.servers,
                &mut search_item_models_and_conf,
                &search_result,
            );
            for server_link in search_result
                .server_links
                .iter()
                .filter(|s| s.project_id == project.id)
            {
                search_item_models_and_conf.push((
                    Self::get_server_link_model(server_link, project),
                    MatchConfidence::Normal,
                    vec![],
                ));
            }
            for (project_note, conf) in search_result
                .project_notes
                .iter()
                .filter(|(s, _c)| s.project_id == project.id)
            {
                search_item_models_and_conf.push((
                    Self::get_project_note_model(project_note, project),
                    *conf,
                    vec![],
                ));
            }
            for project_poi in search_result
                .project_pois
                .iter()
                .filter(|s| s.project_id == project.id)
            {
                search_item_models_and_conf.push((
                    Self::get_project_poi_model(project_poi, project),
                    MatchConfidence::Normal,
                    vec![],
                ));
            }
            // sort by confidence then display
            // dbg!(&search_item_models_and_conf);
            search_item_models_and_conf.sort_by_key(|(_sim, c, _children)| std::cmp::Reverse(*c));
            // dbg!(&search_item_models_and_conf);
            for (sim, _conf, children) in search_item_models_and_conf.iter() {
                list_store.append(sim);
                for child in children.iter() {
                    list_store.append(child);
                }
            }
        }

        // list_store.set_group_start_indices(search_items.len(), group_start_indices);
        // for search_item in search_items {
        //     list_store.append(&Self::get_item_model(search_item));
        // }
        let selection_model = gtk::SingleSelection::new(Some(list_store.clone()));

        let mut selection_idx = 0;
        if let Some((item_type, item_id)) = selection {
            if let Some(index) = list_store.get_index(item_type, item_id) {
                selection_idx = index;
            }
        }

        let s = self.clone();
        selection_model.connect_selected_notify(move |sel| {
            if let Some(selected) = sel.selected_item() {
                let search_item_list_model = selected.downcast::<SearchItemModel>().unwrap();
                let project_id = search_item_list_model.project_id();
                let item_id = search_item_list_model.id();
                let item_type = search_item_list_model.search_item_type();
                let sub_id = search_item_list_model.server_id();
                s.emit_by_name::<()>("select-item", &[&project_id, &item_id, &item_type, &sub_id]);
            }
        });
        self.imp()
            .search_item_list
            .set_model(Some(&selection_model));
        if selection_idx == 0 {
            if let Some((project_id, item_id, item_type, sub_id)) = list_store.get_search_item(0) {
                self.emit_by_name::<()>(
                    "select-item",
                    &[&project_id, &item_id, &item_type, &sub_id],
                );
            }
        } else {
            let s = self.clone();
            glib::idle_add_local(move || {
                s.imp().search_item_list.scroll_to(
                    selection_idx,
                    gtk::ListScrollFlags::SELECT,
                    None,
                );
                ControlFlow::Break
            });
        }
    }

    fn project_display_servers(
        &mut self,
        project: &Project,
        servers: &[(Server, MatchConfidence)],
        parent_search_item_models_and_conf: &mut Vec<(
            SearchItemModel,
            MatchConfidence,
            Vec<SearchItemModel>,
        )>,
        search_result: &SearchResult,
    ) {
        for (server, server_confidence) in
            servers.iter().filter(|(s, _c)| s.project_id == project.id)
        {
            let mut server_search_item_models_and_conf = vec![];
            let server_model = Self::get_server_model(server, project);
            for server_website in search_result
                .server_websites
                .iter()
                .filter(|sw| sw.server_id == server.id)
            {
                server_search_item_models_and_conf.push((
                    Self::get_server_website_model(server_website, project),
                    MatchConfidence::Normal,
                ));
            }
            for (server_note, conf) in search_result
                .server_notes
                .iter()
                .filter(|(sn, _c)| sn.server_id == server.id)
            {
                server_search_item_models_and_conf
                    .push((Self::get_server_note_model(server_note, project), *conf));
            }
            for server_user in search_result
                .server_extra_users
                .iter()
                .filter(|su| su.server_id == server.id)
            {
                server_search_item_models_and_conf.push((
                    Self::get_server_extra_user_account_model(server_user, project),
                    MatchConfidence::Normal,
                ));
            }
            for server_db in search_result
                .server_databases
                .iter()
                .filter(|sd| sd.server_id == server.id)
            {
                server_search_item_models_and_conf.push((
                    Self::get_server_database_model(server_db, project),
                    MatchConfidence::Normal,
                ));
            }
            for server_poi in search_result
                .server_pois
                .iter()
                .filter(|sp| sp.server_id == server.id)
            {
                server_search_item_models_and_conf.push((
                    Self::get_server_poi_model(server_poi, project),
                    MatchConfidence::Normal,
                ));
            }
            // sort by confidence then display
            server_search_item_models_and_conf.sort_by_key(|(_sim, c)| std::cmp::Reverse(*c));
            parent_search_item_models_and_conf.push((
                server_model,
                *server_confidence,
                server_search_item_models_and_conf
                    .into_iter()
                    .map(|(sim, _c)| sim)
                    .collect_vec(),
            ));
        }
    }

    pub fn displayed_items(&self) -> gtk::SingleSelection {
        self.imp()
            .search_item_list
            .model()
            .unwrap()
            .downcast::<gtk::SingleSelection>()
            .unwrap()
    }

    fn get_project_model(project: &Project) -> SearchItemModel {
        SearchItemModel::new(
            project.id,
            None,
            project.id,
            SearchItemType::Project,
            project.name.clone(),
            // there'll always a project item under, nevermind all the possible envs of the project
            None,
            None,
            None,
        )
    }

    fn get_server_model(server: &Server, project: &Project) -> SearchItemModel {
        SearchItemModel::new(
            server.id,
            None,
            project.id,
            SearchItemType::Server,
            server.desc.clone(),
            Some(server.environment),
            Some(project.name.to_owned()),
            Some(server::custom_icon(server)),
        )
    }

    fn get_server_website_model(item: &ServerWebsite, project: &Project) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            Some(item.server_id),
            project.id,
            SearchItemType::ServerWebsite,
            item.desc.clone(),
            None,
            Some(project.name.to_owned()),
            None,
        )
    }

    fn get_server_note_model(item: &ServerNote, project: &Project) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            Some(item.server_id),
            project.id,
            SearchItemType::ServerNote,
            item.title.clone(),
            None,
            Some(project.name.to_owned()),
            None,
        )
    }

    fn get_server_extra_user_account_model(
        item: &ServerExtraUserAccount,
        project: &Project,
    ) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            Some(item.server_id),
            project.id,
            SearchItemType::ServerExtraUserAccount,
            item.desc.clone(),
            None,
            Some(project.name.to_owned()),
            None,
        )
    }

    fn get_server_database_model(item: &ServerDatabase, project: &Project) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            Some(item.server_id),
            project.id,
            SearchItemType::ServerDatabase,
            item.desc.clone(),
            None,
            Some(project.name.to_owned()),
            None,
        )
    }

    fn get_server_poi_model(item: &ServerPointOfInterest, project: &Project) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            Some(item.server_id),
            project.id,
            SearchItemType::ServerPoi,
            item.desc.clone(),
            None,
            Some(project.name.to_owned()),
            Some(server::server_poi_custom_icon(item)),
        )
    }

    fn get_server_link_model(item: &ServerLink, project: &Project) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            None, // TODO is that correct?
            project.id,
            SearchItemType::ServerLink,
            item.desc.clone(),
            None,
            Some(project.name.to_owned()),
            None,
        )
    }

    fn get_project_note_model(item: &ProjectNote, project: &Project) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            None,
            project.id,
            SearchItemType::ProjectNote,
            item.title.clone(),
            Some(if item.has_prod {
                EnvironmentType::EnvProd
            } else if item.has_uat {
                EnvironmentType::EnvUat
            } else if item.has_stage {
                EnvironmentType::EnvStage
            } else {
                EnvironmentType::EnvDevelopment
            }),
            Some(project.name.to_owned()),
            None,
        )
    }

    fn get_project_poi_model(item: &ProjectPointOfInterest, project: &Project) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            None,
            project.id,
            SearchItemType::ProjectPointOfInterest,
            item.desc.clone(),
            None,
            Some(project.name.to_owned()),
            Some(project_poi::custom_icon(item)),
        )
    }
}
