use glib::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use projectpadsql::models::EnvironmentType;
use strum_macros::FromRepr;

use crate::widgets::project_item_model::ProjectItemType;

#[derive(FromRepr, Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum SearchItemType {
    Server = 1,
    ServerLink = 2,
    ProjectNote = 3,
    ProjectPointOfInterest = 4,
    Project = 5,
    ServerWebsite = 6,
    ServerNote = 7,
    ServerDatabase = 8,
    ServerPoi = 9,
    ServerExtraUserAccount = 10,
}

impl SearchItemType {
    pub fn to_project_item_type(&self) -> Option<ProjectItemType> {
        match self {
            SearchItemType::Server => Some(ProjectItemType::Server),
            SearchItemType::ServerLink => Some(ProjectItemType::ServerLink),
            SearchItemType::ProjectNote => Some(ProjectItemType::ProjectNote),
            SearchItemType::ProjectPointOfInterest => Some(ProjectItemType::ProjectPointOfInterest),
            _ => None,
        }
    }
}

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::SearchItemModel)]
    pub struct SearchItemModel {
        #[property(get, set)]
        id: Rc<RefCell<i32>>,
        #[property(get, set)]
        project_id: Rc<RefCell<i32>>,
        #[property(get, set)]
        server_id: Rc<RefCell<i32>>,
        #[property(get, set)]
        title: Rc<RefCell<String>>,
        #[property(get, set)]
        icon: Rc<RefCell<String>>,
        #[property(get, set)]
        env_desc: Rc<RefCell<String>>,
        #[property(get, set)]
        env_classes: Rc<RefCell<Vec<String>>>,
        #[property(get, set)]
        group_name: Rc<RefCell<String>>,
        #[property(get, set)]
        search_item_type: Rc<RefCell<u8>>,
        #[property(get, set)]
        is_server_item: Rc<RefCell<bool>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchItemModel {
        const NAME: &'static str = "SearchItemModel";
        // type ParentType = glib::Object;
        type Type = super::SearchItemModel;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SearchItemModel {}
}

glib::wrapper! {
    pub struct SearchItemModel(ObjectSubclass<imp::SearchItemModel>);
}

fn env_to_css(val: &EnvironmentType) -> Vec<String> {
    vec![
        match val {
            EnvironmentType::EnvDevelopment => "project-item-dev",
            EnvironmentType::EnvStage => "project-item-staging",
            EnvironmentType::EnvUat => "project-item-uat",
            EnvironmentType::EnvProd => "project-item-prod",
        }
        .to_string(),
        "caption-heading".to_string(),
    ]
}

fn env_to_desc(val: &EnvironmentType) -> String {
    match val {
        EnvironmentType::EnvDevelopment => "DEV",
        EnvironmentType::EnvStage => "STG",
        EnvironmentType::EnvUat => "UAT",
        EnvironmentType::EnvProd => "PRD",
    }
    .to_string()
}

impl SearchItemModel {
    pub fn new(
        id: i32,
        server_id: Option<i32>,
        project_id: i32,
        search_item_type: SearchItemType,
        title: String,
        environment: Option<EnvironmentType>,
        group_name: Option<String>,
        custom_icon: Option<&'static str>,
    ) -> Self {
        let search_item_type_icon = Self::get_search_item_type_icon(search_item_type);
        Object::builder()
            .property("id", id)
            .property("project-id", project_id)
            .property("server-id", server_id.unwrap_or(-1))
            .property("search-item-type", search_item_type as u8)
            .property("title", title)
            .property("icon", custom_icon.unwrap_or(search_item_type_icon))
            .property(
                "env-desc",
                &match &environment {
                    Some(e) => env_to_desc(e),
                    _ => "".to_owned(),
                },
            )
            .property(
                "env-classes",
                &match &environment {
                    Some(e) => env_to_css(e),
                    _ => vec![],
                },
            )
            .property("group-name", group_name.unwrap_or("".to_string()))
            .property(
                "is-server-item",
                match search_item_type {
                    SearchItemType::ServerPoi => true,
                    SearchItemType::ServerNote => true,
                    SearchItemType::ServerWebsite => true,
                    SearchItemType::ServerExtraUserAccount => true,
                    SearchItemType::ServerDatabase => true,
                    _ => false,
                },
            )
            .build()
    }

    pub fn get_search_item_type_icon(search_item_type: SearchItemType) -> &'static str {
        match search_item_type {
            SearchItemType::Server => "server",
            SearchItemType::ServerLink => "share-square",
            SearchItemType::ProjectNote => "clipboard",
            SearchItemType::ProjectPointOfInterest => "cube",
            SearchItemType::Project => "cubes",
            SearchItemType::ServerWebsite => "globe",
            SearchItemType::ServerNote => "clipboard",
            SearchItemType::ServerDatabase => "database",
            SearchItemType::ServerPoi => "cube",
            SearchItemType::ServerExtraUserAccount => "user",
        }
    }
}

// mod imp {
//     use super::*;

//     #[derive(Debug, Default)]
//     pub struct SearchItemModel {
//         pub title: String,
//     }

//     #[glib::object_subclass]
//     impl ObjectSubclass for SearchItemModel {
//         const NAME: &'static str = "SearchItemModel";
//         type ParentType = glib::Object;
//         type Type = super::SearchItemModel;
//     }

//     impl ObjectImpl for SearchItemModel {}
// }

// glib::wrapper! {
//     pub struct SearchItemModel(ObjectSubclass<imp::SearchItemModel>);
// }
