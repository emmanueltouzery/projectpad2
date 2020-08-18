use super::dialog_helpers;
use super::standard_dialogs::*;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{Server, ServerPointOfInterest};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Debug)]
pub enum Msg {
    GotGroups(Vec<String>),
    OkPressed,
    ServerPoiUpdated(ServerPointOfInterest),
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<ServerPointOfInterest, (String, Option<String>)>;

pub struct Model {
    relm: relm::Relm<ServerPoiAddEditDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    groups_store: gtk::ListStore,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,
    _server_poi_updated_channel: relm::Channel<SaveResult>,
    server_poi_updated_sender: relm::Sender<SaveResult>,
    server_id: i32,
    server_poi_id: Option<i32>,

    description: String,
    path: String,
    group_name: Option<String>,
}

#[widget]
impl Widget for ServerPoiAddEditDialog {
    fn init_view(&mut self) {
        self.init_group();
    }

    fn init_group(&self) {
        let s = self.model.groups_sender.clone();
        let sid = self.model.server_id;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                s.send(dialog_helpers::get_server_group_names(sql_conn, sid))
                    .unwrap();
            }))
            .unwrap();
        dialog_helpers::init_group_control(&self.model.groups_store, &self.group);
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, i32, Option<ServerPointOfInterest>),
    ) -> Model {
        let (db_sender, server_id, server_poi) = params;
        let stream = relm.stream().clone();
        let stream2 = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream2.emit(Msg::GotGroups(groups));
        });
        let (server_poi_updated_channel, server_poi_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(srv) => stream.emit(Msg::ServerPoiUpdated(srv)),
                Err((msg, e)) => display_error_str(&msg, e),
            });
        Model {
            relm: relm.clone(),
            db_sender,
            server_id,
            server_poi_id: server_poi.as_ref().map(|s| s.id),
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            _groups_channel: groups_channel,
            groups_sender,
            description: server_poi
                .as_ref()
                .map(|s| s.desc.clone())
                .unwrap_or_else(|| "".to_string()),
            path: server_poi
                .as_ref()
                .map(|s| s.path.clone())
                .unwrap_or_else(|| "".to_string()),
            group_name: server_poi.as_ref().and_then(|s| s.group_name.clone()),
            _server_poi_updated_channel: server_poi_updated_channel,
            server_poi_updated_sender,
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
                self.update_server_poi();
            }
            Msg::ServerPoiUpdated(_) => {} // meant for my parent, not me
        }
    }

    fn update_server_poi(&self) {
        let new_desc = self.desc_entry.get_text();
        let new_path = self.path_entry.get_text();
        let s = self.model.server_poi_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
                let changeset = (
                    srv_poi::desc.eq(new_desc.as_str()),
                    srv_poi::path.eq(new_path.as_str()),
                );
            }))
            .unwrap();
    }

    view! {
        #[name="grid"]
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
                text: "Path",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
            },
            #[name="path_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.path,
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
        }
    }
}
