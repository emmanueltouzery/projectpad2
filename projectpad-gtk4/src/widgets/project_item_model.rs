use std::collections::HashSet;

use glib::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use projectpadsql::models::EnvironmentType;
use strum_macros::FromRepr;

#[derive(FromRepr, Debug, PartialEq)]
#[repr(u8)]
pub enum ProjectItemType {
    Server = 1,
    ServerLink = 2,
    ProjectNote = 3,
    ProjectPointOfInterest = 4,
}

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ProjectItemModel)]
    pub struct ProjectItemModel {
        #[property(get, set)]
        id: Rc<RefCell<i32>>,
        #[property(get, set)]
        title: Rc<RefCell<String>>,
        #[property(get, set)]
        has_dev: Rc<RefCell<bool>>,
        #[property(get, set)]
        has_stg: Rc<RefCell<bool>>,
        #[property(get, set)]
        has_uat: Rc<RefCell<bool>>,
        #[property(get, set)]
        has_prod: Rc<RefCell<bool>>,
        #[property(get, set)]
        group_name: Rc<RefCell<String>>,
        #[property(get, set)]
        project_item_type: Rc<RefCell<u8>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectItemModel {
        const NAME: &'static str = "ProjectItemModel";
        // type ParentType = glib::Object;
        type Type = super::ProjectItemModel;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectItemModel {}
}

glib::wrapper! {
    pub struct ProjectItemModel(ObjectSubclass<imp::ProjectItemModel>);
}

impl ProjectItemModel {
    pub fn new(
        id: i32,
        project_item_type: ProjectItemType,
        title: String,
        environments: HashSet<EnvironmentType>,
        group_name: Option<String>,
    ) -> Self {
        let has_all_envs = environments.len() == 4;
        Object::builder()
            .property("id", id)
            .property("project-item-type", project_item_type as u8)
            .property("title", title)
            .property(
                "has-dev",
                !has_all_envs && environments.contains(&EnvironmentType::EnvDevelopment),
            )
            .property(
                "has-stg",
                !has_all_envs && environments.contains(&EnvironmentType::EnvStage),
            )
            .property(
                "has-uat",
                !has_all_envs && environments.contains(&EnvironmentType::EnvUat),
            )
            .property(
                "has-prod",
                !has_all_envs && environments.contains(&EnvironmentType::EnvProd),
            )
            .property("group-name", group_name.unwrap_or("".to_string()))
            .build()
    }
}

// mod imp {
//     use super::*;

//     #[derive(Debug, Default)]
//     pub struct ProjectItemModel {
//         pub title: String,
//     }

//     #[glib::object_subclass]
//     impl ObjectSubclass for ProjectItemModel {
//         const NAME: &'static str = "ProjectItemModel";
//         type ParentType = glib::Object;
//         type Type = super::ProjectItemModel;
//     }

//     impl ObjectImpl for ProjectItemModel {}
// }

// glib::wrapper! {
//     pub struct ProjectItemModel(ObjectSubclass<imp::ProjectItemModel>);
// }
