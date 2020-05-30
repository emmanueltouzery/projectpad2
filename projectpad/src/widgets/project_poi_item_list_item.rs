use super::win::ProjectPoiItem;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    project_poi_item: ProjectPoiItem,
}

#[widget]
impl Widget for ProjectPoiItemListItem {
    fn model(relm: &relm::Relm<Self>, project_poi_item: ProjectPoiItem) -> Model {
        Model { project_poi_item }
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        gtk::Box {
            spacing: 10,
            orientation: gtk::Orientation::Vertical,
            gtk::Label {
                text: &self.model.project_poi_item.name
            }
        }
    }
}
