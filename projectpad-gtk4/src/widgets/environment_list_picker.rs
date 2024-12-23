use adw::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    use glib::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::EnvironmentListPicker)]
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
        let hbox = gtk::Box::builder()
            .css_classes(["linked"])
            .homogeneous(true)
            .valign(gtk::Align::Center)
            .build();

        let dev_btn = gtk::ToggleButton::builder()
            .label("DEV")
            .css_classes(["toggle-project-item-dev", "caption-heading"])
            .build();
        hbox.append(&dev_btn);
        this.bind_property("env_dev", &dev_btn, "active")
            .bidirectional()
            .sync_create()
            .build();

        let stg_btn = gtk::ToggleButton::builder()
            .label("STG")
            .css_classes(["toggle-project-item-staging", "caption-heading"])
            .build();
        this.bind_property("env_stg", &stg_btn, "active")
            .bidirectional()
            .sync_create()
            .build();
        hbox.append(&stg_btn);

        let uat_btn = gtk::ToggleButton::builder()
            .label("UAT")
            .css_classes(["toggle-project-item-uat", "caption-heading"])
            .build();
        this.bind_property("env_uat", &uat_btn, "active")
            .bidirectional()
            .sync_create()
            .build();
        hbox.append(&uat_btn);

        let prd_btn = gtk::ToggleButton::builder()
            .label("PRD")
            .css_classes(["toggle-project-item-prod", "caption-heading"])
            .build();
        this.bind_property("env_prd", &prd_btn, "active")
            .bidirectional()
            .sync_create()
            .build();
        hbox.append(&prd_btn);

        this.set_child(Some(&hbox));
        this
    }
}
