use std::collections::HashMap;

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
        pub index_to_group: RefCell<HashMap<u32, (u32, u32)>>,
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
            self.index_to_group.borrow()[&position]
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

    pub fn set_group_start_indices(&mut self, group_start_indices: HashMap<i32, String>) {
        let mut indices: Vec<u32> = group_start_indices
            .keys()
            .map(|k| u32::try_from(*k).unwrap())
            .collect();
        indices.sort();
        let mut index_to_group = HashMap::<u32, (u32, u32)>::new();
        let mut cur_idx = 0;
        for i in 0..indices.len() {
            let start = indices[i];
            if start > cur_idx {
                index_to_group.insert(cur_idx, (cur_idx, start));
            }
            let end_idx = if i < indices.len() - 1 {
                indices[i + 1]
            } else {
                cur_idx + 1
            };
            index_to_group.insert(start, (start, end_idx));
            cur_idx = start;
        }
        self.imp().index_to_group.replace(index_to_group);
    }
}
