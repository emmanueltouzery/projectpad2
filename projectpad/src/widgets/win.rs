use super::project_items_list::Msg as ProjectItemsListMsg;
use super::project_items_list::{ProjectItem, ProjectItemsList};
use super::project_list::{Msg::ProjectActivated, ProjectList};
use super::project_poi_contents::Msg as ProjectPoiContentsMsg;
use super::project_poi_contents::ProjectPoiContents;
use super::project_poi_header::Msg as ProjectPoiHeaderMsg;
use super::project_poi_header::ProjectPoiHeader;
use super::project_summary::Msg as ProjectSummaryMsg;
use super::project_summary::ProjectSummary;
use crate::sql_thread::SqlFunc;
use crate::widgets::project_items_list::Msg::ProjectItemSelected;
use crate::widgets::project_summary::Msg::EnvironmentChanged;
use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, Project};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

const CSS_DATA: &[u8] = include_bytes!("../../resources/style.css");

#[derive(Msg)]
pub enum Msg {
    Quit,
    ProjectActivated(Project),
    EnvironmentChanged(EnvironmentType),
    ProjectItemSelected(Option<ProjectItem>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPoiItem {
    pub name: String,
    // TODO groups
}
pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
}

#[widget]
impl Widget for Win {
    fn init_view(&mut self) {
        if let Err(err) = self.load_style() {
            println!("Error loading the CSS: {}", err);
        }
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        Model { db_sender }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Quit => gtk::main_quit(),
            Msg::ProjectActivated(project) => {
                self.project_items_list
                    .emit(ProjectItemsListMsg::ActiveProjectChanged(project.clone()));
                self.project_summary
                    .emit(ProjectSummaryMsg::ProjectActivated(project));
            }
            Msg::EnvironmentChanged(env) => {
                self.project_items_list
                    .emit(ProjectItemsListMsg::ActiveEnvironmentChanged(env));
            }
            Msg::ProjectItemSelected(pi) => {
                self.project_poi_header
                    .emit(ProjectPoiHeaderMsg::ProjectItemSelected(pi.clone()));
                self.project_poi_contents
                    .emit(ProjectPoiContentsMsg::ProjectItemSelected(pi.clone()));
            }
        }
    }

    fn load_style(&self) -> Result<(), Box<dyn std::error::Error>> {
        let screen = self.window.get_screen().unwrap();
        let css = gtk::CssProvider::new();
        css.load_from_data(CSS_DATA)?;
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        Ok(())
    }

    view! {
        #[name="window"]
        gtk::Window {
            property_default_width: 1000,
            property_default_height: 650,
            gtk::Box {
                ProjectList(self.model.db_sender.clone()) {
                    property_width_request: 60,
                    ProjectActivated(ref prj) => Msg::ProjectActivated(prj.clone())
                },
                gtk::Box {
                    orientation: gtk::Orientation::Vertical,
                    #[name="project_summary"]
                    ProjectSummary() {
                        EnvironmentChanged(env) => Msg::EnvironmentChanged(env)
                    },
                    #[name="project_items_list"]
                    ProjectItemsList(self.model.db_sender.clone()) {
                        property_width_request: 260,
                        child: {
                            fill: true,
                            expand: true,
                        },
                        ProjectItemSelected(ref pi) => Msg::ProjectItemSelected(pi.clone())
                    },
                },
                gtk::Box {
                    orientation: gtk::Orientation::Vertical,
                    child: {
                        fill: true,
                        expand: true,
                    },
                    #[name="project_poi_header"]
                    ProjectPoiHeader(),
                    #[name="project_poi_contents"]
                    ProjectPoiContents(self.model.db_sender.clone()) {
                        child: {
                            fill: true,
                            expand: true,
                        },
                    }
                }
            },
            delete_event(_, _) => (Msg::Quit, Inhibit(false)),
        }
    }
}
