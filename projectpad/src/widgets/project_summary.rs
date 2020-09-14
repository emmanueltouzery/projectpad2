use super::dialogs::dialog_helpers;
use super::dialogs::project_add_edit_dlg;
use super::dialogs::project_add_edit_dlg::Msg as MsgProjectAddEditDialog;
use super::dialogs::project_add_edit_dlg::ProjectAddEditDialog;
use super::dialogs::project_add_item_dlg;
use super::dialogs::project_add_item_dlg::ProjectAddItemDialog;
use super::dialogs::standard_dialogs;
use super::project_items_list::ProjectItem;
use crate::icons::Icon;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, Project};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    ProjectActivated(Project),
    ProjectUpdated(Project),
    EnvironmentToggled(EnvironmentType), // implementation detail
    EnvironmentChanged(EnvironmentType),
    ProjectEnvironmentSelectedFromElsewhere((Project, EnvironmentType)),
    AddProjectItem,
    EditProject,
    ProjectAddItemActionCompleted(ProjectItem),
    ProjectAddItemChangeTitleTitle(&'static str),
    ProjectItemAdded(ProjectItem),
}

pub struct Model {
    relm: relm::Relm<ProjectSummary>,
    db_sender: mpsc::Sender<SqlFunc>,
    project: Option<Project>,
    title: gtk::Label,
    btn_and_handler: Vec<(gtk::RadioButton, glib::SignalHandlerId)>,
    header_popover: gtk::Popover,
    project_add_edit_dialog: Option<(relm::Component<ProjectAddEditDialog>, gtk::Dialog)>,
    project_add_item_component: Option<relm::Component<ProjectAddItemDialog>>,
    project_add_item_dialog: Option<gtk::Dialog>,
    cur_environment: EnvironmentType,
}

#[widget]
impl Widget for ProjectSummary {
    fn init_view(&mut self) {
        self.model.title.show_all();

        self.radio_stg.join_group(Some(&self.radio_dev));
        self.radio_uat.join_group(Some(&self.radio_stg));
        self.radio_prd.join_group(Some(&self.radio_uat));

        // must tie the signal handlers manually so i can block emission when we get
        // updated from outside
        let relm = self.model.relm.clone();
        self.model.btn_and_handler.push((
            self.radio_dev.clone(),
            self.radio_dev.connect_toggled(move |_| {
                relm.stream()
                    .emit(Msg::EnvironmentToggled(EnvironmentType::EnvDevelopment))
            }),
        ));
        let relm = self.model.relm.clone();
        self.model.btn_and_handler.push((
            self.radio_stg.clone(),
            self.radio_stg.connect_toggled(move |_| {
                relm.stream()
                    .emit(Msg::EnvironmentToggled(EnvironmentType::EnvStage))
            }),
        ));
        let relm = self.model.relm.clone();
        self.model.btn_and_handler.push((
            self.radio_uat.clone(),
            self.radio_uat.connect_toggled(move |_| {
                relm.stream()
                    .emit(Msg::EnvironmentToggled(EnvironmentType::EnvUat))
            }),
        ));
        let relm = self.model.relm.clone();
        self.model.btn_and_handler.push((
            self.radio_prd.clone(),
            self.radio_prd.connect_toggled(move |_| {
                relm.stream()
                    .emit(Msg::EnvironmentToggled(EnvironmentType::EnvProd))
            }),
        ));
        self.init_actions_popover();
    }

    fn init_actions_popover(&self) {
        let popover_vbox = gtk::BoxBuilder::new()
            .margin(10)
            .orientation(gtk::Orientation::Vertical)
            .build();
        let popover_btn = gtk::ModelButtonBuilder::new().label("Add...").build();
        relm::connect!(
            self.model.relm,
            popover_btn,
            connect_clicked(_),
            Msg::AddProjectItem
        );
        popover_vbox.add(&popover_btn);
        let popover_edit_btn = gtk::ModelButtonBuilder::new().label("Edit").build();
        relm::connect!(
            self.model.relm,
            popover_edit_btn,
            connect_clicked(_),
            Msg::EditProject
        );
        popover_vbox.add(&popover_edit_btn);
        popover_vbox.show_all();
        self.model.header_popover.add(&popover_vbox);
        self.header_actions_btn
            .set_popover(Some(&self.model.header_popover));
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        Model {
            project: None,
            db_sender,
            relm: relm.clone(),
            title: gtk::LabelBuilder::new()
                .margin_top(8)
                .margin_bottom(8)
                .build(),
            btn_and_handler: vec![],
            header_popover: gtk::Popover::new(None::<&gtk::Button>),
            project_add_item_dialog: None,
            project_add_item_component: None,
            project_add_edit_dialog: None,
            cur_environment: EnvironmentType::EnvDevelopment,
        }
    }

