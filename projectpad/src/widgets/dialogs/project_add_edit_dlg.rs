use super::dialog_helpers;
use super::environments_picker::Msg::EnvironmentToggled as EnvironmentsPickerMsgEnvToggled;
use super::environments_picker::{EnvironmentsPicker, SelectedEnvironments};
use super::file_contents_button::FileContentsButton;
use super::file_contents_button::Msg::FileChanged as FileContentsButtonFileChanged;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, Project};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    EnvironmentToggled(EnvironmentType),
    IconChanged((Option<String>, Option<Vec<u8>>)),
    OkPressed,
}

pub struct Model {
    name: String,
    icon: Option<Vec<u8>>,
    has_dev: bool,
    has_stg: bool,
    has_uat: bool,
    has_prod: bool,
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
            has_dev: p.map(|p| p.has_dev).unwrap_or(false),
            has_stg: p.map(|p| p.has_stage).unwrap_or(false),
            has_uat: p.map(|p| p.has_uat).unwrap_or(false),
            has_prod: p.map(|p| p.has_prod).unwrap_or(false),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::EnvironmentToggled(EnvironmentType::EnvDevelopment) => {
                self.model.has_dev = !self.model.has_dev;
            }
            Msg::EnvironmentToggled(EnvironmentType::EnvStage) => {
                self.model.has_stg = !self.model.has_stg;
            }
            Msg::EnvironmentToggled(EnvironmentType::EnvUat) => {
                self.model.has_uat = !self.model.has_uat;
            }
            Msg::EnvironmentToggled(EnvironmentType::EnvProd) => {
                self.model.has_prod = !self.model.has_prod;
            }
            Msg::IconChanged((_, contents)) => {
                self.model.icon = contents;
            }
            Msg::OkPressed => {}
        }
    }

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
            FileContentsButton((
                Some(self.model.name.clone()).filter(|n| !n.is_empty()).map(|n| format!("<{} picture>", &n)),
                self.model.icon.clone(),
            )) {
                FileContentsButtonFileChanged(ref val) => Msg::IconChanged(val.clone()),
                cell: {
                    left_attach: 1,
                    top_attach: 6,
                },
            },
            EnvironmentsPicker(SelectedEnvironments {
                has_dev: self.model.has_dev,
                has_stg: self.model.has_stg,
                has_uat: self.model.has_uat,
                has_prod: self.model.has_prod,
            }) {
                cell: {
                    left_attach: 0,
                    top_attach: 3,
                    width: 2,
                },
                EnvironmentsPickerMsgEnvToggled(env_type) => Msg::EnvironmentToggled(env_type)
            },
        }
    }
}
