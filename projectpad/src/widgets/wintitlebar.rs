use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {}

#[widget]
impl Widget for WinTitleBar {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {}
    }

    fn update(&mut self, event: Msg) {}

    view! {
        #[name="header_bar"]
        gtk::HeaderBar {
            show_close_button: true,
            title: Some("Projectpad"),
        }
    }
}
