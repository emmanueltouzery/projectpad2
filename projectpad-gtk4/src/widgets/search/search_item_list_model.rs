use std::collections::HashMap;

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

// need a special model so i can have list headers
// https://discourse.gnome.org/t/gtk4-listview-header-rows/18777

use super::search_item_model::SearchItemModel;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Debug, Default)]
    pub struct SearchItemListModel {
        pub items: RefCell<Vec<SearchItemModel>>,
        pub index_to_group: RefCell<HashMap<u32, (u32, u32)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchItemListModel {
        const NAME: &'static str = "SearchItemListModel";
        type Type = super::SearchItemListModel;
        type Interfaces = (gio::ListModel, gtk::SectionModel);
    }

    impl ObjectImpl for SearchItemListModel {}

    impl ListModelImpl for SearchItemListModel {
        fn item_type(&self) -> glib::Type {
            SearchItemModel::static_type()
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

    impl SectionModelImpl for SearchItemListModel {
        fn section(&self, position: u32) -> (u32, u32) {
            self.index_to_group.borrow()[&position]
        }
    }
}

glib::wrapper! {
    pub struct SearchItemListModel(ObjectSubclass<imp::SearchItemListModel>) @implements gio::ListModel, gtk::SectionModel;

}

impl SearchItemListModel {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn append(&mut self, item: &SearchItemModel) {
        self.imp().items.borrow_mut().push(item.clone());
    }

    pub fn set_group_start_indices(
        &mut self,
        items_len: usize,
        group_start_indices: HashMap<i32, String>,
    ) {
        let mut indices: Vec<u32> = group_start_indices
            .keys()
            .map(|k| u32::try_from(*k).unwrap())
            .collect();
        indices.sort();
        let mut index_to_group = HashMap::<u32, (u32, u32)>::new();
        let mut cur_idx = 0;
        if indices.is_empty() {
            index_to_group.insert(0, (0, u32::try_from(items_len).unwrap()));
        } else {
            for i in 0..indices.len() {
                let start = indices[i];
                if start > cur_idx {
                    index_to_group.insert(cur_idx, (cur_idx, start));
                }
                cur_idx = start;
                let end_idx = if i < indices.len() - 1 {
                    indices[i + 1]
                } else {
                    u32::try_from(items_len).unwrap()
                };
                index_to_group.insert(start, (start, end_idx));
            }
        }
        self.imp().index_to_group.replace(index_to_group);
    }
}
