use super::win::{ProjectPoi, ProjectPoiItem};
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    cur_project_item: Option<ProjectPoi>,
    project_poi_items: Vec<ProjectPoiItem>,
}

#[widget]
impl Widget for ProjectPoiContents {
    fn model(relm: &relm::Relm<Self>, params: (Option<ProjectPoi>, Vec<ProjectPoiItem>)) -> Model {
        Model {
            cur_project_item: params.0,
            project_poi_items: params.1,
        }
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        gtk::Box {
        }
    }
}
