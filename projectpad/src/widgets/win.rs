use super::project_items_list::ProjectItemsList;
use super::project_list::ProjectList;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    Quit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub name: String,
}

impl Project {
    fn new<S: Into<String>>(name: S) -> Project {
        Project { name: name.into() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerType {
    Application,
    Database,
    HttpServerOrProxy,
    Monitoring,
    Reporting,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPoi {
    pub name: String,
    pub address: String,
    pub username: String,
    pub server_type: ServerType,
}

impl ProjectPoi {
    fn new<S: Into<String>>(
        name: S,
        address: S,
        username: S,
        server_type: ServerType,
    ) -> ProjectPoi {
        ProjectPoi {
            name: name.into(),
            address: address.into(),
            username: username.into(),
            server_type,
        }
    }
}

pub struct Model {
    projects: Vec<Project>,
    cur_project: Option<Project>,
    project_items: Vec<ProjectPoi>,
}

#[widget]
impl Widget for Win {
    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {
            projects: vec![Project::new("Hubli"), Project::new("Dan")],
            cur_project: Some(Project::new("Hubli")),
            project_items: vec![
                ProjectPoi::new("AFCp", "117.23.13.13", "razvoj", ServerType::Application),
                ProjectPoi::new("AFC SQL", "34.23.43.53", "razvoj", ServerType::Database),
            ],
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
                    property_width_request: 60,
                },
                ProjectItemsList((self.model.cur_project.clone(), self.model.project_items.clone())) {
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
