use std::collections::HashSet;

use glib::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use projectpadsql::models::{EnvironmentType, Project};
use strum_macros::FromRepr;

#[derive(FromRepr, Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum ProjectItemType {
    Server = 1,
    ServerLink = 2,
    ProjectNote = 3,
    ProjectPointOfInterest = 4,
}

impl ProjectItemType {
    pub fn get_icon(&self) -> &'static str {
        match self {
            ProjectItemType::Server => "server",
            ProjectItemType::ServerLink => "link",
            ProjectItemType::ProjectNote => "clipboard",
            ProjectItemType::ProjectPointOfInterest => "cube",
        }
    }
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
        icon: Rc<RefCell<String>>,
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
        project: &Project,
        id: i32,
        project_item_type: ProjectItemType,
        title: String,
        environments: HashSet<EnvironmentType>,
        group_name: Option<String>,
    ) -> Self {
        let has_all_envs = Self::project_get_envs(project).is_subset(&environments);
        let icon = project_item_type.get_icon();
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
            .property("icon", icon)
            .build()
    }

    fn project_get_envs(project: &Project) -> HashSet<EnvironmentType> {
        let mut env_set = HashSet::new();
        if project.has_uat {
            env_set.insert(EnvironmentType::EnvUat);
        }
        if project.has_dev {
            env_set.insert(EnvironmentType::EnvDevelopment);
        }
        if project.has_stage {
            env_set.insert(EnvironmentType::EnvStage);
        }
        if project.has_prod {
            env_set.insert(EnvironmentType::EnvProd);
        }
        env_set
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
