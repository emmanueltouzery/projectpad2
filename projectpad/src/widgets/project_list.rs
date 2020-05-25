use super::project_badge::ProjectBadge;
use super::win::Project;
use gtk::prelude::*;
use relm::ContainerWidget;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    projects: Vec<Project>,
}

#[widget]
impl Widget for ProjectList {
    fn init_view(&mut self) {
        // ???
        self.project_list.get_style_context().add_class("item_list");
        self.update_projects_list();
    }

    fn model(relm: &relm::Relm<Self>, projects: Vec<Project>) -> Model {
        Model { projects }
    }

    fn update(&mut self, event: Msg) {}

    fn update_projects_list(&mut self) {
        for child in self.project_list.get_children() {
            self.project_list.remove(&child);
        }
        for project in &self.model.projects {
            let child = self
                .project_list
                .add_widget::<ProjectBadge>(project.clone());
            std::mem::forget(child); // !!!!!!!
                                     // // this is a little confusing for me here, but somehow
                                     // // the child doesn't get notified of an event triggered
                                     // // there, but I as the parent get notified. So handle it here.
                                     // relm::connect!(
                                     //     child@EventSourceListItemMsg::ActionsClicked(ref btn),
                                     //     self.model.relm,
                                     //     Msg::ActionsClicked(btn.clone(), ep_name, cfg_name.clone())
                                     // );
        }
    }

    view! {
        #[name="project_list"]
        gtk::Box {
            orientation: gtk::Orientation::Vertical
        }
    }
}
