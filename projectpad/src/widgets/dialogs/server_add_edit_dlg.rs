use super::dialog_helpers;
use super::file_contents_button::FileContentsButton;
use super::file_contents_button::Msg::FileChanged as AuthFileChanged;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::password_field;
use crate::widgets::password_field::Msg as PasswordFieldMsg;
use crate::widgets::password_field::Msg::PublishPassword as PasswordFieldMsgPublishPassword;
use crate::widgets::password_field::PasswordField;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, Server, ServerAccessType, ServerType};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::str::FromStr;
use std::sync::mpsc;
use strum::IntoEnumIterator;

#[derive(Msg, Debug, Clone)]
pub enum Msg {
    SetEnvironmentType(EnvironmentType),
    GotGroups(Vec<String>),
    AuthFileChanged((Option<String>, Option<Vec<u8>>)),
    OkPressed,
    GotPassword(String),
    ServerUpdated(Server),
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<Server, (String, Option<String>)>;

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    _server_updated_channel: relm::Channel<SaveResult>,
    server_updated_sender: relm::Sender<SaveResult>,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,
    groups_store: gtk::ListStore,
    project_id: i32,
    server_id: Option<i32>,
    environment_type: Option<EnvironmentType>,

    description: String,
    is_retired: bool,
    address: String,
    text: String,
    group_name: Option<String>,
    username: String,
    password: String,
    server_type: ServerType,
    server_access_type: ServerAccessType,
    auth_key_filename: Option<String>,
    // store the auth key & not the Path, because it's what I have
    // when reading from SQL. So by storing it also when adding a new
    // server, I have the same data for add & edit.
    auth_key: Option<Vec<u8>>,
}

