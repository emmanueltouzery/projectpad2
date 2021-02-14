use super::dialogs::dialog_helpers;
use super::dialogs::project_add_edit_dlg::Msg as MsgProjectAddEditDialog;
use super::dialogs::project_add_edit_dlg::ProjectAddEditDialog;
use super::dialogs::project_add_item_dlg;
use super::dialogs::project_add_item_dlg::ProjectAddItemDialog;
use super::dialogs::standard_dialogs;
use super::project_items_list::ProjectItem;
use super::wintitlebar::left_align_menu;
use crate::icons::Icon;
use crate::sql_thread::SqlFunc;
use crate::sql_util;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{
    EnvironmentType, Project, Server, ServerDatabase, ServerLink, ServerWebsite,
};
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
    AskDeleteProject,
    DeleteProject,
    ProjectDeleted(Project),
    ProjectAddItemActionCompleted(Box<ProjectItem>),
    ProjectAddItemChangeTitleTitle(&'static str),
    ProjectItemAdded(ProjectItem),
}

// String for details, because I can't pass Error across threads
type DeleteResult = Result<Project, (&'static str, Option<String>)>;

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
    _project_deleted_channel: relm::Channel<DeleteResult>,
    project_deleted_sender: relm::Sender<DeleteResult>,
}

#[widget]
impl Widget for ProjectSummary {
    fn init_view(&mut self) {
        self.model.title.show_all();

        self.widgets
            .buttons_box
            .get_style_context()
            .add_class("linked");

        self.widgets
            .radio_stg
            .join_group(Some(&self.widgets.radio_dev));
        self.widgets
            .radio_uat
            .join_group(Some(&self.widgets.radio_stg));
        self.widgets
            .radio_prd
            .join_group(Some(&self.widgets.radio_uat));

        // must tie the signal handlers manually so i can block emission when we get
        // updated from outside
        let relm = self.model.relm.clone();
        self.model.btn_and_handler.push((
            self.widgets.radio_dev.clone(),
            self.widgets.radio_dev.connect_toggled(move |_| {
                relm.stream()
                    .emit(Msg::EnvironmentToggled(EnvironmentType::EnvDevelopment))
            }),
        ));
        let relm = self.model.relm.clone();
        self.model.btn_and_handler.push((
            self.widgets.radio_stg.clone(),
            self.widgets.radio_stg.connect_toggled(move |_| {
                relm.stream()
                    .emit(Msg::EnvironmentToggled(EnvironmentType::EnvStage))
            }),
        ));
        let relm = self.model.relm.clone();
        self.model.btn_and_handler.push((
            self.widgets.radio_uat.clone(),
            self.widgets.radio_uat.connect_toggled(move |_| {
                relm.stream()
                    .emit(Msg::EnvironmentToggled(EnvironmentType::EnvUat))
            }),
        ));
        let relm = self.model.relm.clone();
        self.model.btn_and_handler.push((
            self.widgets.radio_prd.clone(),
            self.widgets.radio_prd.connect_toggled(move |_| {
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
        let popover_add_btn = gtk::ModelButtonBuilder::new()
            .label("Add project item...")
            .build();
        left_align_menu(&popover_add_btn);
        relm::connect!(
            self.model.relm,
            popover_add_btn,
            connect_clicked(_),
            Msg::AddProjectItem
        );
        popover_vbox.add(&popover_add_btn);
        let popover_edit_btn = gtk::ModelButtonBuilder::new().label("Edit").build();
        left_align_menu(&popover_edit_btn);
        relm::connect!(
            self.model.relm,
            popover_edit_btn,
            connect_clicked(_),
            Msg::EditProject
        );
        popover_vbox.add(&popover_edit_btn);
        let popover_delete_btn = gtk::ModelButtonBuilder::new().label("Delete").build();
        left_align_menu(&popover_delete_btn);
        relm::connect!(
            self.model.relm,
            popover_delete_btn,
            connect_clicked(_),
            Msg::AskDeleteProject
        );
        popover_vbox.add(&popover_delete_btn);
        popover_vbox.show_all();
        self.model.header_popover.add(&popover_vbox);
        self.widgets
            .header_actions_btn
            .set_popover(Some(&self.model.header_popover));
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (_project_deleted_channel, project_deleted_sender) =
            relm::Channel::new(move |r: DeleteResult| match r {
                Ok(p) => stream.emit(Msg::ProjectDeleted(p)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
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
            _project_deleted_channel,
            project_deleted_sender,
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
                .unwrap_or_else(|| "".to_string()),
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
                self.widgets.radio_dev.set_sensitive(prj.has_dev);
                self.widgets.radio_stg.set_sensitive(prj.has_stage);
                self.widgets.radio_uat.set_sensitive(prj.has_uat);
                self.widgets.radio_prd.set_sensitive(prj.has_prod);
                if prj.has_prod {
                    self.widgets.radio_prd.set_active(true);
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::EnvironmentChanged(EnvironmentType::EnvProd));
                } else if prj.has_uat {
                    self.widgets.radio_uat.set_active(true);
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::EnvironmentChanged(EnvironmentType::EnvUat));
                } else if prj.has_stage {
                    self.widgets.radio_stg.set_active(true);
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::EnvironmentChanged(EnvironmentType::EnvStage));
                } else if prj.has_dev {
                    self.widgets.radio_dev.set_active(true);
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
                EnvironmentType::EnvDevelopment if self.widgets.radio_dev.get_active() => self
                    .model
                    .relm
                    .stream()
                    .emit(Msg::EnvironmentChanged(EnvironmentType::EnvDevelopment)),
                EnvironmentType::EnvStage if self.widgets.radio_stg.get_active() => self
                    .model
                    .relm
                    .stream()
                    .emit(Msg::EnvironmentChanged(EnvironmentType::EnvStage)),
                EnvironmentType::EnvUat if self.widgets.radio_uat.get_active() => self
                    .model
                    .relm
                    .stream()
                    .emit(Msg::EnvironmentChanged(EnvironmentType::EnvUat)),
                EnvironmentType::EnvProd if self.widgets.radio_prd.get_active() => self
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
                    EnvironmentType::EnvProd => self.widgets.radio_prd.set_active(true),
                    EnvironmentType::EnvUat => self.widgets.radio_uat.set_active(true),
                    EnvironmentType::EnvStage => self.widgets.radio_stg.set_active(true),
                    EnvironmentType::EnvDevelopment => self.widgets.radio_dev.set_active(true),
                }
                self.widgets.radio_dev.set_sensitive(prj.has_dev);
                self.widgets.radio_stg.set_sensitive(prj.has_stage);
                self.widgets.radio_uat.set_sensitive(prj.has_uat);
                self.widgets.radio_prd.set_sensitive(prj.has_prod);
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
                    .emit(Msg::ProjectItemAdded(*project_item));
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
            Msg::AskDeleteProject => {
                self.handle_project_delete();
            }
            Msg::DeleteProject => {
                if let Some(prj) = self.model.project.clone() {
                    self.delete_project(prj);
                }
            }
            // meant for my parent
            Msg::ProjectDeleted(_) => {}
            // meant for my parent
            Msg::ProjectItemAdded(_) => {}
        }
    }

    fn delete_project(&self, prj: Project) {
        let prj_id = prj.id;
        let s = self.model.project_deleted_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                use projectpadsql::schema::server::dsl as srv;
                use projectpadsql::schema::server_database::dsl as db;
                use projectpadsql::schema::server_link::dsl as srv_link;
                use projectpadsql::schema::server_website::dsl as srvw;

                // we cannot delete a project if a server under it is
                // linked to from another project
                let dependent_serverlinks = srv_link::server_link
                    .inner_join(srv::server)
                    .filter(
                        srv::project_id
                            .eq(prj_id)
                            .and(srv_link::project_id.ne(prj_id)),
                    )
                    .load::<(ServerLink, Server)>(sql_conn)
                    .unwrap();

                let contained_dbs: Vec<_> = db::server_database
                    .inner_join(srv::server)
                    .filter(srv::project_id.eq(prj_id))
                    .load::<(ServerDatabase, Server)>(sql_conn)
                    .unwrap()
                    .into_iter()
                    .map(|x| x.0)
                    .collect();

                let dependent_websites: Vec<_> = srvw::server_website
                    .inner_join(srv::server)
                    .filter(
                        srv::project_id.ne(prj_id).and(
                            srvw::server_database_id
                                .eq_any(contained_dbs.iter().map(|d| d.id).collect::<Vec<_>>()),
                        ),
                    )
                    .load::<(ServerWebsite, Server)>(sql_conn)
                    .unwrap()
                    .into_iter()
                    .map(|x| x.0)
                    .collect();
                if !dependent_serverlinks.is_empty() {
                    s.send(Err((
                        "Cannot delete project",
                        Some(format!(
                            "servers {} on that server are linked to by servers {}",
                            itertools::join(
                                dependent_serverlinks.iter().map(|(_, s)| &s.desc),
                                ", "
                            ),
                            itertools::join(
                                dependent_serverlinks.iter().map(|(l, _)| &l.desc),
                                ", "
                            )
                        )),
                    )))
                } else if !dependent_websites.is_empty() {
                    s.send(Err((
                        "Cannot delete project",
                        Some(format!(
                            "databases {} on that server are linked to by websites {}",
                            itertools::join(
                                dependent_websites.iter().map(|w| &contained_dbs
                                    .iter()
                                    .find(|d| Some(d.id) == w.server_database_id)
                                    .unwrap()
                                    .desc),
                                ", "
                            ),
                            itertools::join(dependent_websites.iter().map(|w| &w.desc), ", ")
                        )),
                    )))
                } else {
                    s.send(
                        sql_util::delete_row(sql_conn, prj::project, prj_id).map(|_| prj.clone()),
                    )
                }
                .unwrap();
            }))
            .unwrap();
    }

    fn handle_project_delete(&self) {
        if let Some(prj) = self.model.project.as_ref() {
            let relm = self.model.relm.clone();
            standard_dialogs::confirm_deletion(
                &format!("Delete {}", prj.name),
                &format!(
                    "Are you sure you want to delete the project {}? This action cannot be undone.",
                    prj.name
                ),
                self.widgets
                    .project_summary_root
                    .clone()
                    .upcast::<gtk::Widget>(),
                move || relm.stream().emit(Msg::DeleteProject),
            );
        }
    }

    fn show_project_edit_dialog(&mut self) {
        let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
            self.widgets
                .project_summary_root
                .clone()
                .upcast::<gtk::Widget>(),
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
        let d_c = dialog_contents.stream().clone();
        let dialog = standard_dialogs::modal_dialog(
            self.widgets
                .project_summary_root
                .clone()
                .upcast::<gtk::Widget>(),
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
            #[name="buttons_box"]
            gtk::Box {
                homogeneous: true,
                margin_start: 5,
                margin_end: 5,
                child: {
                    padding: 5,
                },
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
                    label: "Prod",
                    mode: false,
                    sensitive: false,
                },
            }
        }
    }
}
