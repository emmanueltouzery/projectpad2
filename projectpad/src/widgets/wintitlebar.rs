use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    SearchClicked,
    SearchActiveChanged(bool),
    SearchTextChanged(String),
    SearchTextChangedFromElsewhere((String, gdk::EventKey)),
}

pub struct Model {
    relm: relm::Relm<WinTitleBar>,
    search_toggle_signal: Option<glib::SignalHandlerId>,
}

#[widget]
impl Widget for WinTitleBar {
    fn init_view(&mut self) {
        let relm = self.model.relm.clone();
        self.model.search_toggle_signal = Some(self.search_toggle.connect_toggled(move |_| {
            relm.stream().emit(Msg::SearchClicked);
        }));
    }

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {
            relm: relm.clone(),
            search_toggle_signal: None,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::SearchClicked => {
                let new_visible = self.search_toggle.get_active();
                self.search_entry.grab_focus();
                self.model
                    .relm
                    .stream()
                    .emit(Msg::SearchActiveChanged(new_visible));
            }
            Msg::SearchActiveChanged(is_active) => {
                self.search_toggle.set_active(is_active);
                self.search_entry.set_visible(is_active);
            }
            Msg::SearchTextChanged(_) => {} // meant for my parent
            Msg::SearchTextChangedFromElsewhere((txt, evt)) => {
                if !self.search_toggle.get_active() {
                    // we want to block the signal of the search button toggle,
                    // because when you click the search button we set the focus
                    // and select the search text. if we did that when search
                    // is triggered by someone typing, the first letter would
                    // be lost when typing the second letter, due to the selection
                    // so we block the search button toggle signal & handle things
                    // by hand.
                    self.search_toggle
                        .block_signal(&self.model.search_toggle_signal.as_ref().unwrap());
                    self.search_entry.set_visible(true);
                    self.search_toggle.set_active(true);
                    self.search_entry.grab_focus_without_selecting();

                    self.search_entry.set_text(&txt);
                    self.search_toggle
                        .unblock_signal(&self.model.search_toggle_signal.as_ref().unwrap());
                    self.search_entry.set_position(1);
                }
            }
        }
    }

    view! {
        #[name="header_bar"]
        gtk::HeaderBar {
            show_close_button: true,
            title: Some("Projectpad"),
            #[name="search_toggle"]
            gtk::ToggleButton {
                image: Some(&gtk::Image::new_from_icon_name(Some("edit-find-symbolic"), gtk::IconSize::Menu)),
                child: {
                    pack_type: gtk::PackType::End
                },
            },
            #[name="search_entry"]
            gtk::SearchEntry {
                visible: false,
                child: {
                    pack_type: gtk::PackType::End
                },
                changed(entry) => Msg::SearchTextChanged(entry.get_text()
                                                         .map(|t| t.to_string()).unwrap_or_else(|| "".to_string()))
            },
        }
    }
}
