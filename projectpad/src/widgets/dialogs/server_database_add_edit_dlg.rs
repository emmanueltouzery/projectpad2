use super::dialog_helpers;
use super::server_poi_add_edit_dlg::fetch_server_groups;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::ServerDatabase;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    GotGroups(Vec<String>),
}

pub struct Model {
    relm: relm::Relm<ServerDatabaseAddEditDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,

    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,

    description: String,
    name: String,
    group_name: Option<String>,
    text: String,
    username: String,
    password: String,
}

#[widget]
impl Widget for ServerDatabaseAddEditDialog {
    fn init_view(&mut self) {
        self.init_group();
    }

    fn init_group(&self) {
        dialog_helpers::init_group_control(&self.model.groups_store, &self.group);
        fetch_server_groups(
            &self.model.groups_sender,
            self.model.server_id,
            &self.model.db_sender,
        );
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, i32, Option<ServerDatabase>),
    ) -> Model {
        let (db_sender, server_id, server_db) = params;
        let sd = server_db.as_ref();
        let stream = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        Model {
            relm: relm.clone(),
            db_sender,
            server_id,
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            _groups_channel: groups_channel,
            groups_sender,
            description: sd.map(|d| d.desc.clone()).unwrap_or_else(|| "".to_string()),
            name: sd.map(|d| d.name.clone()).unwrap_or_else(|| "".to_string()),
            group_name: sd.and_then(|s| s.group_name.clone()),
            text: sd.map(|d| d.text.clone()).unwrap_or_else(|| "".to_string()),
            username: sd
                .map(|d| d.username.clone())
                .unwrap_or_else(|| "".to_string()),
            password: sd
                .map(|d| d.password.clone())
                .unwrap_or_else(|| "".to_string()),
        }
    }

    fn update(&mut self, event: Msg) {}

    view! {
        gtk::Grid {
            margin_start: 30,
            margin_end: 30,
            margin_top: 10,
            margin_bottom: 5,
            row_spacing: 5,
            column_spacing: 10,
            gtk::Label {
                text: "Description",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 0,
                },
            },
            #[name="desc_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.description,
                cell: {
                    left_attach: 1,
                    top_attach: 0,
                },
            },
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
            gtk::Label {
                text: "Text",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 3,
                },
            },
            #[name="text_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.text,
                cell: {
                    left_attach: 1,
                    top_attach: 3,
                },
            },
            gtk::Label {
                text: "Username",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 4,
                },
            },
            #[name="username_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.username,
                cell: {
                    left_attach: 1,
                    top_attach: 4,
                },
            },
            gtk::Label {
                text: "Password",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 5,
                },
            },
            #[name="password_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.password,
                visibility: false,
                input_purpose: gtk::InputPurpose::Password,
                cell: {
                    left_attach: 1,
                    top_attach: 5,
                },
            },
        }
    }
}
