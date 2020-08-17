use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::ServerPointOfInterest;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Debug)]
pub enum Msg {}

pub struct Model {
    relm: relm::Relm<ServerPoiAddEditDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    project_id: i32,
    server_poi_id: Option<i32>,
    description: String,
}

#[widget]
impl Widget for ServerPoiAddEditDialog {
    fn init_view(&mut self) {}

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, i32, Option<ServerPointOfInterest>),
    ) -> Model {
        let (db_sender, project_id, server_poi) = params;
        Model {
            relm: relm.clone(),
            db_sender,
            project_id,
            server_poi_id: server_poi.as_ref().map(|s| s.id),
            description: server_poi
                .as_ref()
                .map(|s| s.desc.clone())
                .unwrap_or_else(|| "".to_string()),
        }
    }

    fn update(&mut self, event: Msg) {}

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
        }
    }
}
