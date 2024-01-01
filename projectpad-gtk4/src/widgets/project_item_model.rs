use glib::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

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

impl ProjectItemModel {
    pub fn new(id: i32, title: String, environment: Env, group_name: Option<String>) -> Self {
        Object::builder()
            .property("id", id)
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
