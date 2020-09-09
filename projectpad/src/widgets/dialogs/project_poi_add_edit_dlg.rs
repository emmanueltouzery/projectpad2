use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use projectpadsql::models::{InterestType, ProjectPointOfInterest, RunOn};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    OkPressed,
    PoiUpdated(ProjectPointOfInterest),
}

pub struct Model {
    relm: relm::Relm<ProjectPoiAddEditDialog>,
}

#[widget]
impl Widget for ProjectPoiAddEditDialog {
    fn init_view(&mut self) {}

    fn model(
        relm: &relm::Relm<Self>,
        params: (
            mpsc::Sender<SqlFunc>,
            i32,
            Option<ProjectPointOfInterest>,
            gtk::AccelGroup,
        ),
    ) -> Model {
        Model { relm: relm.clone() }
    }

    fn update(&mut self, event: Msg) {}

    view! {
        gtk::Grid {}
    }
}
