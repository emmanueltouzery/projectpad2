use super::auth_key_button::AuthKeyButton;
use super::auth_key_button::Msg::AuthFileChanged as AuthKeyButtonFileChanged;
use super::dialog_helpers;
use super::environments_picker::Msg::EnvironmentToggled as EnvironmentsPickerMsgEnvToggled;
use super::environments_picker::{EnvironmentsPicker, SelectedEnvironments};
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::Project;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    OkPressed,
}

pub struct Model {
    name: String,
    icon: Option<Vec<u8>>,
    selected_environments: SelectedEnvironments,
}

#[widget]
impl Widget for ProjectAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, Option<Project>, gtk::AccelGroup),
    ) -> Model {
        let (sql_sender, project, _) = params;
        let p = project.as_ref();
        Model {
            name: p.map(|p| p.name.clone()).unwrap_or("".to_string()),
            icon: p.and_then(|p| p.icon.clone()),
            selected_environments: p
                .map(|p| SelectedEnvironments {
                    has_dev: p.has_dev,
                    has_uat: p.has_uat,
                    has_stg: p.has_stage,
                    has_prod: p.has_prod,
                })
                .unwrap_or(SelectedEnvironments {
                    has_dev: false,
                    has_stg: false,
                    has_uat: false,
                    has_prod: false,
                }),
        }
    }

    fn update(&mut self, event: Msg) {}

    view! {
        #[name="grid"]
        gtk::Grid {
            gtk::Label {
                text: "Name",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
            },
            #[name="name_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.name,
                cell: {
                    left_attach: 1,
                    top_attach: 1,
                },
            },
            AuthKeyButton((
                Some(self.model.name.clone()).filter(|n| !n.is_empty()),
                self.model.icon.clone(),
            )) {
                // AuthKeyButtonFileChanged(ref val) => Msg::AuthFileChanged(val.clone()),
                cell: {
                    left_attach: 1,
                    top_attach: 6,
                },
            },
            EnvironmentsPicker(self.model.selected_environments.clone()) {
                cell: {
                    left_attach: 0,
                    top_attach: 3,
                    width: 2,
                },
                // EnvironmentsPickerMsgEnvToggled(env_type) => Msg::EnvironmentToggled(env_type)
            },
        }
    }
}