    fn set_project(&mut self, project: Project) {
        self.model.project = Some(project);
        self.model.title.set_markup(
            &self
                .model
                .project
                .as_ref()
                .map(|p| format!("<b>{}</b>", &p.name))
                .unwrap_or("".to_string()),
        );
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectUpdated(prj) => {
                self.model
                    .project_add_edit_dialog
                    .as_ref()
                    .unwrap()
                    .1
                    .close();
                self.model.project_add_edit_dialog = None;
                self.model.relm.stream().emit(Msg::ProjectActivated(prj));
            }
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
                self.set_project(prj);
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
            Msg::EnvironmentChanged(env) => {
                /* also meant for my parent */
                self.model.cur_environment = env;
            }
            Msg::ProjectEnvironmentSelectedFromElsewhere((prj, env)) => {
                for (btn, handler_id) in &self.model.btn_and_handler {
                    // block the event handlers so that we don't spuriously notify
                    // others of this change
                    btn.block_signal(handler_id);
                }
                match env {
                    EnvironmentType::EnvProd => self.radio_prd.set_active(true),
                    EnvironmentType::EnvUat => self.radio_uat.set_active(true),
                    EnvironmentType::EnvStage => self.radio_stg.set_active(true),
                    EnvironmentType::EnvDevelopment => self.radio_dev.set_active(true),
                }
                self.radio_dev.set_sensitive(prj.has_dev);
                self.radio_stg.set_sensitive(prj.has_stage);
                self.radio_uat.set_sensitive(prj.has_uat);
                self.radio_prd.set_sensitive(prj.has_prod);
                self.set_project(prj);
                for (btn, handler_id) in &self.model.btn_and_handler {
                    // unblock the event handlers
                    btn.unblock_signal(handler_id);
                }
            }
            Msg::ProjectAddItemActionCompleted(project_item) => {
                self.model.project_add_item_dialog.as_ref().unwrap().close();
                self.model.project_add_item_dialog = None;
                self.model.project_add_item_component = None;
                // refresh
                self.model
                    .relm
                    .stream()
                    .emit(Msg::ProjectItemAdded(project_item.clone()));
            }
            Msg::ProjectAddItemChangeTitleTitle(title) => {
                self.model
                    .project_add_item_dialog
                    .as_ref()
                    .unwrap()
                    .set_title(title);
            }
            Msg::AddProjectItem => {
                self.show_project_add_item_dialog();
            }
            Msg::EditProject => {
                self.show_project_edit_dialog();
            }
            // meant for my parent
            Msg::ProjectItemAdded(_) => {}
        }
    }

    fn show_project_edit_dialog(&mut self) {
        let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
            self.project_summary_root.clone().upcast::<gtk::Widget>(),
            (
                self.model.db_sender.clone(),
                self.model.project.clone(),
                gtk::AccelGroup::new(),
            ),
            MsgProjectAddEditDialog::OkPressed,
            "Project",
        );
        relm::connect!(
            component@MsgProjectAddEditDialog::ProjectUpdated(ref project),
            self.model.relm,
            Msg::ProjectUpdated(project.clone())
        );
        self.model.project_add_edit_dialog = Some((component, dialog.clone()));
        dialog.show();
    }

    fn show_project_add_item_dialog(&mut self) {
        let dialog_contents = relm::init::<ProjectAddItemDialog>((
            self.model.db_sender.clone(),
            self.model.project.as_ref().unwrap().id,
            self.model.cur_environment,
        ))
        .expect("error initializing the server add item modal");
        let d_c = dialog_contents.clone();
        let dialog = standard_dialogs::modal_dialog(
            self.project_summary_root.clone().upcast::<gtk::Widget>(),
            600,
            200,
            "Add project item".to_string(),
        );
        let (dialog, component, ok_btn) = standard_dialogs::prepare_custom_dialog(
            dialog.clone(),
            dialog_contents,
            move |ok_btn| {
                if ok_btn.get_label() == Some("Next".into()) {
                    d_c.emit(project_add_item_dlg::Msg::ShowSecondTab(dialog.clone()));
                    ok_btn.set_label("Done");
                } else {
                    d_c.emit(project_add_item_dlg::Msg::OkPressed);
                }
            },
        );
        ok_btn.set_label("Next");
        relm::connect!(
            component@project_add_item_dlg::Msg::ActionCompleted(ref pi),
            self.model.relm,
            Msg::ProjectAddItemActionCompleted(pi.clone())
        );
        relm::connect!(
            component@project_add_item_dlg::Msg::ChangeDialogTitle(title),
            self.model.relm,
            Msg::ProjectAddItemChangeTitleTitle(title)
        );
        self.model.project_add_item_component = Some(component);
        self.model.project_add_item_dialog = Some(dialog.clone());
        dialog.show();
    }

    view! {
        #[name="project_summary_root"]
        gtk::Box {
            orientation: gtk::Orientation::Vertical,
            gtk::Box {
                center_widget: Some(&self.model.title),
                #[name="header_actions_btn"]
                gtk::MenuButton {
                    child: {
                        pack_type: gtk::PackType::End,
                    },
                    always_show_image: true,
                    image: Some(&gtk::Image::from_icon_name(
                        Some(Icon::COG.name()), gtk::IconSize::Menu)),
                    halign: gtk::Align::End,
                    valign: gtk::Align::Center,
                    margin_top: 5,
                    margin_end: 5,
                },
            },
            gtk::Box {
                homogeneous: true,
                child: {
                    padding: 5,
                },
                spacing: 3,
                #[name="radio_dev"]
                gtk::RadioButton {
                    label: "Dev",
                    mode: false,
                    sensitive: false,
                    // must tie the event handlers in the init_view so i can temporarily block them sometimes
                    // toggled => Msg::EnvironmentToggled(EnvironmentType::EnvDevelopment)
                },
                #[name="radio_stg"]
                gtk::RadioButton {
                    label: "Stg",
                    mode: false,
                    sensitive: false,
                },
                #[name="radio_uat"]
                gtk::RadioButton {
                    label: "Uat",
                    mode: false,
                    sensitive: false,
                },
                #[name="radio_prd"]
                gtk::RadioButton {
                    label: "Prd",
                    mode: false,
                    sensitive: false,
                },
            }
        }
    }
}
