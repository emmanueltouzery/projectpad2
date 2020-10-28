use super::dialog_helpers;
use super::pick_projectpad_item_button;
use super::pick_projectpad_item_button::Msg::ItemSelected as PickPpItemSelected;
use super::pick_projectpad_item_button::Msg::RemoveItem as PickPpItemRemoved;
use super::pick_projectpad_item_button::{PickProjectpadItemButton, PickProjectpadItemParams};
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::password_field;
use crate::widgets::password_field::Msg as PasswordFieldMsg;
use crate::widgets::password_field::Msg::PublishPassword as PasswordFieldMsgPublishPassword;
use crate::widgets::password_field::PasswordField;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{ServerDatabase, ServerWebsite};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    GotGroups(Vec<String>),
    GotProjectNameAndId((String, i32)),
    ServerDbSelected(i32),
    ServerDbRemoved,
    OkPressed,
    GotPassword(String),
    ServerWwwUpdated((ServerWebsite, Option<ServerDatabase>)),
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<(ServerWebsite, Option<ServerDatabase>), (String, Option<String>)>;

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    server_www_id: Option<i32>,

    _projectname_id_channel: relm::Channel<(String, i32)>,
    projectname_id_sender: relm::Sender<(String, i32)>,

    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,

    _server_www_updated_channel: relm::Channel<SaveResult>,
    server_www_updated_sender: relm::Sender<SaveResult>,

    server_database_id: Option<i32>,
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
        self.fetch_project_name_and_id();
    }

    fn init_group(&self) {
        dialog_helpers::init_group_control(&self.model.groups_store, &self.group);
        dialog_helpers::fetch_server_groups(
            &self.model.groups_sender,
            self.model.server_id,
            &self.model.db_sender,
        );
    }

    fn fetch_project_name_and_id(&self) {
        let s = self.model.projectname_id_sender.clone();
        let server_id = self.model.server_id;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                use projectpadsql::schema::server::dsl as srv;
                let data = srv::server
                    .inner_join(prj::project)
                    .select((prj::name, prj::id))
                    .filter(srv::id.eq(server_id))
                    .first::<(String, i32)>(sql_conn)
                    .unwrap();
                s.send(data).unwrap();
            }))
            .unwrap();
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (
            mpsc::Sender<SqlFunc>,
            i32,
            Option<ServerWebsite>,
            gtk::AccelGroup,
        ),
    ) -> Model {
        let (db_sender, server_id, server_www, _) = params;
        let sw = server_www.as_ref();
        let stream = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        let stream2 = relm.stream().clone();
        let (server_www_updated_channel, server_www_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(srv_www) => stream2.emit(Msg::ServerWwwUpdated(srv_www)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        let stream3 = relm.stream().clone();
        let (projectname_id_channel, projectname_id_sender) =
            relm::Channel::new(move |projectname_id: (String, i32)| {
                stream3.emit(Msg::GotProjectNameAndId(projectname_id));
            });
        Model {
            db_sender,
            server_id,
            server_www_id: sw.map(|d| d.id),
            projectname_id_sender,
            _projectname_id_channel: projectname_id_channel,
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            _groups_channel: groups_channel,
            groups_sender,
            _server_www_updated_channel: server_www_updated_channel,
            server_www_updated_sender,
            server_database_id: sw.and_then(|d| d.server_database_id),
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
            Msg::GotProjectNameAndId((name, id)) => {
                self.pick_db_button.stream().emit(
                    pick_projectpad_item_button::Msg::SetProjectNameAndId(Some((name, id))),
                );
            }
            Msg::ServerDbSelected(db_id) => {
                self.model.server_database_id = Some(db_id);
            }
            Msg::ServerDbRemoved => {
                self.model.server_database_id = None;
            }
            Msg::OkPressed => {
                self.password_entry
                    .stream()
                    .emit(PasswordFieldMsg::RequestPassword);
            }
            Msg::GotPassword(pass) => {
                self.update_server_www(pass);
            }
            // meant for my parent
            Msg::ServerWwwUpdated(_) => {}
        }
    }

    fn update_server_www(&self, new_password: String) {
        let server_id = self.model.server_id;
        let server_www_id = self.model.server_www_id;
        let new_desc = self.desc_entry.get_text();
        let new_url = self.url_entry.get_text();
        let new_text = self.text_entry.get_text();
        let new_group = self.group.get_active_text();
        let new_username = self.username_entry.get_text();
        let new_databaseid = self.model.server_database_id;
        let s = self.model.server_www_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_database::dsl as srv_db;
                use projectpadsql::schema::server_website::dsl as srv_www;
                let changeset = (
                    srv_www::desc.eq(new_desc.as_str()),
                    srv_www::url.eq(new_url.as_str()),
                    srv_www::text.eq(new_text.as_str()),
                    // never store Some("") for group, we want None then.
                    srv_www::group_name.eq(new_group
                        .as_ref()
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())),
                    srv_www::username.eq(new_username.as_str()),
                    srv_www::password.eq(new_password.as_str()),
                    srv_www::server_database_id.eq(new_databaseid),
                    srv_www::server_id.eq(server_id),
                );
                let server_www_after_result = perform_insert_or_update!(
                    sql_conn,
                    server_www_id,
                    srv_www::server_website,
                    srv_www::id,
                    changeset,
                    ServerWebsite,
                );
                let server_db = server_www_after_result
                    .as_ref()
                    .ok()
                    .and_then(|www| www.server_database_id)
                    .and_then(|db_id| srv_db::server_database.find(db_id).first(sql_conn).ok());
                s.send(server_www_after_result.map(|s| (s, server_db)))
                    .unwrap();
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
                text: &self.model.url,
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
            PasswordField((self.model.password.clone(), password_field::ActivatesDefault::No)) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 5,
                },
                PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPassword(pass.clone())
            },
            gtk::Label {
                text: "Database",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 6,
                },
            },
            #[name="pick_db_button"]
            PickProjectpadItemButton(PickProjectpadItemParams {
                db_sender: self.model.db_sender.clone(),
                item_type: pick_projectpad_item_button::ItemType::ServerDatabase,
                item_id: self.model.server_database_id,
                project_name_id: None, // we get the project name later through a message
            }) {
                cell: {
                    left_attach: 1,
                    top_attach: 6,
                },
                PickPpItemSelected(ref v) => Msg::ServerDbSelected(v.1),
                PickPpItemRemoved => Msg::ServerDbRemoved
            }
        }
    }
}
