use glib::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use gtk::subclass::widget::WidgetClassSubclassExt;

use crate::Project;

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
        #[template_child]
        pub add_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub project_badge_list: TemplateChild<gtk::Box>,
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
    pub fn set_projects(&mut self, project: Vec<Project>) {
        // self.imp().project_badge_list.clear();
    }
}
