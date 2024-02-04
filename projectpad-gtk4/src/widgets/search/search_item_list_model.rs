use std::collections::HashMap;

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

// need a special model so i can have list headers
// https://discourse.gnome.org/t/gtk4-listview-header-rows/18777

use super::search_item_model::{SearchItemModel, SearchItemType};

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Debug, Default)]
    pub struct SearchItemListModel {
        pub items: RefCell<Vec<SearchItemModel>>,
        pub projects_indices: RefCell<Vec<usize>>,
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
        fn section(&self, position_: u32) -> (u32, u32) {
            let position = position_ as usize;
            let projects_indices = self.projects_indices.borrow();
            let items = self.items.borrow();
            let mut cur_pos = 0;
            while cur_pos < projects_indices.len() {
                if projects_indices[cur_pos] >= position {
                    return (
                        projects_indices[cur_pos] as u32,
                        if cur_pos == projects_indices.len() - 1 {
                            items.len()
                        } else {
                            projects_indices[cur_pos + 1]
                        } as u32,
                    );
                }
                cur_pos += 1;
            }
            (0, projects_indices.len() as u32)
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
        if item.property_value("search_item_type").get() == Ok(SearchItemType::Project as u8) {
            self.imp()
                .projects_indices
                .borrow_mut()
                .push(self.imp().items.borrow().len());
        } else {
            self.imp().items.borrow_mut().push(item.clone());
        }
    }

    pub fn get_search_item(&self, index: u32) -> Option<(i32, u8)> {
        if let Some(search_item_model) = self.imp().items.borrow().get(index as usize) {
            Some((search_item_model.id(), search_item_model.search_item_type()))
        } else {
            None
        }
    }
}
