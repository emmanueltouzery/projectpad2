use glib::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use strum_macros::FromRepr;

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

pub enum Env {
    Dev,
    Staging,
    Uat,
    Prod,
}

fn env_to_css(val: &Env) -> Vec<String> {
    vec![
        match val {
            Env::Dev => "project-item-dev",
            Env::Staging => "project-item-staging",
            Env::Uat => "project-item-uat",
            Env::Prod => "project-item-prod",
        }
        .to_string(),
        "caption-heading".to_string(),
    ]
}

fn env_to_desc(val: &Env) -> String {
    match val {
        Env::Dev => "DEV",
        Env::Staging => "STG",
        Env::Uat => "UAT",
        Env::Prod => "PRD",
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
        environment: Env,
        group_name: Option<String>,
    ) -> Self {
        let icon = match search_item_type {
            SearchItemType::Server => "server",
            SearchItemType::ServerLink => "share-square",
            SearchItemType::ProjectNote => "clipboard",
            SearchItemType::ProjectPointOfInterest => "cube",
            SearchItemType::Project => "cubes",
            SearchItemType::ServerWebsite => "globe",
            SearchItemType::ServerNote => "clipboard",
            SearchItemType::ServerDatabase => "database",
            SearchItemType::ServerPoi => "cube",
        };
        Object::builder()
            .property("id", id)
            .property("project-id", project_id)
            .property("server-id", server_id.unwrap_or(-1))
            .property("search-item-type", search_item_type as u8)
            .property("title", title)
            .property("icon", icon)
            .property("env-desc", env_to_desc(&environment))
            .property("env-classes", env_to_css(&environment))
            .property("group-name", group_name.unwrap_or("".to_string()))
            .build()
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
