use std::collections::HashSet;

use adw::prelude::*;
use projectpadsql::models::EnvironmentType;

mod imp {

    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };

    #[derive(Debug, Default)]
    pub struct EnvironmentListPicker {}

    #[glib::object_subclass]
    impl ObjectSubclass for EnvironmentListPicker {
        const NAME: &'static str = "EnvironmentListPicker";
        type ParentType = adw::Bin;
        type Type = super::EnvironmentListPicker;
    }

    impl ObjectImpl for EnvironmentListPicker {}

    impl WidgetImpl for EnvironmentListPicker {}

    impl adw::subclass::prelude::BinImpl for EnvironmentListPicker {}
}

glib::wrapper! {
    pub struct EnvironmentListPicker(ObjectSubclass<imp::EnvironmentListPicker>)
        @extends gtk::Widget, adw::Bin;
}

impl EnvironmentListPicker {
    pub fn new(envs: HashSet<EnvironmentType>) -> Self {
        let this = glib::Object::new::<Self>();
        let hbox = gtk::Box::builder().css_classes(["linked"]).build();
        hbox.append(
            &gtk::ToggleButton::builder()
                .label("DEV")
                .active(envs.contains(&EnvironmentType::EnvDevelopment))
                .build(),
        );
        hbox.append(
            &gtk::ToggleButton::builder()
                .label("STG")
                .active(envs.contains(&EnvironmentType::EnvStage))
                .build(),
        );
        hbox.append(
            &gtk::ToggleButton::builder()
                .label("UAT")
                .active(envs.contains(&EnvironmentType::EnvUat))
                .build(),
        );
        hbox.append(
            &gtk::ToggleButton::builder()
                .label("PRD")
                .active(envs.contains(&EnvironmentType::EnvProd))
                .build(),
        );
        this.set_child(Some(&hbox));
        this
    }
}
