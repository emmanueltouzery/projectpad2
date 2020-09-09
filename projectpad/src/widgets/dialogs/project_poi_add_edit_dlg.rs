use super::dialog_helpers;
use super::server_poi_add_edit_dlg::{init_interest_type_combo, poi_get_text_label};
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::{InterestType, ProjectPointOfInterest};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    OkPressed,
    InterestTypeChanged,
    PoiUpdated(ProjectPointOfInterest),
}

pub struct Model {
    relm: relm::Relm<ProjectPoiAddEditDialog>,

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
        // self.init_group();
    }

    fn init_interest_type(&self) {
        init_interest_type_combo(
            &self.interest_type,
            self.model.interest_type.to_string().as_str(),
        );
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
        let interest_type = project_poi
            .as_ref()
            .map(|s| s.interest_type)
            .unwrap_or(InterestType::PoiApplication);
        let poi = project_poi.as_ref();
        Model {
            relm: relm.clone(),
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

    fn update(&mut self, event: Msg) {}

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
