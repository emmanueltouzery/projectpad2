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
    pub fn new(title: String) -> Self {
        Object::builder().property("title", title).build()
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
