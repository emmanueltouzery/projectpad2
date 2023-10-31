use glib::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

use crate::ProjectItem;

use super::project_item_model::ProjectItemModel;
use super::project_item_row::ProjectItemRow;

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
    #[template(resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_list.ui")]
    pub struct ProjectList {
        // #[template_child]
        // pub add_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub project_item_list: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectList {
        const NAME: &'static str = "ProjectList";
        type ParentType = gtk::Box;
        type Type = super::ProjectList;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProjectList {}

    impl WidgetImpl for ProjectList {}

    impl BoxImpl for ProjectList {}
}

glib::wrapper! {
    pub struct ProjectList(ObjectSubclass<imp::ProjectList>)
        @extends gtk::Widget, gtk::Box;
}

impl ProjectList {
    pub fn set_project_items(&mut self, project: Vec<ProjectItem>) {
        let list_store = gio::ListStore::new::<ProjectItemModel>();
        let item: ProjectItemModel = ProjectItemModel::new("it me".to_string()); // glib::object::Object::new();
        list_store.append(&item);
        let selection_model = gtk::NoSelection::new(Some(list_store));
        self.imp()
            .project_item_list
            .set_model(Some(&selection_model));
    }
}
