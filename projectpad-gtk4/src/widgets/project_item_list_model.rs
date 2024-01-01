use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use super::project_item_model::ProjectItemModel;

mod imp {
    use std::cell::RefCell;

    use crate::widgets::project_item_model::ProjectItemModel;

    use super::*;

    #[derive(Debug, Default)]
    pub struct ProjectItemListModel {
        pub items: RefCell<Vec<ProjectItemModel>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectItemListModel {
        const NAME: &'static str = "ProjectItemListModel";
        type Type = super::ProjectItemListModel;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for ProjectItemListModel {}

    impl ListModelImpl for ProjectItemListModel {
        fn item_type(&self) -> glib::Type {
            ProjectItemModel::static_type()
        }

        fn n_items(&self) -> u32 {
            self.items.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.items
                .borrow()
                .get(usize::try_from(position).unwrap())
                .map(|o| o.clone().upcast::<glib::Object>())
        }
    }
}

glib::wrapper! {
    pub struct ProjectItemListModel(ObjectSubclass<imp::ProjectItemListModel>) @implements gio::ListModel;

}

impl ProjectItemListModel {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn append(&mut self, item: &ProjectItemModel) {
        self.imp().items.borrow_mut().push(item.clone());
    }
}
