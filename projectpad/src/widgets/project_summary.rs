use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, Project};
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    ProjectActivated(Project),
    EnvironmentChanged(EnvironmentType),
}

pub struct Model {
    relm: relm::Relm<ProjectSummary>,
    project: Option<Project>,
}

#[widget]
impl Widget for ProjectSummary {
    fn init_view(&mut self) {
        self.radio_stg.join_group(Some(&self.radio_dev));
        self.radio_uat.join_group(Some(&self.radio_stg));
        self.radio_prd.join_group(Some(&self.radio_uat));
    }

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {
            project: None,
            relm: relm.clone(),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectActivated(prj) => {
                self.radio_dev.set_sensitive(prj.has_dev);
                self.radio_stg.set_sensitive(prj.has_stage);
                self.radio_uat.set_sensitive(prj.has_uat);
                self.radio_prd.set_sensitive(prj.has_prod);
                if prj.has_prod {
                    self.radio_prd.set_active(true);
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::EnvironmentChanged(EnvironmentType::EnvProd));
                } else if prj.has_uat {
                    self.radio_uat.set_active(true);
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::EnvironmentChanged(EnvironmentType::EnvUat));
                } else if prj.has_stage {
                    self.radio_stg.set_active(true);
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::EnvironmentChanged(EnvironmentType::EnvStage));
                } else if prj.has_dev {
                    self.radio_dev.set_active(true);
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::EnvironmentChanged(EnvironmentType::EnvDevelopment));
                }
                self.model.project = Some(prj);
            }
            Msg::EnvironmentChanged(_) => { /* meant for my parent */ }
        }
    }

    view! {
        gtk::Box {
            orientation: gtk::Orientation::Vertical,
            gtk::Label {
                margin_top: 8,
                margin_bottom: 8,
                markup: &self.model.project.as_ref()
                            .map(|p| format!("<b>{}</b>", &p.name))
                            .unwrap_or("".to_string())
            },
            gtk::Box {
                homogeneous: true,
                margin_start: 35,
                margin_end: 35,
                child: {
                    padding: 5,
                },
                spacing: 3,
                #[name="radio_dev"]
                gtk::RadioButton {
                    label: "Dev",
                    mode: false,
                    sensitive: false,
                    clicked => Msg::EnvironmentChanged(EnvironmentType::EnvDevelopment)
                },
                #[name="radio_stg"]
                gtk::RadioButton {
                    label: "Stg",
                    mode: false,
                    sensitive: false,
                    clicked => Msg::EnvironmentChanged(EnvironmentType::EnvStage)
                },
                #[name="radio_uat"]
                gtk::RadioButton {
                    label: "Uat",
                    mode: false,
                    sensitive: false,
                    clicked => Msg::EnvironmentChanged(EnvironmentType::EnvUat)
                },
                #[name="radio_prd"]
                gtk::RadioButton {
                    label: "Prd",
                    mode: false,
                    sensitive: false,
                    clicked => Msg::EnvironmentChanged(EnvironmentType::EnvProd)
                },
            }
        }
    }
}
