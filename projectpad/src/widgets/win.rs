use super::project_list::ProjectList;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    Quit,
}

#[derive(Clone, Debug)]
pub struct Project {
    pub name: String,
}

impl Project {
    fn new<S: Into<String>>(name: S) -> Project {
        Project { name: name.into() }
    }
}

pub struct Model {
    projects: Vec<Project>,
}

#[widget]
impl Widget for Win {
    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {
            projects: vec![Project::new("Hubli"), Project::new("Dan")],
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Quit => gtk::main_quit(),
        }
    }

    view! {
        gtk::Window {
            gtk::Box {
                // ProjectBadge(self.model.projects.first().unwrap().clone()) {
                ProjectList(self.model.projects.clone()) {
                    child: {
                        fill: true,
                        expand: true,
                    },
                }
            },
            delete_event(_, _) => (Msg::Quit, Inhibit(false)),
        }
    }
}
