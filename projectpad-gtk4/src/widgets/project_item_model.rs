use glib::prelude::*;
use glib::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ProjectItemModel)]
    pub struct ProjectItemModel {
        #[property(get, set)]
        title: Rc<RefCell<String>>,
        #[property(get, set)]
        css_classes: Rc<RefCell<Vec<String>>>,
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

fn env_to_css(val: Env) -> Vec<String> {
    vec![match val {
        Env::Dev => "project-item-dev",
        Env::Staging => "project-item-staging",
        Env::Uat => "project-item-uat",
        Env::Prod => "project-item-prod",
    }
    .to_string()]
}

impl ProjectItemModel {
    pub fn new(title: String, environment: Env) -> Self {
        Object::builder()
            .property("title", title)
            .property("css-classes", env_to_css(environment))
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
