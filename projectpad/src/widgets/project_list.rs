use super::project_badge::Msg as ProjectBadgeMsg;
use super::project_badge::ProjectBadge;
use gtk::prelude::*;
use projectpadsql::models::Project;
use relm::ContainerWidget;
use relm::{Component, Widget};
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    ProjectActivated(Project),
}

pub struct Model {
    relm: relm::Relm<ProjectList>,
    projects: Vec<Project>,
    // need to keep hold of the children widgets
    children_widgets: Vec<Component<ProjectBadge>>,
}

#[widget]
impl Widget for ProjectList {
    fn init_view(&mut self) {
        self.update_projects_list();
    }

    fn model(relm: &relm::Relm<Self>, projects: Vec<Project>) -> Model {
        Model {
            relm: relm.clone(),
            projects,
            children_widgets: vec![],
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectActivated(ref _project) => {}
        }
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
                ProjectBadgeMsg::ActiveProjectChanged(project.clone()));
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
