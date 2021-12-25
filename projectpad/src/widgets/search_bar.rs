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

pub fn note_search_next<T>(textview: &T, note_search_text: &Option<String>)
where
    T: gtk::TextViewExt,
{
    let buffer = textview.get_buffer().unwrap();
    if let (Some((_start, end)), Some(search)) =
        (buffer.get_selection_bounds(), note_search_text.clone())
    {
        apply_search(
            textview,
            end.forward_search(&search, gtk::TextSearchFlags::all(), None),
        );
    }
}

pub fn note_search_previous<T>(textview: &T, note_search_text: &Option<String>)
where
    T: gtk::TextViewExt,
{
    let buffer = textview.get_buffer().unwrap();
    if let (Some((start, _end)), Some(search)) =
        (buffer.get_selection_bounds(), note_search_text.clone())
    {
        apply_search(
            textview,
            start.backward_search(&search, gtk::TextSearchFlags::all(), None),
        );
    }
}

pub fn apply_search<T>(textview: &T, range: Option<(gtk::TextIter, gtk::TextIter)>)
where
    T: gtk::TextViewExt,
{
    if let Some((mut start, end)) = range {
        textview.get_buffer().unwrap().select_range(&start, &end);
        textview.scroll_to_iter(&mut start, 0.0, false, 0.0, 0.0);
    }
}

pub fn note_search_change<T>(textview: &T, text: &str)
where
    T: gtk::TextViewExt,
{
    apply_search(
        textview,
        textview
            .get_buffer()
            .unwrap()
            .get_start_iter()
            .forward_search(text, gtk::TextSearchFlags::all(), None),
    );
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
                self.widgets.revealer.set_reveal_child(r);
                if r {
                    self.widgets.search_entry.grab_focus();
                }
            }
            Msg::KeyRelease(e) => {
                if e.get_keyval() == gdk::keys::constants::Escape
                    && self.widgets.revealer.get_reveal_child()
                {
                    self.model.relm.stream().emit(Msg::Reveal(false));
                } else if is_plaintext_key(&e) {
                    self.model.relm.stream().emit(Msg::SearchChanged(
                        self.widgets.search_entry.get_text().to_string(),
                    ));
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
            #[style_class="search_frame"]
            gtk::Frame {
                hexpand: false,
                vexpand: false,
                margin_end: 15,
                // https://developer.gnome.org/Buttons/#Linked_buttons
                #[style_class="linked"]
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
