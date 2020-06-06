use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, Project};
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    ProjectActivated(Project),
    EnvironmentToggled(EnvironmentType), // implementation detail
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
            Msg::EnvironmentToggled(env) => match env {
                // sadly the radio button api is a bit of mess, toggled is emitted
                // on both the one that gets de-activated and the one that gets
                // activated. 'clicked' does the same, too.
                // => must filter to re-emit only the one that gets activated.
                // https://stackoverflow.com/questions/13385024/read-gtk-radio-button-signal-only-when-selected
                EnvironmentType::EnvDevelopment if self.radio_dev.get_active() => self
                    .model
                    .relm
                    .stream()
                    .emit(Msg::EnvironmentChanged(EnvironmentType::EnvDevelopment)),
                EnvironmentType::EnvStage if self.radio_stg.get_active() => self
                    .model
                    .relm
                    .stream()
                    .emit(Msg::EnvironmentChanged(EnvironmentType::EnvStage)),
                EnvironmentType::EnvUat if self.radio_uat.get_active() => self
                    .model
                    .relm
                    .stream()
                    .emit(Msg::EnvironmentChanged(EnvironmentType::EnvUat)),
                EnvironmentType::EnvProd if self.radio_prd.get_active() => self
                    .model
                    .relm
                    .stream()
                    .emit(Msg::EnvironmentChanged(EnvironmentType::EnvProd)),
                _ => {}
            },
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
                    toggled => Msg::EnvironmentToggled(EnvironmentType::EnvDevelopment)
                },
                #[name="radio_stg"]
                gtk::RadioButton {
                    label: "Stg",
                    mode: false,
                    sensitive: false,
                    toggled => Msg::EnvironmentToggled(EnvironmentType::EnvStage)
                },
                #[name="radio_uat"]
                gtk::RadioButton {
                    label: "Uat",
                    mode: false,
                    sensitive: false,
                    toggled => Msg::EnvironmentToggled(EnvironmentType::EnvUat)
                },
                #[name="radio_prd"]
                gtk::RadioButton {
                    label: "Prd",
                    mode: false,
                    sensitive: false,
                    toggled => Msg::EnvironmentToggled(EnvironmentType::EnvProd)
                },
            }
        }
    }
}
