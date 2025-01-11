use super::dialog_helpers;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{InterestType, RunOn, ServerPointOfInterest};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::str::FromStr;
use std::sync::mpsc;
use strum::IntoEnumIterator;

#[derive(Msg, Debug, Clone)]
pub enum Msg {
    GotGroups(Vec<String>),
    InterestTypeChanged,
    OkPressed,
    ServerPoiUpdated(ServerPointOfInterest),
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<ServerPointOfInterest, (String, Option<String>)>;

pub fn interest_type_desc(interest_type: InterestType) -> &'static str {
    match interest_type {
        InterestType::PoiLogFile => "Log file",
        InterestType::PoiConfigFile => "Config file",
        InterestType::PoiApplication => "Application",
        InterestType::PoiCommandToRun => "Command to run",
        InterestType::PoiBackupArchive => "Backup/archive",
        InterestType::PoiCommandTerminal => "Command to run (terminal)",
    }
}

pub fn init_interest_type_combo(combo: &gtk::ComboBoxText, interest_type: &str) {
    let mut entries: Vec<_> = InterestType::iter()
        .map(|st| (st, interest_type_desc(st)))
        .collect();
    entries.sort_by_key(|p| p.1);
    for (entry_type, entry_desc) in entries {
        combo.append(Some(&entry_type.to_string()), entry_desc);
    }
    combo.set_active_id(Some(interest_type));
}

pub struct Model {
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
    text: String,
    run_on: RunOn,
    is_run_on_visible: bool,
    group_name: Option<String>,
    interest_type: InterestType,
}

