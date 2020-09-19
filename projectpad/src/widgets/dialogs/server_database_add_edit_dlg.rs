use super::dialog_helpers;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::password_field::Msg as PasswordFieldMsg;
use crate::widgets::password_field::Msg::PublishPassword as PasswordFieldMsgPublishPassword;
use crate::widgets::password_field::PasswordField;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::ServerDatabase;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    GotGroups(Vec<String>),
    OkPressed,
    GotPassword(String),
    ServerDbUpdated(ServerDatabase),
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<ServerDatabase, (String, Option<String>)>;

pub struct Model {
    relm: relm::Relm<ServerDatabaseAddEditDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    server_db_id: Option<i32>,

    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,

    _server_db_updated_channel: relm::Channel<SaveResult>,
    server_db_updated_sender: relm::Sender<SaveResult>,

    description: String,
    name: String,
    group_name: Option<String>,
    text: String,
    username: String,
    password: String,
}

pub const SERVER_DATABASE_ADD_EDIT_WIDTH: i32 = 600;
pub const SERVER_DATABASE_ADD_EDIT_HEIGHT: i32 = 200;

#[widget]
impl Widget for ServerDatabaseAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.root);
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
        params: (
            mpsc::Sender<SqlFunc>,
            i32,
            Option<ServerDatabase>,
            gtk::AccelGroup,
        ),
    ) -> Model {
        let (db_sender, server_id, server_db, _) = params;
        let sd = server_db.as_ref();
        let stream = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        let stream2 = relm.stream().clone();
        let (server_db_updated_channel, server_db_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(srv_db) => stream2.emit(Msg::ServerDbUpdated(srv_db)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        Model {
            relm: relm.clone(),
            db_sender,
            server_id,
            server_db_id: sd.map(|d| d.id),
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            _groups_channel: groups_channel,
            groups_sender,
            _server_db_updated_channel: server_db_updated_channel,
            server_db_updated_sender,
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
                self.password_entry
                    .stream()
                    .emit(PasswordFieldMsg::RequestPassword);
            }
            Msg::GotPassword(pass) => {
                self.update_server_db(pass);
            }
            // meant for my parent
            Msg::ServerDbUpdated(_) => {}
        }
    }

    fn update_server_db(&self, new_password: String) {
        let server_id = self.model.server_id;
        let server_db_id = self.model.server_db_id;
        let new_desc = self.desc_entry.get_text();
        let new_name = self.name_entry.get_text();
        let new_group = self.group.get_active_text();
        let new_text = self.text_entry.get_text();
        let new_username = self.username_entry.get_text();
        let s = self.model.server_db_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_database::dsl as srv_db;
                let changeset = (
                    srv_db::desc.eq(new_desc.as_str()),
                    srv_db::name.eq(new_name.as_str()),
                    // never store Some("") for group, we want None then.
                    srv_db::group_name.eq(new_group
                        .as_ref()
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())),
                    srv_db::text.eq(new_text.as_str()),
                    srv_db::username.eq(new_username.as_str()),
                    srv_db::password.eq(new_password.as_str()),
                    srv_db::server_id.eq(server_id),
                );
                let server_db_after_result = perform_insert_or_update!(
                    sql_conn,
                    server_db_id,
                    srv_db::server_database,
                    srv_db::id,
                    changeset,
                    ServerDatabase,
                );
                s.send(server_db_after_result).unwrap();
            }))
            .unwrap();
    }

    view! {
        #[name="root"]
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
            PasswordField(self.model.password.clone()) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 5,
                },
                PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPassword(pass.clone())
            },
        }
    }
}
