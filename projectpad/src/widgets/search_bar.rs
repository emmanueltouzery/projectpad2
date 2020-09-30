// NOTE it turns out gtk has a SearchBar OOB
// https://gtk-rs.org/docs/gtk/struct.SearchBar.html
// maybe migrate to that? Not sure what it brings to the table...
use super::win::is_plaintext_key;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    Reveal(bool),
    KeyRelease(gdk::EventKey),
    SearchChanged(String),
    SearchPrevious,
    SearchNext,
}

pub struct Model {
    relm: relm::Relm<SearchBar>,
}

#[widget]
impl Widget for SearchBar {
    fn init_view(&mut self) {
        // https://developer.gnome.org/Buttons/#Linked_buttons
        self.search_box.get_style_context().add_class("linked");

        self.frame.get_style_context().add_class("search_frame");
    }

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
            Msg::KeyRelease(e) => {
                if is_plaintext_key(&e) {
                    self.model
                        .relm
                        .stream()
                        .emit(Msg::SearchChanged(self.search_entry.get_text().to_string()));
                }
            }
            // meant for my parent
            Msg::SearchNext => {}
            Msg::SearchPrevious => {}
            Msg::SearchChanged(_) => {}
        }
    }

    view! {
        #[name="revealer"]
        gtk::Revealer {
            valign: gtk::Align::Start,
            halign: gtk::Align::End,
            #[name="frame"]
            gtk::Frame {
                hexpand: false,
                vexpand: false,
                margin_end: 15,
                #[name="search_box"]
                gtk::Box {
                    #[name="search_entry"]
                    gtk::SearchEntry {
                        width_chars: 25,
                        margin_top: 5,
                        margin_bottom: 5,
                        margin_start: 5,
                        key_release_event(_, event) => (Msg::KeyRelease(event.clone()), Inhibit(false)),
                    },
                    gtk::Button {
                        margin_top: 5,
                        margin_bottom: 5,
                        always_show_image: true,
                        image: Some(&gtk::Image::from_icon_name(
                            Some("go-up-symbolic"), gtk::IconSize::Menu)),
                        button_press_event(_, _) => (Msg::SearchPrevious, Inhibit(false)),
                    },
                    gtk::Button {
                        margin_top: 5,
                        margin_bottom: 5,
                        margin_end: 5,
                        always_show_image: true,
                        image: Some(&gtk::Image::from_icon_name(
                            Some("go-down-symbolic"), gtk::IconSize::Menu)),
                        button_press_event(_, _) => (Msg::SearchNext, Inhibit(false)),
                    },
                }
            }
        }
    }
}
