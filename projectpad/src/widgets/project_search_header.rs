use gtk::prelude::*;
use projectpadsql::models::Project;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    project: Project,
}

#[widget]
impl Widget for ProjectSearchHeader {
    fn init_view(&mut self) {}

    fn model(_relm: &relm::Relm<Self>, project: Project) -> Model {
        Model { project }
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        gtk::Box {
            gtk::Label {
                text: &self.model.project.name
            }
        }
    }
}
