use gtk::prelude::*;
use relm::{Component, Widget};
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    Quit,
}

pub struct Model {}

#[widget]
impl Widget for Win {
    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {}
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Quit => gtk::main_quit(),
        }
    }

    view! {
        gtk::Window {
            gtk::Box {},
            delete_event(_, _) => (Msg::Quit, Inhibit(false)),
        }
    }
}
