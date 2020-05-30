use gtk::prelude::*;
use projectpadsql::models::Project;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    project: Option<Project>,
}

#[widget]
impl Widget for ProjectSummary {
    fn model(project: Option<Project>) -> Model {
        Model { project }
    }

    fn update(&mut self, event: Msg) {}

    view! {
        gtk::Box {
            orientation: gtk::Orientation::Vertical,
            gtk::Label {
                margin_top: 8,
                margin_bottom: 8,
                markup: "<b>Hubli</b>"
            },
            gtk::Box {
                homogeneous: true,
                margin_start: 35,
                margin_end: 35,
                child: {
                    padding: 5,
                },
                spacing: 3,
                gtk::Label {
                    markup: "<b>Dev</b>",
                },
                gtk::Label {
                    text: "Stg",
                },
                gtk::Label {
                    text: "Uat",
                },
                gtk::Label {
                    text: "Prd",
                },
            }
        }
    }
}
