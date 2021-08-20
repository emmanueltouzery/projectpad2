use gtk::prelude::*;
use projectpadsql::models::EnvironmentType;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg, Debug)]
pub enum Msg {
    EnvironmentSelected(EnvironmentType),
}

pub struct Model {
    environment: EnvironmentType,
}

#[widget]
impl Widget for EnvironmentPicker {
    fn init_view(&mut self) {
        self.widgets
            .radio_stg
            .join_group(Some(&self.widgets.radio_dev));
        self.widgets
            .radio_uat
            .join_group(Some(&self.widgets.radio_stg));
        self.widgets
            .radio_prd
            .join_group(Some(&self.widgets.radio_uat));
        match self.model.environment {
            EnvironmentType::EnvProd => self.widgets.radio_prd.set_active(true),
            EnvironmentType::EnvUat => self.widgets.radio_uat.set_active(true),
            EnvironmentType::EnvStage => self.widgets.radio_stg.set_active(true),
            EnvironmentType::EnvDevelopment => self.widgets.radio_dev.set_active(true),
        }
    }

    fn model(_relm: &relm::Relm<Self>, environment: EnvironmentType) -> Model {
        Model { environment }
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        #[style_class="linked"]
        gtk::Box {
            #[name="radio_dev"]
            gtk::RadioButton {
                label: "Dev",
                hexpand: true,
                mode: false,
                toggled => Msg::EnvironmentSelected(EnvironmentType::EnvDevelopment),
            },
            #[name="radio_stg"]
            gtk::RadioButton {
                label: "Stg",
                hexpand: true,
                mode: false,
                toggled => Msg::EnvironmentSelected(EnvironmentType::EnvStage),
            },
            #[name="radio_uat"]
            gtk::RadioButton {
                label: "Uat",
                hexpand: true,
                mode: false,
                toggled => Msg::EnvironmentSelected(EnvironmentType::EnvUat),
            },
            #[name="radio_prd"]
            gtk::RadioButton {
                label: "Prod",
                hexpand: true,
                mode: false,
                toggled => Msg::EnvironmentSelected(EnvironmentType::EnvProd),
            },
        }
    }
}
