use gtk::prelude::*;
use projectpadsql::models::Server;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    project_poi: Server,
}

#[widget]
impl Widget for ProjectPoiListItem {
    fn model(relm: &relm::Relm<Self>, project_poi: Server) -> Model {
        Model { project_poi }
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        gtk::Box {
            spacing: 10,
            orientation: gtk::Orientation::Vertical,
            gtk::Box {
                spacing: 10,
                gtk::Label {
                    text: &self.model.project_poi.desc
                },
                // gtk::Label {
                //     text: &self.model.project_poi.address
                // }
            },
            gtk::Label {
                text: &self.model.project_poi.username
            }
        }
    }
}
