use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    pub text: String,
    pub secondary_desc: Option<String>,
}

#[widget]
impl Widget for ProjectPoiListItem {
    fn model(relm: &relm::Relm<Self>, model: Model) -> Model {
        model
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        gtk::Box {
            spacing: 10,
            orientation: gtk::Orientation::Vertical,
            gtk::Box {
                spacing: 10,
                gtk::Label {
                    text: &self.model.text
                },
                // gtk::Label {
                //     text: &self.model.project_poi.address
                // }
            },
            gtk::Label {
                text: self.model.secondary_desc.as_deref().unwrap_or("")
            }
        }
    }
}
