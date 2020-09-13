use super::dialog_helpers;
use super::environments_picker;
use super::environments_picker::EnvironmentsPicker;
use super::note_edit;
use super::note_edit::Msg::PublishContents as NotePublishContents;
use super::note_edit::NoteEdit;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::ProjectNote;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    GotGroups(Vec<String>),
    OkPressed,
    UpdateProjectNote(String),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    accel_group: gtk::AccelGroup,
    project_id: i32,

    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,

    title: String,
    group_name: Option<String>,
    selected_environments: environments_picker::SelectedEnvironments,
    contents: String,
}

#[widget]
impl Widget for ProjectNoteAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
        self.init_group();
        self.grid.set_property_width_request(700);
        self.grid.set_property_height_request(500);
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
        let selected_environments = environments_picker::SelectedEnvironments {
            has_dev: pn.map(|d| d.has_dev).unwrap_or(false),
            has_stg: pn.map(|d| d.has_stage).unwrap_or(false),
            has_uat: pn.map(|d| d.has_uat).unwrap_or(false),
            has_prod: pn.map(|d| d.has_prod).unwrap_or(false),
        };
        Model {
            db_sender,
            accel_group,
            project_id,
            _groups_channel: groups_channel,
            groups_sender,
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            title: pn
                .map(|d| d.title.clone())
                .unwrap_or_else(|| "".to_string()),
            selected_environments,
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
            Msg::OkPressed => {}
            Msg::UpdateProjectNote(new_contents) => {}
        }
    }

    view! {
        #[name="grid"]
        gtk::Grid {
            gtk::Label {
                text: "Title",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 0,
                },
            },
            #[name="title_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.title,
                cell: {
                    left_attach: 1,
                    top_attach: 0,
                },
            },
            gtk::Label {
                text: "Group",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
            },
            #[name="group"]
            gtk::ComboBoxText({has_entry: true}) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 1,
                },
            },
            EnvironmentsPicker(self.model.selected_environments.clone()) {
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                    width: 2,
                },
            },
            #[name="note_edit"]
            NoteEdit((self.model.contents.clone(), self.model.accel_group.clone())) {
                cell: {
                    left_attach: 0,
                    top_attach: 3,
                    width: 2,
                },
                NotePublishContents(ref contents) => Msg::UpdateProjectNote(contents.clone())
            }
        }
    }
}