#[widget]
impl Widget for ServerPoiAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.widgets.grid);
        self.init_interest_type();
        self.init_group();
        self.init_run_on();
    }

    fn init_interest_type(&self) {
        init_interest_type_combo(
            &self.widgets.interest_type,
            self.model.interest_type.to_string().as_str(),
        );
    }

    fn run_on_desc(run_on: RunOn) -> &'static str {
        match run_on {
            RunOn::RunOnClient => "Client",
            RunOn::RunOnServer => "Server",
        }
    }

    fn init_run_on(&self) {
        let mut entries: Vec<_> = RunOn::iter()
            .map(|st| (st, Self::run_on_desc(st)))
            .collect();
        entries.sort_by_key(|p| p.1);
        for (entry_type, entry_desc) in entries {
            self.widgets
                .run_on
                .append(Some(&entry_type.to_string()), entry_desc);
        }
        self.widgets
            .run_on
            .set_active_id(Some(&self.model.run_on.to_string()));
    }

    fn init_group(&self) {
        dialog_helpers::init_group_control(&self.model.groups_store, &self.widgets.group);
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
            Option<ServerPointOfInterest>,
            gtk::AccelGroup,
        ),
    ) -> Model {
        let (db_sender, server_id, server_poi, _) = params;
        let stream = relm.stream().clone();
        let stream2 = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream2.emit(Msg::GotGroups(groups));
        });
        let (server_poi_updated_channel, server_poi_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(srv) => stream.emit(Msg::ServerPoiUpdated(srv)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        let interest_type = server_poi
            .as_ref()
            .map(|s| s.interest_type)
            .unwrap_or(InterestType::PoiApplication);
        let poi = server_poi.as_ref();
        Model {
            db_sender,
            server_id,
            server_poi_id: poi.map(|s| s.id),
            groups_store: gtk::ListStore::new(&[String::static_type()]),
            _groups_channel: groups_channel,
            groups_sender,
            description: poi
                .map(|s| s.desc.clone())
                .unwrap_or_else(|| "".to_string()),
            path: poi
                .map(|s| s.path.clone())
                .unwrap_or_else(|| "".to_string()),
            text: poi
                .map(|s| s.text.clone())
                .unwrap_or_else(|| "".to_string()),
            group_name: poi.and_then(|s| s.group_name.clone()),
            interest_type,
            is_run_on_visible: Self::is_run_on_visible(interest_type),
            run_on: poi.map(|s| s.run_on).unwrap_or(RunOn::RunOnServer),
            _server_poi_updated_channel: server_poi_updated_channel,
            server_poi_updated_sender,
        }
    }

    fn is_run_on_visible(interest_type: InterestType) -> bool {
        interest_type == InterestType::PoiCommandToRun
            || interest_type == InterestType::PoiCommandTerminal
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::GotGroups(groups) => {
                dialog_helpers::fill_groups(
                    &self.model.groups_store,
                    &self.widgets.group,
                    &groups,
                    &self.model.group_name,
                );
            }
            Msg::InterestTypeChanged => {
                self.model.is_run_on_visible =
                    Self::is_run_on_visible(self.combo_read_interest_type());
                // need to update so the 'text' label gets updated
                self.model.interest_type = self.combo_read_interest_type();
            }
            Msg::OkPressed => {
                self.update_server_poi();
            }
            Msg::ServerPoiUpdated(_) => {} // meant for my parent, not me
        }
    }

    fn combo_read_interest_type(&self) -> InterestType {
        self.widgets
            .interest_type
            .active_id()
            .map(|s| InterestType::from_str(s.as_str()).expect("Error parsing the interest type!?"))
            .expect("interest type not specified!?")
    }

    fn update_server_poi(&self) {
        let server_poi_id = self.model.server_poi_id;
        let server_id = self.model.server_id;
        let new_desc = self.widgets.desc_entry.text();
        let new_path = self.widgets.path_entry.text();
        let new_text = self.widgets.text_entry.text();
        let new_group = self.widgets.group.active_text();
        let new_interest_type = self.combo_read_interest_type();
        let new_run_on = self
            .widgets
            .run_on
            .active_id()
            .map(|s| RunOn::from_str(s.as_str()).expect("Error parsing the run_on!?"))
            .expect("run_on not specified!?");
        let s = self.model.server_poi_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
                let changeset = (
                    srv_poi::desc.eq(new_desc.as_str()),
                    srv_poi::path.eq(new_path.as_str()),
                    srv_poi::text.eq(new_text.as_str()),
                    // never store Some("") for group, we want None then.
                    srv_poi::group_name.eq(new_group
                        .as_ref()
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())),
                    srv_poi::interest_type.eq(new_interest_type),
                    srv_poi::run_on.eq(new_run_on),
                    srv_poi::server_id.eq(server_id),
                );
                let server_poi_after_result = perform_insert_or_update!(
                    sql_conn,
                    server_poi_id,
                    srv_poi::server_point_of_interest,
                    srv_poi::id,
                    changeset,
                    ServerPointOfInterest,
                );
                s.send(server_poi_after_result).unwrap();
            }))
            .unwrap();
    }

    view! {
        #[name="grid"]
        gtk::Grid {
            visible: false,
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
                activates_default: true,
                text: &self.model.path,
                cell: {
                    left_attach: 1,
                    top_attach: 1,
                },
            },
            #[name="text"]
            gtk::Label {
                text: poi_get_text_label(self.model.interest_type),
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                },
            },
            #[name="text_entry"]
            gtk::Entry {
                hexpand: true,
                activates_default: true,
                text: &self.model.text,
                cell: {
                    left_attach: 1,
                    top_attach: 2,
                },
            },
            gtk::Label {
                text: "Run on",
                halign: gtk::Align::End,
                visible: self.model.is_run_on_visible,
                cell: {
                    left_attach: 0,
                    top_attach: 3,
                },
            },
            #[name="run_on"]
            gtk::ComboBoxText {
                hexpand: true,
                visible: self.model.is_run_on_visible,
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
                text: "Interest type",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 5,
                },
            },
            #[name="interest_type"]
            gtk::ComboBoxText {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 5,
                },
                changed(_) => Msg::InterestTypeChanged
            },
        }
    }
}
