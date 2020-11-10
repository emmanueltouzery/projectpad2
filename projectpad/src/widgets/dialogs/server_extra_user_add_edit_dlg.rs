use super::dialog_helpers;
use super::file_contents_button::FileContentsButton;
use super::file_contents_button::Msg::FileChanged as FileContentsButtonFileChanged;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::password_field;
use crate::widgets::password_field::Msg as PasswordFieldMsg;
use crate::widgets::password_field::Msg::PublishPassword as PasswordFieldMsgPublishPassword;
use crate::widgets::password_field::PasswordField;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::ServerExtraUserAccount;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    GotGroups(Vec<String>),
    AuthFileChanged((Option<String>, Option<Vec<u8>>)),
    OkPressed,
    GotPassword(String),
    ServerUserUpdated(ServerExtraUserAccount),
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<ServerExtraUserAccount, (String, Option<String>)>;

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    server_user_id: Option<i32>,

    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,

    _server_user_updated_channel: relm::Channel<SaveResult>,
    server_user_updated_sender: relm::Sender<SaveResult>,

    description: String,
    group_name: Option<String>,
    username: String,
    password: String,
    auth_key_filename: Option<String>,
    // store the auth key & not the Path, because it's what I have
    // when reading from SQL. So by storing it also when adding a new
    // server, I have the same data for add & edit.
    auth_key: Option<Vec<u8>>,
}

#[widget]
impl Widget for ServerExtraUserAddEditDialog {
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
        params: (
            mpsc::Sender<SqlFunc>,
            i32,
            Option<ServerExtraUserAccount>,
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
        let (server_user_updated_channel, server_user_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(srv_db) => stream2.emit(Msg::ServerUserUpdated(srv_db)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        Model {
            db_sender,
            server_id,
            server_user_id: sd.map(|d| d.id),
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            _groups_channel: groups_channel,
            groups_sender,
            _server_user_updated_channel: server_user_updated_channel,
            server_user_updated_sender,
            description: sd.map(|d| d.desc.clone()).unwrap_or_else(|| "".to_string()),
            group_name: sd.and_then(|s| s.group_name.clone()),
            username: sd
                .map(|d| d.username.clone())
                .unwrap_or_else(|| "".to_string()),
            password: sd
                .map(|d| d.password.clone())
                .unwrap_or_else(|| "".to_string()),
            auth_key_filename: sd.and_then(|s| s.auth_key_filename.clone()),
            auth_key: sd.and_then(|s| s.auth_key.clone()),
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
            Msg::AuthFileChanged(ref kv) => {
                self.model.auth_key_filename = kv.0.clone();
                self.model.auth_key = kv.1.clone();
            }
            Msg::OkPressed => {
                self.password_entry
                    .stream()
                    .emit(PasswordFieldMsg::RequestPassword);
            }
            Msg::GotPassword(pass) => {
                self.update_server_user(pass);
            }
            // meant for my parent
            Msg::ServerUserUpdated(_) => {}
        }
    }

    fn update_server_user(&self, new_password: String) {
        let server_id = self.model.server_id;
        let server_user_id = self.model.server_user_id;
        let new_desc = self.desc_entry.get_text();
        let new_group = self.group.get_active_text();
        let new_username = self.username_entry.get_text();
        let new_authkey = self.model.auth_key.clone();
        let new_authkey_filename = self.model.auth_key_filename.clone();
        let s = self.model.server_user_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
                let changeset = (
                    srv_usr::desc.eq(new_desc.as_str()),
                    // never store Some("") for group, we want None then.
                    srv_usr::group_name.eq(new_group
                        .as_ref()
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())),
                    srv_usr::username.eq(new_username.as_str()),
                    srv_usr::password.eq(new_password.as_str()),
                    srv_usr::auth_key.eq(new_authkey.as_ref()),
                    srv_usr::auth_key_filename.eq(new_authkey_filename.as_ref()),
                    srv_usr::server_id.eq(server_id),
                );
                let server_db_after_result = perform_insert_or_update!(
                    sql_conn,
                    server_user_id,
                    srv_usr::server_extra_user_account,
                    srv_usr::id,
                    changeset,
                    ServerExtraUserAccount,
                );
                s.send(server_db_after_result).unwrap();
            }))
            .unwrap();
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
                activates_default: true,
                text: &self.model.description,
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
                activates_default: true,
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
            PasswordField((self.model.password.clone(), password_field::ActivatesDefault::Yes)) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 5,
                },
                PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPassword(pass.clone())
            },
            gtk::Label {
                text: "Authentication key",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 6,
                },
            },
            FileContentsButton((
                self.model.auth_key_filename.clone(),
                self.model.auth_key.clone(),
                None,
            )) {
                FileContentsButtonFileChanged(ref val) => Msg::AuthFileChanged(val.clone()),
                cell: {
                    left_attach: 1,
                    top_attach: 6,
                },
            },
        }
    }
}
