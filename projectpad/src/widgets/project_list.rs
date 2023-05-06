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

#[derive(Clone)]
pub enum UpdateParents {
    Yes,
    No,
}

#[derive(Msg, Clone)]
pub enum Msg {
    DbPrepared,
    ProjectActivated((Project, UpdateParents)),
    GotProjects(Vec<Project>),
    ProjectSelectedFromElsewhere(i32),
    ForceReload,
    ProjectListChanged,
    AddProject,
    MouseEnterProject(i32),
    MouseLeaveProject(i32),
    UpdateProjectTooltip(Option<(String, i32)>),
    DarkThemeToggled,
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    relm: relm::Relm<ProjectList>,
    projects: Vec<Project>,
    active_project_id: Option<i32>,
    // need to keep hold of the children widgets
    children_widgets: Vec<Component<ProjectBadge>>,
    _channel: relm::Channel<Vec<Project>>,
    sender: relm::Sender<Vec<Project>>,
}

#[widget]
impl Widget for ProjectList {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |prjs: Vec<Project>| {
            stream.emit(Msg::GotProjects(prjs));
        });
        Model {
            relm: relm.clone(),
            db_sender,
            _channel: channel,
            sender,
            projects: vec![],
            active_project_id: None,
            children_widgets: vec![],
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::DbPrepared => {
                self.fetch_projects();
            }
            Msg::ProjectActivated((ref project, _)) => {
                self.model.active_project_id = Some(project.id);
                for badge in &self.model.children_widgets {
                    badge
                        .stream()
                        .emit(ProjectBadgeMsg::ActiveProjectChanged(project.id));
                }
            }
            Msg::GotProjects(prjs) => {
                self.model.projects = prjs;
                self.update_projects_list();
            }
            Msg::ProjectSelectedFromElsewhere(pid) => {
                if let Some(prj) = self.model.projects.iter().find(|p| p.id == pid) {
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::ProjectActivated((prj.clone(), UpdateParents::No)));
                    self.model.active_project_id = Some(pid);
                }
            }
            Msg::ProjectListChanged => {
                self.fetch_projects();
            }
            Msg::ForceReload => {
                // by resetting the active project id, we'll
                // notify our parent that the project changed,
                // forcing a complete reload, even if the project
                // is the same as before -- useful if we think
                // a lower level (project item or server item)
                // changed
                self.model.active_project_id = None;
                self.fetch_projects();
            }
            // for my parent
            Msg::AddProject => {}
            Msg::MouseEnterProject(id) => {
                if let Some(idx) = self.model.projects.iter().position(|p| p.id == id) {
                    let child_widget = self.model.children_widgets[idx as usize].widget();
                    let y = -self
                        .widgets
                        .scroll
                        .translate_coordinates(child_widget, 0, 0)
                        .unwrap()
                        .1;
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::UpdateProjectTooltip(Some((
                            self.model.projects[idx].name.clone(),
                            y,
                        ))));
                }
            }
            Msg::MouseLeaveProject(_id) => {
                self.model
                    .relm
                    .stream()
                    .emit(Msg::UpdateProjectTooltip(None));
            }
            Msg::UpdateProjectTooltip(_) => {}
            Msg::DarkThemeToggled => {
                for child in &self.model.children_widgets {
                    child.stream().emit(ProjectBadgeMsg::DarkThemeToggled);
                }
            }
        }
    }

    fn load_projects(db_conn: &mut SqliteConnection) -> Vec<Project> {
        use projectpadsql::schema::project::dsl::*;
        project.order(name.asc()).load::<Project>(db_conn).unwrap()
    }

    fn fetch_projects(&mut self) {
        let s = self.model.sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                let prjs = Self::load_projects(sql_conn);
                s.send(prjs).unwrap();
            }))
            .unwrap();
    }

    fn update_projects_list(&mut self) {
        for child in self.widgets.project_list.children() {
            self.widgets.project_list.remove(&child);
        }
        self.model.children_widgets.clear();
        for project in &self.model.projects {
            let child = self
                .widgets
                .project_list
                .add_widget::<ProjectBadge>(project.clone());
            relm::connect!(
                child@ProjectBadgeMsg::Activate(ref project),
                self.model.relm,
                Msg::ProjectActivated((project.clone(), UpdateParents::Yes))
            );
            relm::connect!(
                child@ProjectBadgeMsg::MouseEnterProject(id),
                self.model.relm,
                Msg::MouseEnterProject(id)
            );
            relm::connect!(
                child@ProjectBadgeMsg::MouseLeaveProject(id),
                self.model.relm,
                Msg::MouseLeaveProject(id)
            );
            self.model.children_widgets.push(child);
        }
        let add_btn = gtk::Button::builder()
            .always_show_image(true)
            .image(&gtk::Image::from_icon_name(
                Some("list-add-symbolic"),
                gtk::IconSize::Menu,
            ))
            .relief(gtk::ReliefStyle::None)
            .build();
        add_btn.show();
        if self.model.projects.is_empty() {
            add_btn.style_context().add_class("suggested-action");
        }
        relm::connect!(
            self.model.relm,
            add_btn,
            connect_clicked(_),
            Msg::AddProject
        );
        self.widgets.project_list.add(&add_btn);
        if let Some(pid) = self.model.active_project_id {
            if let Some(p) = self.model.projects.iter().find(|p| p.id == pid) {
                self.model
                    .relm
                    .stream()
                    .emit(Msg::ProjectActivated((p.clone(), UpdateParents::No)));
                return;
            }
        }
        // if we have projects, select the first one
        if let Some(prj) = self.model.projects.first() {
            self.model
                .relm
                .stream()
                .emit(Msg::ProjectActivated((prj.clone(), UpdateParents::Yes)));
        }
    }

    view! {
        #[name="scroll"]
        gtk::ScrolledWindow {
            margin_top: 5,
            margin_start: 2,
            margin_end: 2,
            #[name="project_list"]
            gtk::Box {
                orientation: gtk::Orientation::Vertical
            }
        }
    }
}
