use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    FilterChanged(Option<String>),
}

pub struct Model {
    filter: Option<String>,
}

#[widget]
impl Widget for SearchView {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model { filter: None }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::FilterChanged(filter) => {
                self.model.filter = filter;
                self.refresh_view();
            }
        }
    }

    fn refresh_view(&self) {
        println!("refresh");
    }

    view! {
        gtk::ScrolledWindow {
            #[name="search_result_box"]
            gtk::Box {
                orientation: gtk::Orientation::Vertical,
                spacing: 10,
            }
        }
    }
}
