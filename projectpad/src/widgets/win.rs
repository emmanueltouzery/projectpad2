use super::project_items_list::Msg as ProjectItemsListMsg;
use super::project_items_list::{ProjectItem, ProjectItemsList};
use super::project_list::{Msg::ProjectActivated, ProjectList};
use super::project_poi_contents::Msg as ProjectPoiContentsMsg;
use super::project_poi_contents::ProjectPoiContents;
use super::project_poi_header::Msg as ProjectPoiHeaderMsg;
use super::project_poi_header::ProjectPoiHeader;
use super::project_summary::Msg as ProjectSummaryMsg;
use super::project_summary::ProjectSummary;
use super::search_view::Msg as SearchViewMsg;
use super::search_view::SearchView;
use super::wintitlebar::Msg as WinTitleBarMsg;
use super::wintitlebar::WinTitleBar;
use crate::sql_thread::SqlFunc;
use crate::widgets::project_items_list::Msg::ProjectItemSelected;
use crate::widgets::project_summary::Msg::EnvironmentChanged;
use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, Project};
use relm::{Component, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

const CSS_DATA: &[u8] = include_bytes!("../../resources/style.css");

#[derive(Msg)]
pub enum Msg {
    Quit,
    ProjectActivated(Project),
    EnvironmentChanged(EnvironmentType),
    ProjectItemSelected(Option<ProjectItem>),
    SearchActiveChanged(bool),
    SearchTextChanged(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPoiItem {
    pub name: String,
    // TODO groups
}
pub struct Model {
    relm: relm::Relm<Win>,
    db_sender: mpsc::Sender<SqlFunc>,
    titlebar: Component<WinTitleBar>,
}

const CHILD_NAME_NORMAL: &str = "normal";
const CHILD_NAME_SEARCH: &str = "search";

#[widget]
impl Widget for Win {
    fn init_view(&mut self) {
        if let Err(err) = self.load_style() {
            println!("Error loading the CSS: {}", err);
        }
        let titlebar = &self.model.titlebar;
        relm::connect!(titlebar@WinTitleBarMsg::SearchActiveChanged(is_active),
                               self.model.relm, Msg::SearchActiveChanged(is_active));
        relm::connect!(titlebar@WinTitleBarMsg::SearchTextChanged(ref search_text),
                               self.model.relm, Msg::SearchTextChanged(search_text.clone()));
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        gtk::IconTheme::get_default()
            .unwrap()
            .add_resource_path("/icons");
        let titlebar = relm::init::<WinTitleBar>(()).expect("win title bar init");
        Model {
            relm: relm.clone(),
            db_sender,
            titlebar,
        }
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
                    .emit(ProjectPoiContentsMsg::ProjectItemSelected(pi));
            }
            Msg::SearchActiveChanged(is_active) => {
                self.normal_or_search_stack
                    .set_visible_child_name(if is_active {
                        CHILD_NAME_SEARCH
                    } else {
                        CHILD_NAME_NORMAL
                    });
            }
            Msg::SearchTextChanged(search_text) => {
                self.search_view
                    .emit(SearchViewMsg::FilterChanged(Some(search_text)));
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
            titlebar: Some(self.model.titlebar.widget()),
            property_default_width: 1000,
            property_default_height: 650,
            #[name="normal_or_search_stack"]
            gtk::Stack {
                gtk::Box {
                    child: {
                        name: Some(CHILD_NAME_NORMAL)
                    },
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
                        spacing: 10,
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
                #[name="search_view"]
                SearchView() {
                    child: {
                        name: Some(CHILD_NAME_SEARCH)
                    },
                }
            },
            delete_event(_, _) => (Msg::Quit, Inhibit(false)),
        }
    }
}
