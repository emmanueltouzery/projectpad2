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
        type Interfaces = (gio::ListModel, gtk::SectionModel);
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

    impl SectionModelImpl for ProjectItemListModel {
        fn section(&self, position: u32) -> (u32, u32) {
            (position, position + 2)
            // if position == 0 {
            //     return (0, 2);
            // }
            // return (3, 10);
        }
    }
}

glib::wrapper! {
    pub struct ProjectItemListModel(ObjectSubclass<imp::ProjectItemListModel>) @implements gio::ListModel, gtk::SectionModel;

}

impl ProjectItemListModel {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn append(&mut self, item: &ProjectItemModel) {
        self.imp().items.borrow_mut().push(item.clone());
    }
}
