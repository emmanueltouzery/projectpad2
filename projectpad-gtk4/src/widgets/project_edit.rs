use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use crate::widgets::environment_list_picker::EnvironmentListPicker;

    use super::*;
    use glib::Properties;
    use gtk::{
        subclass::{
            prelude::{ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Properties, Debug, Default, CompositeTemplate)]
    #[properties(wrapper_type = super::ProjectEdit)]
    #[template(resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/project_edit.ui")]
    pub struct ProjectEdit {
        #[property(get, set)]
        title: Rc<RefCell<String>>,

        #[property(get, set)]
        env_dev: Rc<RefCell<bool>>,
        #[property(get, set)]
        env_stg: Rc<RefCell<bool>>,
        #[property(get, set)]
        env_uat: Rc<RefCell<bool>>,
        #[property(get, set)]
        env_prd: Rc<RefCell<bool>>,

        #[template_child]
        pub env_picker: TemplateChild<EnvironmentListPicker>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectEdit {
        const NAME: &'static str = "ProjectEdit";
        type ParentType = adw::Bin;
        type Type = super::ProjectEdit;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectEdit {}

    impl WidgetImpl for ProjectEdit {}

    impl adw::subclass::prelude::BinImpl for ProjectEdit {}
}

glib::wrapper! {
    pub struct ProjectEdit(ObjectSubclass<imp::ProjectEdit>)
        @extends gtk::Widget, adw::Bin;
}

impl ProjectEdit {
    pub fn new() -> Self {
        let this = glib::Object::new::<Self>();

        this
    }
}
