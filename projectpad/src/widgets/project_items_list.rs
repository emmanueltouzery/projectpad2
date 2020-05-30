use super::project_poi_list_item::ProjectPoiListItem;
use super::win::ProjectPoi;
use gtk::prelude::*;
use projectpadsql::models::Project;
use relm::{ContainerWidget, Widget};
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    EventSelected,
}

pub struct Model {
    relm: relm::Relm<ProjectItemsList>,
    project: Option<Project>,
    project_pois: Vec<ProjectPoi>,
}

#[widget]
impl Widget for ProjectItemsList {
    fn init_view(&mut self) {
        self.update_items_list();
        relm::connect!(
            self.model.relm,
            self.project_items_list,
            connect_row_selected(_, _),
            Msg::EventSelected
        );
    }

    fn model(relm: &relm::Relm<Self>, project: (Option<Project>, Vec<ProjectPoi>)) -> Model {
        Model {
            relm: relm.clone(),
            project: project.0,
            project_pois: project.1,
        }
    }

    fn update(&mut self, event: Msg) {}

    fn update_items_list(&mut self) {
        for child in self.project_items_list.get_children() {
            self.project_items_list.remove(&child);
        }
        for project_poi in &self.model.project_pois {
            let _child = self
                .project_items_list
                .add_widget::<ProjectPoiListItem>(project_poi.clone());
        }
    }

    view! {
        #[name="project_items_list"]
        gtk::ListBox {}
    }
}
