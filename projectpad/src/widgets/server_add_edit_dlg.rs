use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg, Debug)]
pub enum Msg {}

pub struct Model {
    relm: relm::Relm<ServerAddEditDialog>,
}

#[widget]
impl Widget for ServerAddEditDialog {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model { relm: relm.clone() }
    }

    fn update(&mut self, msg: Msg) {}

    view! {
        gtk::Box {}
    }
}
