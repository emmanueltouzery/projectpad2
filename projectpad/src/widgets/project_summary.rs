use gtk::prelude::*;
use projectpadsql::models::Project;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    ProjectActivated(Project),
}

pub struct Model {
    project: Option<Project>,
}

#[widget]
impl Widget for ProjectSummary {
    fn model(_: ()) -> Model {
        Model { project: None }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectActivated(prj) => self.model.project = Some(prj),
        }
    }

    view! {
        gtk::Box {
            orientation: gtk::Orientation::Vertical,
            gtk::Label {
                margin_top: 8,
                margin_bottom: 8,
                markup: &self.model.project.as_ref().map(|p| format!("<b>{}</b>", &p.name)).unwrap_or("".to_string())
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
