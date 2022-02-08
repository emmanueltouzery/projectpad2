use super::dialog_helpers;
use super::pick_projectpad_item_button;
use super::pick_projectpad_item_button::Msg::ItemSelected as PickPpItemSelected;
use super::pick_projectpad_item_button::Msg::RemoveItem as PickPpItemRemoved;
use super::pick_projectpad_item_button::{PickProjectpadItemButton, PickProjectpadItemParams};
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, ServerLink};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    SetEnvironmentType(EnvironmentType),
    GotGroups(Vec<String>),
    GotLinkedGroups(Vec<String>),
    GotProjectNameAndId((String, i32)),
    ServerSelected(i32),
    ServerRemoved,
    OkPressed,
    ServerLinkUpdated(ServerLink),
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<ServerLink, (String, Option<String>)>;

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    project_id: i32,
    server_link_id: Option<i32>,
    environment_type: Option<EnvironmentType>,

    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,

    linked_groups_store: gtk::ListStore,
    _linked_groups_channel: relm::Channel<Vec<String>>,
    linked_groups_sender: relm::Sender<Vec<String>>,

    _projectname_id_channel: relm::Channel<(String, i32)>,
    projectname_id_sender: relm::Sender<(String, i32)>,

    _server_link_updated_channel: relm::Channel<SaveResult>,
    server_link_updated_sender: relm::Sender<SaveResult>,

    description: String,
    group_name: Option<String>,
    linked_server_id: Option<i32>,
    linked_group_name: Option<String>,
}

