use glib::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

use super::project_item_model::{Env, ProjectItemModel};

// https://gtk-rs.org/gtk4-rs/stable/latest/book/todo_1.html
// https://gitlab.com/news-flash/news_flash_gtk/-/blob/master/src/article_list/models/article.rs?ref_type=heads
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
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_item_list.ui"
    )]
    pub struct ProjectItemList {
        #[template_child]
        pub project_item_list: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectItemList {
        const NAME: &'static str = "ProjectItemList";
        type ParentType = gtk::Box;
        type Type = super::ProjectItemList;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProjectItemList {
        fn constructed(&self) {
            self.obj().init_list();
        }
    }

    impl WidgetImpl for ProjectItemList {}

    impl BoxImpl for ProjectItemList {}
}

glib::wrapper! {
    pub struct ProjectItemList(ObjectSubclass<imp::ProjectItemList>)
        @extends gtk::Widget, gtk::Box;
}

impl ProjectItemList {
    pub fn init_list(&self) {
        self.imp().project_item_list.set_factory(Some(
            &gtk::BuilderListItemFactory::from_resource(
                Some(&gtk::BuilderRustScope::new()),
                "/com/github/emmanueltouzery/projectpad2/src/widgets/project_item_row.ui",
            ),
        ));
    }

    pub fn set_project_items(&mut self) {
        let list_store = gio::ListStore::new::<ProjectItemModel>();
        let item: ProjectItemModel = ProjectItemModel::new(1, "Prod".to_string(), Env::Prod); // glib::object::Object::new();
        list_store.append(&item);
        let item: ProjectItemModel = ProjectItemModel::new(2, "UAT".to_string(), Env::Uat); // glib::object::Object::new();
        list_store.append(&item);
        let selection_model = gtk::SingleSelection::new(Some(list_store));
        self.imp()
            .project_item_list
            .set_model(Some(&selection_model));
    }

    pub fn connect_activate<F: Fn(i32) + 'static>(&self, f: F) -> SignalHandlerId {
        self.imp()
            .project_item_list
            .connect_activate(move |list, idx| {
                f(list
                    .model()
                    .unwrap()
                    .upcast::<gio::ListModel>()
                    .item(idx)
                    .unwrap()
                    .property::<i32>("id"))
            })
    }
}
