use super::project_items_list::Msg as ProjectItemsListMsg;
use super::project_items_list::{ProjectItem, ProjectItemsList};
use super::project_list::Msg as ProjectListMsg;
use super::project_list::{Msg::ProjectActivated, ProjectList, UpdateParents};
use super::project_poi_contents::Msg as ProjectPoiContentsMsg;
use super::project_poi_contents::ProjectPoiContents;
use super::project_poi_header::Msg as ProjectPoiHeaderMsg;
use super::project_poi_header::Msg::ServerUpdated as ProjectPoiHeaderServerUpdatedMsg;
use super::project_poi_header::ProjectPoiHeader;
use super::project_summary::Msg as ProjectSummaryMsg;
use super::project_summary::ProjectSummary;
use super::search_view::Msg as SearchViewMsg;
use super::search_view::Msg::OpenItemFull as SearchViewOpenItemFull;
use super::search_view::SearchView;
use super::server_poi_contents::ServerItem;
use super::wintitlebar::Msg as WinTitleBarMsg;
use super::wintitlebar::WinTitleBar;
use crate::sql_thread::SqlFunc;
use crate::widgets::project_items_list::Msg::ProjectItemSelected;
use crate::widgets::project_summary::Msg::EnvironmentChanged;
use gtk::prelude::*;
use projectpadsql::models::{EnvironmentType, Project, Server};
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
    DisplayItem((Project, Option<ProjectItem>, Option<ServerItem>)),
    KeyPress(gdk::EventKey),
    ServerUpdated(Server),
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
            Msg::DisplayItem((project, project_item, _server_item)) => {
                self.project_list
                    .emit(ProjectListMsg::ProjectSelectedFromElsewhere(project.id));
                let env = match &project_item {
                    Some(ProjectItem::Server(s)) => Some(s.environment),
                    Some(ProjectItem::ServerLink(s)) => Some(s.environment),
                    Some(ProjectItem::ProjectNote(n)) if n.has_prod => {
                        Some(EnvironmentType::EnvProd)
                    }
                    Some(ProjectItem::ProjectNote(n)) if n.has_uat => Some(EnvironmentType::EnvUat),
                    Some(ProjectItem::ProjectNote(n)) if n.has_stage => {
                        Some(EnvironmentType::EnvStage)
                    }
                    Some(ProjectItem::ProjectNote(n)) if n.has_dev => {
                        Some(EnvironmentType::EnvDevelopment)
                    }
                    _ => None,
                };
                if let Some(e) = env {
                    self.project_summary.emit(
                        ProjectSummaryMsg::ProjectEnvironmentSelectedFromElsewhere((
                            project.clone(),
                            e,
                        )),
                    );
                }
                self.project_items_list.emit(
                    ProjectItemsListMsg::ProjectItemSelectedFromElsewhere((
                        project,
                        env,
                        project_item,
                    )),
                );
                self.model
                    .relm
                    .stream()
                    .emit(Msg::SearchActiveChanged(false));
                self.model
                    .titlebar
                    .stream()
                    .emit(WinTitleBarMsg::SearchActiveChanged(false));
            }
            Msg::KeyPress(e) => {
                if e.get_keyval() == gdk::keys::constants::Escape {
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::SearchActiveChanged(false));
                    self.model
                        .titlebar
                        .stream()
                        .emit(WinTitleBarMsg::SearchActiveChanged(false));
                } else if let Some(k) = e.get_keyval().to_unicode() {
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::SearchActiveChanged(true));
                    self.search_view
                        .emit(SearchViewMsg::FilterChanged(Some(k.to_string())));
                    self.model
                        .titlebar
                        .emit(WinTitleBarMsg::SearchTextChangedFromElsewhere((
                            k.to_string(),
                            e,
                        )));
                }
            }
            Msg::ServerUpdated(ref srv) => {
                self.project_items_list
                    .stream()
                    .emit(ProjectItemsListMsg::RefreshItemList(ProjectItem::Server(
                        srv.clone(),
                    )));
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
                    #[name="project_list"]
                    ProjectList(self.model.db_sender.clone()) {
                        property_width_request: 60,
                        ProjectActivated((ref prj, UpdateParents::Yes)) => Msg::ProjectActivated(prj.clone())
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
                        ProjectPoiHeader((self.model.db_sender.clone(), None)) {
                            ProjectPoiHeaderServerUpdatedMsg(ref srv) => Msg::ServerUpdated(srv.clone()),
                        },
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
                SearchView(self.model.db_sender.clone()) {
                    child: {
                        name: Some(CHILD_NAME_SEARCH)
                    },
                    SearchViewOpenItemFull(ref item) => Msg::DisplayItem(item.clone())
                }
            },
            delete_event(_, _) => (Msg::Quit, Inhibit(false)),
            key_press_event(_, event) => (Msg::KeyPress(event.clone()), Inhibit(false)),
        }
    }
}
