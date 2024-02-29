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
        env_desc: Rc<RefCell<String>>,
        #[property(get, set)]
        env_classes: Rc<RefCell<Vec<String>>>,
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

impl ProjectItemModel {
    pub fn new(
        id: i32,
        project_item_type: ProjectItemType,
        title: String,
        environment: EnvironmentType,
        group_name: Option<String>,
    ) -> Self {
        Object::builder()
            .property("id", id)
            .property("project-item-type", project_item_type as u8)
            .property("title", title)
            .property("env-desc", env_to_desc(&environment))
            .property("env-classes", env_to_css(&environment))
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
