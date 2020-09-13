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
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, selected_environments: SelectedEnvironments) -> Model {
        Model {
            selected_environments,
        }
    }

    fn update(&mut self, event: Msg) {}

    view! {
        gtk::Grid {
            row_spacing: 5,
            column_spacing: 10,
            #[name="radio_dev"]
            gtk::ToggleButton {
                cell: {
                    left_attach: 0,
                    top_attach: 0,
                },
                label: "Dev",
                hexpand: true,
                active: self.model.selected_environments.has_dev,
                toggled => Msg::EnvironmentToggled(EnvironmentType::EnvDevelopment),
            },
            #[name="radio_stg"]
            gtk::ToggleButton {
                cell: {
                    left_attach: 1,
                    top_attach: 0,
                },
                label: "Stg",
                hexpand: true,
                active: self.model.selected_environments.has_stg,
                toggled => Msg::EnvironmentToggled(EnvironmentType::EnvStage),
            },
            #[name="radio_uat"]
            gtk::ToggleButton {
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
                label: "Uat",
                hexpand: true,
                active: self.model.selected_environments.has_uat,
                toggled => Msg::EnvironmentToggled(EnvironmentType::EnvUat),
            },
            #[name="radio_prd"]
            gtk::ToggleButton {
                cell: {
                    left_attach: 1,
                    top_attach: 1,
                },
                label: "Prd",
                hexpand: true,
                active: self.model.selected_environments.has_prod,
                toggled => Msg::EnvironmentToggled(EnvironmentType::EnvProd),
            },
        }
    }
}
