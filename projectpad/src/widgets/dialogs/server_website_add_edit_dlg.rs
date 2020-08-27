use super::dialog_helpers;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::ServerWebsite;
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

    description: String,
    url: String,
    text: String,
    group_name: Option<String>,
    username: String,
    password: String,
}

#[widget]
impl Widget for ServerWebsiteAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
        self.init_group();
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
        params: (mpsc::Sender<SqlFunc>, i32, Option<ServerWebsite>),
    ) -> Model {
        let (db_sender, server_id, server_www) = params;
        let sw = server_www.as_ref();
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
            description: sw.map(|d| d.desc.clone()).unwrap_or_else(|| "".to_string()),
            url: sw.map(|d| d.url.clone()).unwrap_or_else(|| "".to_string()),
            text: sw.map(|d| d.text.clone()).unwrap_or_else(|| "".to_string()),
            group_name: sw.and_then(|s| s.group_name.clone()),
            username: sw
                .map(|d| d.username.clone())
                .unwrap_or_else(|| "".to_string()),
            password: sw
                .map(|d| d.password.clone())
                .unwrap_or_else(|| "".to_string()),
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
        }
    }

    view! {
        #[name="grid"]
        gtk::Grid {
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
                text: "URL",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
            },
            #[name="url_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.description,
                cell: {
                    left_attach: 1,
                    top_attach: 1,
                },
            },
            gtk::Label {
                text: "Text",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                },
            },
            #[name="text_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.text,
                cell: {
                    left_attach: 1,
                    top_attach: 2,
                },
            },
            gtk::Label {
                text: "Group",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 3,
                },
            },
            #[name="group"]
            gtk::ComboBoxText({has_entry: true}) {
                hexpand: true,
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
            // TODO database
        }
    }
}
