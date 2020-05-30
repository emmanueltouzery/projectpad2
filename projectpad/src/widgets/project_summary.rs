use super::win::Project;
use gtk::prelude::*;
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
                markup: "<b>Hubli</b>"
            },
            gtk::Grid {
                child: {
                    padding: 20,
                },
                column_homogeneous: true,
                row_spacing: 3,
                gtk::Label {
                    markup: "<b>Dev</b>",
                    cell: {
                        left_attach: 0,
                        top_attach: 0,
                    }
                },
                gtk::Label {
                    text: "Stg",
                    cell: {
                        left_attach: 1,
                        top_attach: 0,
                    }
                },
                gtk::Label {
                    text: "Uat",
                    cell: {
                        left_attach: 0,
                        top_attach: 1,
                    }
                },
                gtk::Label {
                    text: "Prd",
                    cell: {
                        left_attach: 1,
                        top_attach: 1,
                    }
                },
            }
        }
    }
}