#[widget]
impl Widget for ServerAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.widgets.grid);
        self.init_server_type();
        self.init_server_access_type();
        self.init_group();
    }

    fn server_type_desc(server_type: ServerType) -> &'static str {
        match server_type {
            ServerType::SrvApplication => "Application",
            ServerType::SrvDatabase => "Database",
            ServerType::SrvHttpOrProxy => "HTTP server or proxy",
            ServerType::SrvMonitoring => "Monitoring",
            ServerType::SrvReporting => "Reporting",
        }
    }

    fn init_server_type(&self) {
        let mut entries: Vec<_> = ServerType::iter()
            .map(|st| (st, Self::server_type_desc(st)))
            .collect();
        entries.sort_by_key(|p| p.1);
        for (entry_type, entry_desc) in entries {
            self.widgets
                .server_type
                .append(Some(&entry_type.to_string()), entry_desc);
        }
        self.widgets
            .server_type
            .set_active_id(Some(&self.model.server_type.to_string()));
    }

    fn server_access_type_desc(access_type: ServerAccessType) -> &'static str {
        match access_type {
            ServerAccessType::SrvAccessSsh => "SSH",
            ServerAccessType::SrvAccessWww => "Website",
            ServerAccessType::SrvAccessRdp => "Remote Desktop (RDP)",
            ServerAccessType::SrvAccessSshTunnel => "SSH tunnel",
        }
    }

    fn init_server_access_type(&self) {
        let mut entries: Vec<_> = ServerAccessType::iter()
            .map(|at| (at, Self::server_access_type_desc(at)))
            .collect();
        entries.sort_by_key(|p| p.1);
        for (entry_type, entry_desc) in entries {
            self.widgets
                .server_access_type
                .append(Some(&entry_type.to_string()), entry_desc);
        }
        self.widgets
            .server_access_type
            .set_active_id(Some(&self.model.server_access_type.to_string()));
    }

    fn init_group(&self) {
        let s = self.model.groups_sender.clone();
        let pid = self.model.project_id;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                s.send(projectpadsql::get_project_group_names(sql_conn, pid))
                    .unwrap();
            }))
            .unwrap();
        dialog_helpers::init_group_control(&self.model.groups_store, &self.widgets.group);
    }

    // TODO probably could take an Option<&Server> and drop some cloning
    // I take the project_id because I may not get a server to get the
    // project_id from.
    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, i32, Option<Server>, gtk::AccelGroup),
    ) -> Model {
        let (db_sender, project_id, server, _) = params;
        let stream = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        let stream2 = relm.stream().clone();
        let (server_updated_channel, server_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(srv) => stream2.emit(Msg::ServerUpdated(srv)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        let srv = server.as_ref();
        Model {
            environment_type: srv.map(|s| s.environment),
            db_sender,
            _groups_channel: groups_channel,
            groups_sender,
            groups_store: gtk::ListStore::new(&[String::static_type()]),
            _server_updated_channel: server_updated_channel,
            server_updated_sender,
            project_id,
            server_id: srv.map(|s| s.id),
            description: srv
                .map(|s| s.desc.clone())
                .unwrap_or_else(|| "".to_string()),
            is_retired: srv.map(|s| s.is_retired).unwrap_or(false),
            address: srv.map(|s| s.ip.clone()).unwrap_or_else(|| "".to_string()),
            text: srv
                .map(|s| s.text.clone())
                .unwrap_or_else(|| "".to_string()),
            group_name: srv.and_then(|s| s.group_name.clone()),
            username: srv
                .map(|s| s.username.clone())
                .unwrap_or_else(|| "".to_string()),
            password: srv
                .map(|s| s.password.clone())
                .unwrap_or_else(|| "".to_string()),
            server_type: srv
                .map(|s| s.server_type)
                .unwrap_or(ServerType::SrvApplication),
            server_access_type: srv
                .map(|s| s.access_type)
                .unwrap_or(ServerAccessType::SrvAccessSsh),
            auth_key_filename: srv.and_then(|s| s.auth_key_filename.clone()),
            auth_key: srv.and_then(|s| s.auth_key.clone()),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::SetEnvironmentType(env) => self.model.environment_type = Some(env),
            Msg::GotGroups(groups) => {
                dialog_helpers::fill_groups(
                    &self.model.groups_store,
                    &self.widgets.group,
                    &groups,
                    &self.model.group_name,
                );
            }
            Msg::AuthFileChanged(ref kv) => {
                self.model.auth_key_filename = kv.0.clone();
                self.model.auth_key = kv.1.clone();
            }
            Msg::OkPressed => {
                self.streams
                    .password_entry
                    .emit(PasswordFieldMsg::RequestPassword);
            }
            Msg::GotPassword(pass) => {
                self.update_server(pass);
            }
            Msg::ServerUpdated(_) => {} // meant for my parent, not me
        }
    }

    fn update_server(&self, new_password: String) {
        let new_env_type = self.model.environment_type.unwrap();
        let server_id = self.model.server_id;
        let project_id = self.model.project_id;
        let new_desc = self.widgets.desc_entry.text();
        let new_is_retired = self.widgets.is_retired_check.is_active();
        let new_address = self.widgets.address_entry.text();
        let new_text = self.widgets.text_entry.text();
        let new_group = self.widgets.group.active_text();
        let new_username = self.widgets.username_entry.text();
        let new_authkey = self.model.auth_key.clone();
        let new_authkey_filename = self.model.auth_key_filename.clone();
        let new_servertype = self
            .widgets
            .server_type
            .active_id()
            .map(|s| ServerType::from_str(s.as_str()).expect("Error parsing the server type!?"))
            .expect("server type not specified!?");
        let new_server_accesstype = self
            .widgets
            .server_access_type
            .active_id()
            .map(|s| {
                ServerAccessType::from_str(s.as_str())
                    .expect("Error parsing the server access type!?")
            })
            .expect("server access type not specified!?");
        let s = self.model.server_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server::dsl as srv;
                let changeset = (
                    srv::desc.eq(new_desc.as_str()),
                    srv::is_retired.eq(new_is_retired),
                    srv::ip.eq(new_address.as_str()),
                    srv::text.eq(new_text.as_str()),
                    // never store Some("") for group, we want None then.
                    srv::group_name.eq(new_group
                        .as_ref()
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())),
                    srv::username.eq(new_username.as_str()),
                    srv::password.eq(new_password.as_str()),
                    srv::auth_key.eq(new_authkey.as_ref()),
                    srv::auth_key_filename.eq(new_authkey_filename.as_ref()),
                    srv::server_type.eq(new_servertype),
                    srv::access_type.eq(new_server_accesstype),
                    srv::environment.eq(new_env_type),
                    srv::project_id.eq(project_id),
                );
                let server_after_result = perform_insert_or_update!(
                    sql_conn,
                    server_id,
                    srv::server,
                    srv::id,
                    changeset,
                    Server,
                );
                s.send(server_after_result).unwrap();
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
                text: &self.model.description,
                activates_default: true,
                cell: {
                    left_attach: 1,
                    top_attach: 0,
                },
            },
            gtk::Label {
                text: "Is retired",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
            },
            #[name="is_retired_check"]
            gtk::CheckButton {
                label: "",
                active: self.model.is_retired,
                cell: {
                    left_attach: 1,
                    top_attach: 1,
                },
            },
            gtk::Label {
                text: "Address",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                },
            },
            #[name="address_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.address,
                activates_default: true,
                placeholder_text: Some("Address or address:port"),
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
                activates_default: true,
                cell: {
                    left_attach: 1,
                    top_attach: 3,
                },
            },
            gtk::Label {
                text: "Group",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 4,
                },
            },
            #[name="group"]
            gtk::ComboBoxText({has_entry: true}) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 4,
                },
            },
            gtk::Label {
                text: "Username",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 5,
                },
            },
            #[name="username_entry"]
            gtk::Entry {
                hexpand: true,
                activates_default: true,
                text: &self.model.username,
                cell: {
                    left_attach: 1,
                    top_attach: 5,
                },
            },
            gtk::Label {
                text: "Password",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 6,
                },
            },
            #[name="password_entry"]
            PasswordField((self.model.password.clone(), password_field::ActivatesDefault::Yes)) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 6,
                },
                PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPassword(pass.clone())
            },
            gtk::Label {
                text: "Authentication key",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 7,
                },
            },
            FileContentsButton((
                self.model.auth_key_filename.clone(),
                self.model.auth_key.clone(),
                None,
            )) {
                AuthFileChanged(ref val) => Msg::AuthFileChanged(val.clone()),
                cell: {
                    left_attach: 1,
                    top_attach: 7,
                },
            },
            gtk::Label {
                text: "Server type",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 8,
                },
            },
            #[name="server_type"]
            gtk::ComboBoxText {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 8,
                },
            },
            gtk::Label {
                text: "Access type",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 9,
                },
            },
            #[name="server_access_type"]
            gtk::ComboBoxText {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 9,
                },
            },
        }
    }
}
