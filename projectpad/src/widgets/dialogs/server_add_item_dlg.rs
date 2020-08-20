use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg, Debug)]
pub enum Msg {}

pub struct Model {
    relm: relm::Relm<ServerAddItemDialog>,
}

#[widget]
impl Widget for ServerAddItemDialog {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model { relm: relm.clone() }
    }

    fn update(&mut self, event: Msg) {}

    view! {
        gtk::Stack {
            gtk::Box {
                margin_top: 10,
                margin_start: 10,
                margin_end: 10,
                margin_bottom: 10,
                spacing: 3,
                orientation: gtk::Orientation::Vertical,
                gtk::RadioButton {
                    label: "Add point of interest",
                },
            }
        }
    }
}
