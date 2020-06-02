use super::project_badge::Msg as ProjectBadgeMsg;
use super::project_badge::ProjectBadge;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::Project;
use relm::ContainerWidget;
use relm::{Component, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Clone)]
pub enum Msg {
    ProjectActivated(Project),
    GotProjects(Vec<Project>),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    relm: relm::Relm<ProjectList>,
    projects: Vec<Project>,
    // need to keep hold of the children widgets
    children_widgets: Vec<Component<ProjectBadge>>,
    _channel: relm::Channel<Vec<Project>>,
    sender: relm::Sender<Vec<Project>>,
}

#[widget]
impl Widget for ProjectList {
    fn init_view(&mut self) {
        self.fetch_projects();
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |prjs: Vec<Project>| {
            println!("emitting {}", prjs.len());
            stream.emit(Msg::GotProjects(prjs));
        });
        Model {
            relm: relm.clone(),
            db_sender,
            _channel: channel,
            sender,
            projects: vec![],
            children_widgets: vec![],
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectActivated(ref _project) => {}
            Msg::GotProjects(prjs) => {
                println!("got projects: {}", prjs.len());
                self.model.projects = prjs;
                self.update_projects_list();
            }
        }
    }

    fn load_projects(db_conn: &SqliteConnection) -> Vec<Project> {
        use projectpadsql::schema::project::dsl::*;
        project.order(name.asc()).load::<Project>(db_conn).unwrap()
    }

    fn fetch_projects(&mut self) {
        let s = self.model.sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                let prjs = Self::load_projects(sql_conn);
                println!("loaded prjs: {}", prjs.len());
                s.send(prjs).unwrap();
            }))
            .unwrap();
    }

    fn update_projects_list(&mut self) {
        for child in self.project_list.get_children() {
            self.project_list.remove(&child);
        }
        self.model.children_widgets.clear();
        for project in &self.model.projects {
            let child = self
                .project_list
                .add_widget::<ProjectBadge>(project.clone());
            relm::connect!(
                child@ProjectBadgeMsg::Activate(ref project),
                self.model.relm,
                Msg::ProjectActivated(project.clone())
            );
            let relm = &self.model.relm;
            relm::connect!(
                relm@Msg::ProjectActivated(ref project),
                child,
                ProjectBadgeMsg::ActiveProjectChanged(project.id));
            self.model.children_widgets.push(child);
        }
    }

    view! {
        gtk::ScrolledWindow {
            #[name="project_list"]
            gtk::Box {
                orientation: gtk::Orientation::Vertical
            }
        }
    }
}
