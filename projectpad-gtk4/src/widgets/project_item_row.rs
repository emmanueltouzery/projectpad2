use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use gtk::subclass::widget::WidgetClassSubclassExt;
use gtk::{
    gio::Action,
    glib::{self, Sender},
};

mod imp {
    use super::*;
    use glib::subclass;
    use gtk::{
        subclass::{
            prelude::{BoxImpl, ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_list.ui")]
    pub struct ProjectItemRow {
        #[template_child]
        pub project_item_name: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectItemRow {
        const NAME: &'static str = "ProjectItemRow";
        type ParentType = gtk::Box;
        type Type = super::ProjectItemRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProjectItemRow {}

    impl WidgetImpl for ProjectItemRow {}

    impl BoxImpl for ProjectItemRow {}
}

glib::wrapper! {
    pub struct ProjectItemRow(ObjectSubclass<imp::ProjectItemRow>)
        @extends gtk::Widget, gtk::Box;
}

impl ProjectItemRow {
    // pub fn set_projects(&mut self, project: Vec<Project>) {
    //     // self.imp().project_badge_list.clear();
    // }
}
