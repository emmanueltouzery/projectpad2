use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    Reveal(bool),
    SearchEntryChanged,
    SearchChanged(String),
}

pub struct Model {
    relm: relm::Relm<SearchBar>,
}

#[widget]
impl Widget for SearchBar {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model { relm: relm.clone() }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Reveal(r) => {
                self.revealer.set_reveal_child(r);
                if r {
                    self.search_entry.grab_focus();
                }
            }
            Msg::SearchEntryChanged => {
                self.model
                    .relm
                    .stream()
                    .emit(Msg::SearchChanged(self.search_entry.get_text().to_string()));
            }
            // meant for my parent
            Msg::SearchChanged(_) => {}
        }
    }

    view! {
        #[name="revealer"]
        gtk::Revealer {
            valign: gtk::Align::Start,
            halign: gtk::Align::End,
            gtk::Frame {
                hexpand: false,
                vexpand: false,
                margin_end: 15,
                #[name="search_entry"]
                gtk::SearchEntry {
                    width_chars: 25,
                    margin_top: 5,
                    margin_bottom: 5,
                    margin_start: 5,
                    margin_end: 5,
                    key_release_event(_, _) => (Msg::SearchEntryChanged, Inhibit(false)),
                }
            }
        }
    }
}
