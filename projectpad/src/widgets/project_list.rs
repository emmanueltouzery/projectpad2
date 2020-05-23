use super::win::Project;
use gtk::prelude::*;
use relm::{Component, Widget};
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    projects: Vec<Project>,
}

#[widget]
impl Widget for ProjectList {
    fn model(relm: &relm::Relm<Self>, projects: Vec<Project>) -> Model {
        Model { projects }
    }

    fn update(&mut self, event: Msg) {}

    view! {
        gtk::Box {}
    }
}
