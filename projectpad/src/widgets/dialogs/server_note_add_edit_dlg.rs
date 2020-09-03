use super::dialog_helpers;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::ServerNote;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    GotGroups(Vec<String>),
    OkPressed,
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,

    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,

    title: String,
    group_name: Option<String>,
    contents: String,
}

#[widget]
impl Widget for ServerNoteAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
        self.init_group();
        self.note_textview
            .get_buffer()
            .unwrap()
            .set_text(&self.model.contents);
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
        params: (mpsc::Sender<SqlFunc>, i32, Option<ServerNote>),
    ) -> Model {
        let (db_sender, server_id, server_note) = params;
        let sn = server_note.as_ref();
        let stream = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        Model {
            db_sender,
            server_id,
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            _groups_channel: groups_channel,
            groups_sender,
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
                // TODO
            }
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
            gtk::ScrolledWindow {
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                    width: 2,
                },
                hexpand: true,
                vexpand: true,
                #[name="note_textview"]
                gtk::TextView {
                    margin_top: 10,
                    margin_start: 10,
                    margin_end: 10,
                    margin_bottom: 10,
                    editable: true,
                }
            }
        }
    }
}
