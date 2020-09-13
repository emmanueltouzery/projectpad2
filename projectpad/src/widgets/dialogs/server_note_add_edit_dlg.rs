use super::dialog_helpers;
use super::note_edit;
use super::note_edit::Msg::PublishContents as NotePublishContents;
use super::note_edit::NoteEdit;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::ServerNote;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    GotGroups(Vec<String>),
    OkPressed,
    UpdateServerNote(String),
    ServerNoteUpdated(ServerNote),
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<ServerNote, (String, Option<String>)>;

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    accel_group: gtk::AccelGroup,
    server_id: i32,
    server_note_id: Option<i32>,

    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,

    _server_note_updated_channel: relm::Channel<SaveResult>,
    server_note_updated_sender: relm::Sender<SaveResult>,

    title: String,
    group_name: Option<String>,
    contents: String,
}

#[widget]
impl Widget for ServerNoteAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
        self.init_group();
        self.grid.set_property_width_request(700);
        self.grid.set_property_height_request(500);
    }

    fn init_group(&self) {
        dialog_helpers::init_group_control(&self.model.groups_store, &self.group);
        dialog_helpers::fetch_server_groups(
            &self.model.groups_sender,
            self.model.server_id,
            &self.model.db_sender,
        );
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (
            mpsc::Sender<SqlFunc>,
            i32,
            Option<ServerNote>,
            gtk::AccelGroup,
        ),
    ) -> Model {
        let (db_sender, server_id, server_note, accel_group) = params;
        let sn = server_note.as_ref();
        let stream = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        let stream2 = relm.stream().clone();
        let (server_note_updated_channel, server_note_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(srv_note) => stream2.emit(Msg::ServerNoteUpdated(srv_note)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        Model {
            db_sender,
            accel_group,
            server_id,
            server_note_id: sn.map(|d| d.id),
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            _groups_channel: groups_channel,
            groups_sender,
            _server_note_updated_channel: server_note_updated_channel,
            server_note_updated_sender,
            title: sn
                .map(|d| d.title.clone())
                .unwrap_or_else(|| "".to_string()),
            contents: sn
                .map(|d| d.contents.clone())
                .unwrap_or_else(|| "".to_string()),
            group_name: sn.and_then(|s| s.group_name.clone()),
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
            Msg::OkPressed => {
                self.note_edit
                    .stream()
                    .emit(note_edit::Msg::RequestContents);
            }
            Msg::UpdateServerNote(new_contents) => {
                self.update_server_note(new_contents);
            }
            // meant for my parent
            Msg::ServerNoteUpdated(_) => {}
        }
    }

    fn update_server_note(&self, new_contents: String) {
        let server_id = self.model.server_id;
        let server_note_id = self.model.server_note_id;
        let new_title = self.title_entry.get_text();
        let new_group = self.group.get_active_text();
        let s = self.model.server_note_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_note::dsl as srv_note;
                let changeset = (
                    srv_note::title.eq(new_title.as_str()),
                    // never store Some("") for group, we want None then.
                    srv_note::group_name.eq(new_group
                        .as_ref()
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())),
                    srv_note::contents.eq(new_contents.as_str()),
                    srv_note::server_id.eq(server_id),
                );
                let server_note_after_result = perform_insert_or_update!(
                    sql_conn,
                    server_note_id,
                    srv_note::server_note,
                    srv_note::id,
                    changeset,
                    ServerNote,
                );
                s.send(server_note_after_result).unwrap();
            }))
            .unwrap();
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
            #[name="note_edit"]
            NoteEdit((self.model.contents.clone(), self.model.accel_group.clone())) {
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                    width: 2,
                },
                NotePublishContents(ref contents) => Msg::UpdateServerNote(contents.clone())
            }
        }
    }
}
