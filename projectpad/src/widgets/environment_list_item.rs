use gtk::prelude::*;
use projectpadsql::models::EnvironmentType;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    env_type: EnvironmentType,
}

#[widget]
impl Widget for EnvironmentListItem {
    fn model(relm: &relm::Relm<Self>, env_type: EnvironmentType) -> Model {
        Model { env_type }
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        gtk::Box {
            spacing: 10,
            orientation: gtk::Orientation::Vertical,
            gtk::Box {
                spacing: 10,
                gtk::Label {
                    markup: &("<b>".to_string() + &self.model.env_type.to_string() + "</b>")
                },
            },
        }
    }
}
