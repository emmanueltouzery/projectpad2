use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

use crate::search_engine;
use crate::widgets::project_items::common;
use crate::widgets::search::search_item_list::SearchItemList;
use std::str::FromStr;

use super::search_item_model::SearchItemType;

mod imp {
    use std::{cell::RefCell, rc::Rc, sync::OnceLock};

    use crate::widgets::search::search_item_list::SearchItemList;

    use super::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };
    use gtk::{CompositeTemplate, TemplateChild};
    use subclass::Signal;

    #[derive(Properties, Debug, Default, CompositeTemplate)]
    #[properties(wrapper_type = super::SearchPicker)]
    #[template(
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/search/search_picker.ui"
    )]
    pub struct SearchPicker {
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_item_list: TemplateChild<SearchItemList>,

        #[property(get, set)]
        search_item_types: Rc<RefCell<String>>,

        #[property(get, set)]
        selected_item_project_id: Rc<RefCell<i32>>,
        #[property(get, set)]
        selected_item_item_id: Rc<RefCell<i32>>,
        #[property(get, set)]
        selected_item_search_item_type: Rc<RefCell<u8>>,
        #[property(get, set)]
        selected_item_server_id: Rc<RefCell<i32>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchPicker {
        const NAME: &'static str = "SearchPicker";
        type ParentType = gtk::Box;
        type Type = super::SearchPicker;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SearchPicker {
        fn constructed(&self) {
            self.obj().init_widget();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("save-auth-key-to-disk")
                    .param_types([String::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for SearchPicker {}

    impl gtk::subclass::prelude::BoxImpl for SearchPicker {}
}

glib::wrapper! {
    pub struct SearchPicker(ObjectSubclass<imp::SearchPicker>)
        @extends gtk::Widget, gtk::Box;
}

impl SearchPicker {
    fn init_widget(&self) {
        self.set_spacing(5);
        self.connect_search_item_types_notify(|sp| {
            sp.refresh_search(sp.get_selection());
        });
        let s = self.clone();
        self.imp().search_entry.connect_changed(move |_| {
            s.refresh_search(None);
        });
        self.refresh_search(self.get_selection());

        self.imp().search_item_list.connect_closure(
            "select-item",
            false,
            glib::closure_local!(@strong self as s => move |_search_item_list: SearchItemList,
                                   project_id: i32, item_id: i32, search_item_type: u8, server_id: i32| {
                let _freeze_guard = s.freeze_notify(); // https://github.com/gtk-rs/gtk-rs-core/issues/1339
                s.set_properties(&[
                    ("selected-item-project-id", &project_id),
                    ("selected-item-item-id", &item_id),
                    ("selected-item-search-item-type", &search_item_type),
                    ("selected-item-server-id", &server_id)
                ]);
            }),
            );
    }

    fn get_selection(&self) -> Option<(SearchItemType, i32)> {
        let item_type_u8 = self.selected_item_search_item_type();
        let item_id = self.selected_item_item_id();
        if item_type_u8 != 0 && item_id != 0 {
            if let Some(search_item_type) = SearchItemType::from_repr(item_type_u8) {
                return Some((search_item_type, item_id));
            }
        }
        None
    }

    fn refresh_search(&self, selection: Option<(SearchItemType, i32)>) {
        let search_text = format!("%{}%", &self.imp().search_entry.text());
        let search_item_type =
            match search_engine::SearchItemsType::from_str(&self.search_item_types()) {
                Ok(sit) => sit,
                Err(_) => search_engine::SearchItemsType::All,
            };
        let search_results_receiver = common::run_sqlfunc(Box::new(move |sql_conn| {
            search_engine::run_search_filter(sql_conn, search_item_type, &search_text, &None, false)
        }));
        let mut sil = self.imp().search_item_list.clone();
        glib::spawn_future_local(async move {
            let search_res = search_results_receiver.recv().await.unwrap();
            sil.set_search_items(search_res, selection);
        });
    }
}
