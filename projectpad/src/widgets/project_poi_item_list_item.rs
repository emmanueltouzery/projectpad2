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
    fn init_view(&mut self) {
        self.items_frame
            .get_style_context()
            .add_class("items_frame");
    }

    fn model(relm: &relm::Relm<Self>, project_poi_item: ProjectPoiItem) -> Model {
        Model { project_poi_item }
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        #[name="items_frame"]
        gtk::Frame {
            margin_start: 20,
            margin_end: 20,
            margin_top: 20,
            gtk::Box {
                spacing: 10,
                orientation: gtk::Orientation::Vertical,
                gtk::Label {
                    text: &self.model.project_poi_item.name
                }
            }
        }
    }
}
