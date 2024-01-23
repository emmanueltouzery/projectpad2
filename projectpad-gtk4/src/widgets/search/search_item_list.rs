use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

mod imp {
    use gtk::{
        subclass::{
            prelude::{BoxImpl, ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/search/search_item_list.ui"
    )]
    pub struct SearchItemList {
        #[template_child]
        pub search_item_list: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchItemList {
        const NAME: &'static str = "SearchItemList";
        type ParentType = gtk::Box;
        type Type = super::SearchItemList;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchItemList {
        fn constructed(&self) {
            self.obj().init_list();
        }
    }

    impl WidgetImpl for SearchItemList {}

    impl BoxImpl for SearchItemList {}
}

glib::wrapper! {
    pub struct SearchItemList(ObjectSubclass<imp::SearchItemList>)
        @extends gtk::Widget, gtk::Box;
}

impl SearchItemList {
    pub fn init_list(&self) {
        self.imp()
            .search_item_list
            .set_factory(Some(&gtk::BuilderListItemFactory::from_resource(
                Some(&gtk::BuilderRustScope::new()),
                "/com/github/emmanueltouzery/projectpad2/src/widgets/search/search_item_row.ui",
            )));
        // self.imp().search_item_list.set_header_factory(Some(
        //     &gtk::BuilderListItemFactory::from_resource(
        //         Some(&gtk::BuilderRustScope::new()),
        //         "/com/github/emmanueltouzery/projectpad2/src/widgets/search_item_header_row.ui",
        //     ),
        // ));
    }
}
