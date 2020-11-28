use super::dialog_helpers;
use super::server_poi_add_edit_dlg::{init_interest_type_combo, poi_get_text_label};
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{InterestType, ProjectPointOfInterest};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::str::FromStr;
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    GotGroups(Vec<String>),
    OkPressed,
    InterestTypeChanged,
    PoiUpdated(ProjectPointOfInterest),
}

// String for details, because I can't pass Error across threads
type SaveResult = Result<ProjectPointOfInterest, (String, Option<String>)>;

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    _project_poi_updated_channel: relm::Channel<SaveResult>,
    project_poi_updated_sender: relm::Sender<SaveResult>,
    _groups_channel: relm::Channel<Vec<String>>,
    groups_sender: relm::Sender<Vec<String>>,
    groups_store: gtk::ListStore,
    project_id: i32,
    project_poi_id: Option<i32>,

    description: String,
    path: String,
    text: String,
    group_name: Option<String>,
    interest_type: InterestType,
}

#[widget]
impl Widget for ProjectPoiAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
        self.init_interest_type();
        self.init_group();
    }

    fn init_interest_type(&self) {
        init_interest_type_combo(
            &self.interest_type,
            self.model.interest_type.to_string().as_str(),
        );
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
        dialog_helpers::init_group_control(&self.model.groups_store, &self.group);
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (
            mpsc::Sender<SqlFunc>,
            i32,
            Option<ProjectPointOfInterest>,
            gtk::AccelGroup,
        ),
    ) -> Model {
        let (db_sender, project_id, project_poi, _) = params;
        let stream = relm.stream().clone();
        let (groups_channel, groups_sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        let stream2 = relm.stream().clone();
        let (project_poi_updated_channel, project_poi_updated_sender) =
            relm::Channel::new(move |r: SaveResult| match r {
                Ok(srv) => stream2.emit(Msg::PoiUpdated(srv)),
                Err((msg, e)) => standard_dialogs::display_error_str(&msg, e),
            });
        let interest_type = project_poi
            .as_ref()
            .map(|s| s.interest_type)
            .unwrap_or(InterestType::PoiApplication);
        let poi = project_poi.as_ref();
        Model {
            db_sender,
            project_id,
            _groups_channel: groups_channel,
            groups_sender,
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            _project_poi_updated_channel: project_poi_updated_channel,
            project_poi_updated_sender,
            project_poi_id: poi.map(|s| s.id),
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
            Msg::InterestTypeChanged => {
                // need to update so the 'text' label gets updated
                self.model.interest_type = self.combo_read_interest_type();
            }
            Msg::OkPressed => {
                self.update_project_poi();
            }
            Msg::PoiUpdated(_) => {} // meant for my parent, not me
        }
    }

    fn combo_read_interest_type(&self) -> InterestType {
        self.interest_type
            .get_active_id()
            .map(|s| InterestType::from_str(s.as_str()).expect("Error parsing the interest type!?"))
            .expect("interest type not specified!?")
    }

    fn update_project_poi(&self) {
        let project_poi_id = self.model.project_poi_id;
        let project_id = self.model.project_id;
        let new_desc = self.desc_entry.get_text();
        let new_path = self.path_entry.get_text();
        let new_text = self.text_entry.get_text();
        let new_group = self.group.get_active_text();
        let new_interest_type = self.combo_read_interest_type();
        let s = self.model.project_poi_updated_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
                let changeset = (
                    prj_poi::desc.eq(new_desc.as_str()),
                    prj_poi::path.eq(new_path.as_str()),
                    prj_poi::text.eq(new_text.as_str()),
                    // never store Some("") for group, we want None then.
                    prj_poi::group_name.eq(new_group
                        .as_ref()
                        .map(|s| s.as_str())
                        .filter(|s| !s.is_empty())),
                    prj_poi::interest_type.eq(new_interest_type),
                    prj_poi::project_id.eq(project_id),
                );
                let project_poi_after_result = perform_insert_or_update!(
                    sql_conn,
                    project_poi_id,
                    prj_poi::project_point_of_interest,
                    prj_poi::id,
                    changeset,
                    ProjectPointOfInterest,
                );
                s.send(project_poi_after_result).unwrap();
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
