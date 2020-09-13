use super::dialog_helpers;
use super::pick_projectpad_item_button;
use super::pick_projectpad_item_button::PickProjectpadItemButton;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::ServerLink;
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
    project_id: i32,

    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,

    description: String,
    group_name: Option<String>,
    linked_server_id: Option<i32>,
}

#[widget]
impl Widget for ServerLinkAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
        self.init_group();
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
            Option<ServerLink>,
            gtk::AccelGroup,
        ),
    ) -> Model {
        let (db_sender, project_id, server_link, accel_group) = params;
        let sl = server_link.as_ref();
        let stream = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        Model {
            db_sender,
            project_id,
            _groups_channel: groups_channel,
            groups_sender,
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            description: sl.map(|d| d.desc.clone()).unwrap_or_else(|| "".to_string()),
            group_name: sl.and_then(|s| s.group_name.clone()),
            linked_server_id: sl.map(|s| s.linked_server_id),
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
            PickProjectpadItemButton((self.model.db_sender.clone(),
                                      pick_projectpad_item_button::ItemType::Server,
                                      self.model.linked_server_id)) {
                cell: {
                    left_attach: 1,
                    top_attach: 3,
                },
                // PickPpItemSelected(ref v) => Msg::ServerDbSelected(v.1),
                // PickPpItemRemoved => Msg::ServerDbRemoved
            }
        }
    }
}
