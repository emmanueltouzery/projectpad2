use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

mod imp {
    use super::*;
    use std::cell::Cell;

    use glib::Properties;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };
    use projectpadsql::models::EnvironmentType;

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::EnvironmentPicker)]
    pub struct EnvironmentPicker {
        #[property(get, set)]
        pub environment: Cell<u8>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EnvironmentPicker {
        const NAME: &'static str = "EnvironmentPicker";
        type ParentType = adw::Bin;
        type Type = super::EnvironmentPicker;

        fn class_init(klass: &mut Self::Class) {
            // Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            // obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EnvironmentPicker {
        fn constructed(&self) {}
    }

    impl WidgetImpl for EnvironmentPicker {}

    impl adw::subclass::prelude::BinImpl for EnvironmentPicker {}
}

glib::wrapper! {
    pub struct EnvironmentPicker(ObjectSubclass<imp::EnvironmentPicker>)
        @extends gtk::Widget, adw::Bin;
}

impl EnvironmentPicker {
    pub fn new() -> Self {
        let this = glib::Object::new::<Self>();
        let dropdown = gtk::DropDown::from_strings(&["DEV", "STG", "UAT", "PRD"]);
        this.set_child(Some(&dropdown));
        this
    }
}
