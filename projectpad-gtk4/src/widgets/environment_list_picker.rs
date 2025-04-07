use adw::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    use glib::*;
    use gtk::{
        subclass::{
            prelude::{ObjectImpl, ObjectSubclass},
            widget::WidgetImpl,
        },
        CompositeTemplate,
    };

    #[derive(Properties, Debug, Default, CompositeTemplate)]
    #[properties(wrapper_type = super::EnvironmentListPicker)]
    #[template(
        resource = "/com/github/emmanueltouzery/projectpad2/src/widgets/environment_list_picker.ui"
    )]
    pub struct EnvironmentListPicker {
        #[property(get, set)]
        env_dev: Rc<RefCell<bool>>,
        #[property(get, set)]
        env_stg: Rc<RefCell<bool>>,
        #[property(get, set)]
        env_uat: Rc<RefCell<bool>>,
        #[property(get, set)]
        env_prd: Rc<RefCell<bool>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EnvironmentListPicker {
        const NAME: &'static str = "EnvironmentListPicker";
        type ParentType = adw::Bin;
        type Type = super::EnvironmentListPicker;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EnvironmentListPicker {}

    impl WidgetImpl for EnvironmentListPicker {}

    impl adw::subclass::prelude::BinImpl for EnvironmentListPicker {}
}

glib::wrapper! {
    pub struct EnvironmentListPicker(ObjectSubclass<imp::EnvironmentListPicker>)
        @extends gtk::Widget, adw::Bin;
}

impl EnvironmentListPicker {
    pub fn new() -> Self {
        let this = glib::Object::new::<Self>();
        this
    }
}
