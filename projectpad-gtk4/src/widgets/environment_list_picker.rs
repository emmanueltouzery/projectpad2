use adw::prelude::*;
use gtk::subclass::prelude::*;
use projectpadsql::models::EnvironmentType;

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

        #[template_child]
        pub dev_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub stg_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub uat_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub prd_btn: TemplateChild<gtk::ToggleButton>,
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
    pub fn new(allowed_envs: &[EnvironmentType]) -> Self {
        let this = glib::Object::new::<Self>();
        if !allowed_envs.contains(&EnvironmentType::EnvDevelopment) {
            this.imp().dev_btn.set_sensitive(false);
        }
        if !allowed_envs.contains(&EnvironmentType::EnvStage) {
            this.imp().stg_btn.set_sensitive(false);
        }
        if !allowed_envs.contains(&EnvironmentType::EnvUat) {
            this.imp().uat_btn.set_sensitive(false);
        }
        if !allowed_envs.contains(&EnvironmentType::EnvProd) {
            this.imp().prd_btn.set_sensitive(false);
        }
        this
    }
}