#[widget]
impl Widget for ServerLinkAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.widgets.grid);
        self.init_group();
        self.fetch_project_name_and_id();

        let must_pick_server_error_label = gtk::builders::LabelBuilder::new()
            .label("You must select a server to link to")
            .build();
        must_pick_server_error_label.show();
        self.widgets
            .must_pick_server_error
            .content_area()
            .add(&must_pick_server_error_label);
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

        self.reload_linked_groups();
        dialog_helpers::init_group_control(
            &self.model.linked_groups_store,
            &self.widgets.linked_group,
        );
    }

    fn reload_linked_groups(&self) {
        let s = self.model.linked_groups_sender.clone();
        match self.model.linked_server_id {
            Some(server_id) => {
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        s.send(projectpadsql::get_server_group_names(sql_conn, server_id))
                            .unwrap();
                    }))
                    .unwrap();
            }
            None => {
                s.send(vec![]).unwrap();
            }
        }
    }

    fn fetch_project_name_and_id(&self) {
        let s = self.model.projectname_id_sender.clone();
        let project_id = self.model.project_id;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                let data = prj::project
                    .select((prj::name, prj::id))
                    .find(project_id)
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
            Option<ServerLink>,
            gtk::AccelGroup,
        ),
    ) -> Model {
        let (db_sender, project_id, server_link, _accel_group) = params;
        let sl = server_link.as_ref();
        let stream = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        let stream2 = relm.stream().clone();
        let (server_link_updated_channel, server_link_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(srv_link) => stream2.emit(Msg::ServerLinkUpdated(srv_link)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        let stream3 = relm.stream().clone();
        let (projectname_id_channel, projectname_id_sender) =
            relm::Channel::new(move |projectname_id: (String, i32)| {
                stream3.emit(Msg::GotProjectNameAndId(projectname_id));
            });
        let stream4 = relm.stream().clone();
        let (linked_groups_channel, linked_groups_sender) =
            relm::Channel::new(move |groups: Vec<String>| {
                stream4.emit(Msg::GotLinkedGroups(groups));
            });
        Model {
            db_sender,
            project_id,
            environment_type: sl.map(|s| s.environment),
            server_link_id: sl.map(|s| s.id),
            projectname_id_sender,
            _projectname_id_channel: projectname_id_channel,
            _groups_channel: groups_channel,
            groups_sender,
            groups_store: gtk::ListStore::new(&[String::static_type()]),
            _linked_groups_channel: linked_groups_channel,
            linked_groups_sender,
            linked_groups_store: gtk::ListStore::new(&[String::static_type()]),
            _server_link_updated_channel: server_link_updated_channel,
            server_link_updated_sender,
            description: sl.map(|d| d.desc.clone()).unwrap_or_else(|| "".to_string()),
            group_name: sl.and_then(|s| s.group_name.clone()),
            linked_server_id: sl.map(|s| s.linked_server_id),
            linked_group_name: sl.and_then(|s| s.linked_group_name.clone()),
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
            Msg::GotLinkedGroups(groups) => {
                dialog_helpers::fill_groups(
                    &self.model.linked_groups_store,
                    &self.widgets.linked_group,
                    &groups,
                    &self.model.linked_group_name,
                );
            }
            Msg::GotProjectNameAndId((name, id)) => {
                self.streams.pick_srv_button.emit(
                    pick_projectpad_item_button::Msg::SetProjectNameAndId(Some((name, id))),
                );
            }
            Msg::ServerSelected(s_id) => {
                self.model.linked_server_id = Some(s_id);
                self.reload_linked_groups();
            }
            Msg::ServerRemoved => {
                self.model.linked_server_id = None;
                self.reload_linked_groups();
            }
            Msg::OkPressed => {
                self.update_server_link();
            }
            // meant for my parent
            Msg::ServerLinkUpdated(_) => {}
        }
    }

    fn update_server_link(&self) {
        let project_id = self.model.project_id;
        let server_link_id = self.model.server_link_id;
        if self.model.linked_server_id.is_none() {
            self.widgets.must_pick_server_error.set_visible(true);
            return;
        }
        let new_linked_server_id = self.model.linked_server_id.unwrap();
        let new_desc = self.widgets.desc_entry.text();
        let new_group = self.widgets.group.active_text();
        let new_linked_group = self.widgets.linked_group.active_text();
        let new_env_type = self.model.environment_type.unwrap();
        let s = self.model.server_link_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_link::dsl as srv_link;
                let changeset = (
                    srv_link::desc.eq(new_desc.as_str()),
                    // never store Some("") for group, we want None then.
                    srv_link::group_name.eq(new_group
                        .as_ref()
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())),
                    // never store Some("") for group, we want None then.
                    srv_link::linked_group_name.eq(new_linked_group
                        .as_ref()
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())),
                    srv_link::linked_server_id.eq(new_linked_server_id),
                    srv_link::project_id.eq(project_id),
                    srv_link::environment.eq(new_env_type),
                );
                let server_link_after_result = perform_insert_or_update!(
                    sql_conn,
                    server_link_id,
                    srv_link::server_link,
                    srv_link::id,
                    changeset,
                    ServerLink,
                );
                s.send(server_link_after_result).unwrap();
            }))
            .unwrap();
    }

    view! {
        #[name="grid"]
        gtk::Grid {
            #[name="must_pick_server_error"]
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
                text: "Description",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
            },
            #[name="desc_entry"]
            gtk::Entry {
                hexpand: true,
                activates_default: true,
                text: &self.model.description,
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
                text: "Server",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 3,
                },
            },
            #[name="pick_srv_button"]
            PickProjectpadItemButton(PickProjectpadItemParams {
                db_sender: self.model.db_sender.clone(),
                item_type:pick_projectpad_item_button::ItemType::Server,
                item_id: self.model.linked_server_id,
                project_name_id: None, // we get the project name later through a message
            }) {
                cell: {
                    left_attach: 1,
                    top_attach: 3,
                },
                PickPpItemSelected(ref v) => Msg::ServerSelected(v.1),
                PickPpItemRemoved => Msg::ServerRemoved
            },
            gtk::Label {
                text: "Linked group",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 4,
                },
            },
            #[name="linked_group"]
            gtk::ComboBoxText({has_entry: true}) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 4,
                },
            },
        }
    }
}
