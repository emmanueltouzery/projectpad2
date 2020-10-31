use super::dialog_helpers;
use super::environments_picker;
use super::environments_picker::EnvironmentsPicker;
use super::environments_picker::Msg::EnvironmentToggled as EnvironmentsPickerMsgEnvToggled;
use super::note_edit;
use super::note_edit::Msg::PublishContents as NotePublishContents;
use super::note_edit::NoteEdit;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, Project, ProjectNote};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    GotGroups(Vec<String>),
    OkPressed,
    UpdateProjectNote(String),
    ProjectNoteUpdated(ProjectNote),
    GotProjectEnvironments(environments_picker::SelectedEnvironments),
    EnvironmentToggled(EnvironmentType),
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<ProjectNote, (String, Option<String>)>;

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    accel_group: gtk::AccelGroup,
    project_id: i32,
    project_note_id: Option<i32>,

    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,

    _project_environments_channel: relm::Channel<environments_picker::SelectedEnvironments>,
    project_environments_sender: relm::Sender<environments_picker::SelectedEnvironments>,

    _project_note_updated_channel: relm::Channel<SaveResult>,
    project_note_updated_sender: relm::Sender<SaveResult>,

    title: String,
    has_dev: bool,
    has_stg: bool,
    has_uat: bool,
    has_prod: bool,
    group_name: Option<String>,
    project_environments: Option<environments_picker::SelectedEnvironments>,
    contents: String,
}

