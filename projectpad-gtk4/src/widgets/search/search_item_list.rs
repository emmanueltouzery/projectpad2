use std::collections::HashMap;

use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase, ServerExtraUserAccount,
    ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};

use crate::search_engine::SearchResult;

use super::search_item_list_model::SearchItemListModel;
use super::search_item_model::{Env, SearchItemModel, SearchItemType};

mod imp {
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
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/search/search_item_list.ui"
    )]
    pub struct SearchItemList {
        #[template_child]
        pub search_item_list: TemplateChild<gtk::ListView>,
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

    impl ObjectImpl for SearchItemList {
        fn constructed(&self) {
            self.obj().init_list();
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
        self.imp()
            .search_item_list
            .set_factory(Some(&gtk::BuilderListItemFactory::from_resource(
                Some(&gtk::BuilderRustScope::new()),
                "/com/github/emmanueltouzery/projectpad2/src/widgets/search/search_item_row.ui",
            )));
        // self.imp().search_item_list.set_header_factory(Some(
        //     &gtk::BuilderListItemFactory::from_resource(
        //         Some(&gtk::BuilderRustScope::new()),
        //         "/com/github/emmanueltouzery/projectpad2/src/widgets/search_item_header_row.ui",
        //     ),
        // ));
    }

    pub fn set_search_items(
        &mut self,
        search_result: SearchResult,
        group_start_indices: HashMap<i32, String>,
    ) {
        let mut list_store = SearchItemListModel::new();
        for project in &search_result.projects {
            list_store.append(&Self::get_project_model(project));
            for server in search_result
                .servers
                .iter()
                .filter(|s| s.project_id == project.id)
            {
                list_store.append(&Self::get_server_model(server));
                for server_website in search_result
                    .server_websites
                    .iter()
                    .filter(|sw| sw.server_id == server.id)
                {
                    list_store.append(&Self::get_server_website_model(server_website));
                }
                for server_note in search_result
                    .server_notes
                    .iter()
                    .filter(|sn| sn.server_id == server.id)
                {
                    list_store.append(&Self::get_server_note_model(server_note));
                }
                for server_user in search_result
                    .server_extra_users
                    .iter()
                    .filter(|su| su.server_id == server.id)
                {
                    list_store.append(&Self::get_server_extra_user_account_model(server_user));
                }
                for server_db in search_result
                    .server_databases
                    .iter()
                    .filter(|sd| sd.server_id == server.id)
                {
                    list_store.append(&Self::get_server_database_model(server_db));
                }
                for server_poi in search_result
                    .server_pois
                    .iter()
                    .filter(|sp| sp.server_id == server.id)
                {
                    list_store.append(&Self::get_server_poi_model(server_poi));
                }
            }
            for server_link in search_result
                .server_links
                .iter()
                .filter(|s| s.project_id == project.id)
            {
                list_store.append(&Self::get_server_link_model(server_link));
            }
            for project_note in search_result
                .project_notes
                .iter()
                .filter(|s| s.project_id == project.id)
            {
                list_store.append(&Self::get_project_note_model(project_note));
            }
            for project_poi in search_result
                .project_pois
                .iter()
                .filter(|s| s.project_id == project.id)
            {
                list_store.append(&Self::get_project_poi_model(project_poi));
            }
        }

        // list_store.set_group_start_indices(search_items.len(), group_start_indices);
        // for search_item in search_items {
        //     list_store.append(&Self::get_item_model(search_item));
        // }
        let selection_model = gtk::SingleSelection::new(Some(list_store));
        self.imp()
            .search_item_list
            .set_model(Some(&selection_model));
    }

    fn get_project_model(project: &Project) -> SearchItemModel {
        SearchItemModel::new(
            project.id,
            SearchItemType::Project,
            project.name.clone(),
            Env::Prod, // TODO
            None,
        )
    }

    fn get_server_model(server: &Server) -> SearchItemModel {
        SearchItemModel::new(
            server.id,
            SearchItemType::Server,
            server.desc.clone(),
            Env::Prod, // TODO
            None,
        )
    }

    fn get_server_website_model(item: &ServerWebsite) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            SearchItemType::ServerWebsite,
            item.desc.clone(),
            Env::Prod, // TODO
            None,
        )
    }

    fn get_server_note_model(item: &ServerNote) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            SearchItemType::ServerNote,
            item.title.clone(),
            Env::Prod, // TODO
            None,
        )
    }

    fn get_server_extra_user_account_model(item: &ServerExtraUserAccount) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            SearchItemType::ServerNote,
            item.desc.clone(),
            Env::Prod, // TODO
            None,
        )
    }

    fn get_server_database_model(item: &ServerDatabase) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            SearchItemType::ServerDatabase,
            item.desc.clone(),
            Env::Prod, // TODO
            None,
        )
    }

    fn get_server_poi_model(item: &ServerPointOfInterest) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            SearchItemType::ServerPoi,
            item.desc.clone(),
            Env::Prod, // TODO
            None,
        )
    }

    fn get_server_link_model(item: &ServerLink) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            SearchItemType::ServerLink,
            item.desc.clone(),
            Env::Prod, // TODO
            None,
        )
    }

    fn get_project_note_model(item: &ProjectNote) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            SearchItemType::ProjectNote,
            item.title.clone(),
            Env::Prod, // TODO
            None,
        )
    }

    fn get_project_poi_model(item: &ProjectPointOfInterest) -> SearchItemModel {
        SearchItemModel::new(
            item.id,
            SearchItemType::ProjectPointOfInterest,
            item.desc.clone(),
            Env::Prod, // TODO
            None,
        )
    }
}
