use super::project_items_list::ProjectItemsList;
use super::project_list::ProjectList;
use super::project_poi_contents::ProjectPoiContents;
use super::project_summary::ProjectSummary;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use gtk::prelude::*;
use projectpadsql::models::Project;
use relm::Widget;
use relm_derive::{widget, Msg};

const CSS_DATA: &[u8] = include_bytes!("../../resources/style.css");

#[derive(Msg)]
pub enum Msg {
    Quit,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPoiItem {
    pub name: String,
    // TODO groups
}

impl ProjectPoiItem {
    fn new<S: Into<String>>(name: S) -> ProjectPoiItem {
        ProjectPoiItem { name: name.into() }
    }
}

pub struct Model {
    db_conn: SqliteConnection,
    projects: Vec<Project>,
    cur_project: Option<Project>,
    project_items: Vec<ProjectPoi>,
    cur_project_item: Option<ProjectPoi>,
    project_poi_items: Vec<ProjectPoiItem>,
}

#[widget]
impl Widget for Win {
    fn init_view(&mut self) {
        if let Err(err) = self.load_style() {
            println!("Error loading the CSS: {}", err);
        }
    }

    fn model(relm: &relm::Relm<Self>, db_conn: SqliteConnection) -> Model {
        use projectpadsql::schema::project::dsl::*;
        let prjs = project.load::<Project>(&db_conn).unwrap();
        println!("{:?}", prjs);
        let project_items = vec![
            ProjectPoi::new("AFCp", "117.23.13.13", "razvoj", ServerType::Application),
            ProjectPoi::new("AFC SQL", "34.23.43.53", "razvoj", ServerType::Database),
        ];
        Model {
            db_conn,
            cur_project: prjs.first().cloned(),
            projects: prjs,
            cur_project_item: project_items.first().cloned(),
            project_items,
            project_poi_items: vec![
                ProjectPoiItem::new("metrics user"),
                ProjectPoiItem::new("Prometheus"),
                ProjectPoiItem::new("afcp"),
            ],
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Quit => gtk::main_quit(),
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
                // ProjectBadge(self.model.projects.first().unwrap().clone()) {
                ProjectList(self.model.projects.clone()) {
                    property_width_request: 60,
                },
                gtk::Box {
                    orientation: gtk::Orientation::Vertical,
                    ProjectSummary(self.model.cur_project.clone()),
                    ProjectItemsList((self.model.cur_project.clone(), self.model.project_items.clone())) {
                        property_width_request: 260,
                        child: {
                            fill: true,
                            expand: true,
                        }
                    },
                },
                ProjectPoiContents(self.model.cur_project_item.clone(), self.model.project_poi_items.clone()) {
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