#[widget]
impl Widget for ProjectNoteAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
        self.init_group();
        self.fetch_project_environments();
        self.grid.set_property_width_request(700);
        self.grid.set_property_height_request(500);

        let no_envs_error_label = gtk::LabelBuilder::new()
            .label("You must select at least one environment which is active on the parent project")
            .build();
        no_envs_error_label.show();
        self.no_envs_error
            .get_content_area()
            .add(&no_envs_error_label);
    }

    fn init_group(&self) {
        let s = self.model.groups_sender.clone();
        let pid = self.model.project_id;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                s.send(dialog_helpers::get_project_group_names(sql_conn, pid))
                    .unwrap();
            }))
            .unwrap();
        dialog_helpers::init_group_control(&self.model.groups_store, &self.group);
    }

    fn fetch_project_environments(&self) {
        let s = self.model.project_environments_sender.clone();
        let pid = self.model.project_id;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                let project: Project = prj::project.find(pid).first(sql_conn).unwrap();
                s.send(environments_picker::SelectedEnvironments {
                    has_dev: project.has_dev,
                    has_stg: project.has_stage,
                    has_uat: project.has_uat,
                    has_prod: project.has_prod,
                })
                .unwrap();
            }))
            .unwrap();
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (
            mpsc::Sender<SqlFunc>,
            i32,
            Option<ProjectNote>,
            gtk::AccelGroup,
        ),
    ) -> Model {
        let (db_sender, project_id, project_note, accel_group) = params;
        let pn = project_note.as_ref();
        let stream = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        let stream2 = relm.stream().clone();
        let (project_note_updated_channel, project_note_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(srv_note) => stream2.emit(Msg::ProjectNoteUpdated(srv_note)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        let stream3 = relm.stream().clone();
        let (project_environments_channel, project_environments_sender) = relm::Channel::new(
            move |project_environments: environments_picker::SelectedEnvironments| {
                stream3.emit(Msg::GotProjectEnvironments(project_environments));
            },
        );
        Model {
            db_sender,
            accel_group,
            project_id,
            project_note_id: pn.map(|d| d.id),
            _groups_channel: groups_channel,
            groups_sender,
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            _project_note_updated_channel: project_note_updated_channel,
            project_note_updated_sender,
            _project_environments_channel: project_environments_channel,
            project_environments_sender,
            has_dev: pn.map(|d| d.has_dev).unwrap_or(false),
            has_stg: pn.map(|d| d.has_stage).unwrap_or(false),
            has_uat: pn.map(|d| d.has_uat).unwrap_or(false),
            has_prod: pn.map(|d| d.has_prod).unwrap_or(false),
            title: pn
                .map(|d| d.title.clone())
                .unwrap_or_else(|| "".to_string()),
            project_environments: None,
            contents: pn
                .map(|d| d.contents.clone())
                .unwrap_or_else(|| "".to_string()),
            group_name: pn.and_then(|s| s.group_name.clone()),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::GotGroups(groups) => {
                dialog_helpers::fill_groups(
                    &self.model.groups_store,
                    &self.group,
                    &groups,
                    &self.model.group_name,
                );
            }
            Msg::GotProjectEnvironments(prj_envs) => {
                self.model.project_environments = Some(prj_envs);
            }
            Msg::OkPressed => {
                self.note_edit
                    .stream()
                    .emit(note_edit::Msg::RequestContents);
            }
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
            Msg::UpdateProjectNote(new_contents) => {
                self.update_project_note(new_contents);
            }
            // for my parent
            Msg::ProjectNoteUpdated(_) => {}
        }
    }

    fn update_project_note(&self, new_contents: String) {
        let new_has_dev = self.model.has_dev;
        let new_has_stg = self.model.has_stg;
        let new_has_uat = self.model.has_uat;
        let new_has_prod = self.model.has_prod;

        let has_envs = if let Some(prj_envs) = &self.model.project_environments {
            (prj_envs.has_dev && new_has_dev)
                || (prj_envs.has_stg && new_has_stg)
                || (prj_envs.has_uat && new_has_uat)
                || (prj_envs.has_prod && new_has_prod)
        } else {
            false
        };
        if !has_envs {
            self.no_envs_error.set_visible(true);
            return;
        }

        let project_id = self.model.project_id;
        let project_note_id = self.model.project_note_id;
        let new_title = self.title_entry.get_text();
        let new_group = self.group.get_active_text();
        let s = self.model.project_note_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project_note::dsl as prj_note;
                let changeset = (
                    prj_note::title.eq(new_title.as_str()),
                    // never store Some("") for group, we want None then.
                    prj_note::group_name.eq(new_group
                        .as_ref()
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())),
                    prj_note::contents.eq(new_contents.as_str()),
                    prj_note::has_dev.eq(new_has_dev),
                    prj_note::has_stage.eq(new_has_stg),
                    prj_note::has_uat.eq(new_has_uat),
                    prj_note::has_prod.eq(new_has_prod),
                    prj_note::project_id.eq(project_id),
                );
                let project_note_after_result = perform_insert_or_update!(
                    sql_conn,
                    project_note_id,
                    prj_note::project_note,
                    prj_note::id,
                    changeset,
                    ProjectNote,
                );
                s.send(project_note_after_result).unwrap();
            }))
            .unwrap();
    }

    view! {
        #[name="grid"]
        gtk::Grid {
            #[name="no_envs_error"]
            gtk::InfoBar {
                message_type: gtk::MessageType::Error,
                cell: {
                    left_attach: 0,
                    top_attach: 0,
                    width: 2,
                },
                visible: false,
            },
            gtk::Label {
                text: "Title",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
            },
            #[name="title_entry"]
            gtk::Entry {
                hexpand: true,
                activates_default: true,
                text: &self.model.title,
                cell: {
                    left_attach: 1,
                    top_attach: 1,
                },
            },
            gtk::Label {
                text: "Group",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                },
            },
            #[name="group"]
            gtk::ComboBoxText({has_entry: true}) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 2,
                },
            },
            EnvironmentsPicker(environments_picker::SelectedEnvironments {
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
            #[name="note_edit"]
            NoteEdit((self.model.contents.clone(), self.model.accel_group.clone())) {
                cell: {
                    left_attach: 0,
                    top_attach: 4,
                    width: 2,
                },
                NotePublishContents(ref contents) => Msg::UpdateProjectNote(contents.clone())
            }
        }
    }
}
