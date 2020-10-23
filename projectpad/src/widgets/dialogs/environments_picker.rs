use gtk::prelude::*;
use projectpadsql::models::EnvironmentType;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    EnvironmentToggled(EnvironmentType),
}

#[derive(Clone)]
pub struct SelectedEnvironments {
    pub has_dev: bool,
    pub has_stg: bool,
    pub has_uat: bool,
    pub has_prod: bool,
}

pub struct Model {
    selected_environments: SelectedEnvironments,
}

#[widget]
impl Widget for EnvironmentsPicker {
    fn init_view(&mut self) {
        self.environments_box
            .get_style_context()
            .add_class("linked");
    }

    fn model(_relm: &relm::Relm<Self>, selected_environments: SelectedEnvironments) -> Model {
        Model {
            selected_environments,
        }
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        #[name="environments_box"]
        gtk::Box {
            #[name="radio_dev"]
            gtk::ToggleButton {
                label: "Dev",
                hexpand: true,
                active: self.model.selected_environments.has_dev,
                toggled => Msg::EnvironmentToggled(EnvironmentType::EnvDevelopment),
            },
            #[name="radio_stg"]
            gtk::ToggleButton {
                label: "Stg",
                hexpand: true,
                active: self.model.selected_environments.has_stg,
                toggled => Msg::EnvironmentToggled(EnvironmentType::EnvStage),
            },
            #[name="radio_uat"]
            gtk::ToggleButton {
                label: "Uat",
                hexpand: true,
                active: self.model.selected_environments.has_uat,
                toggled => Msg::EnvironmentToggled(EnvironmentType::EnvUat),
            },
            #[name="radio_prd"]
            gtk::ToggleButton {
                label: "Prd",
                hexpand: true,
                active: self.model.selected_environments.has_prod,
                toggled => Msg::EnvironmentToggled(EnvironmentType::EnvProd),
            },
        }
    }
}
