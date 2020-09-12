use super::dialog_helpers;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::ProjectNote;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    OkPressed,
}

pub struct Model {
    title: String,
    group_name: Option<String>,
}

#[widget]
impl Widget for ProjectNoteAddEditDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (
            mpsc::Sender<SqlFunc>,
            i32,
            Option<ProjectNote>,
            gtk::AccelGroup,
        ),
    ) -> Model {
        let (db_sender, project_id, project_note, accel_group) = params;
        let pn = project_note.as_ref();
        Model {
            title: pn
                .map(|d| d.title.clone())
                .unwrap_or_else(|| "".to_string()),
            group_name: pn.and_then(|s| s.group_name.clone()),
        }
    }

    fn update(&mut self, event: Msg) {}

    view! {
        #[name="grid"]
        gtk::Grid {
            gtk::Label {
                text: "Title",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 0,
                },
            },
            #[name="title_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.title,
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
                    top_attach: 1,
                },
            },
            #[name="group"]
            gtk::ComboBoxText({has_entry: true}) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 1,
                },
            },
        }
    }
}
