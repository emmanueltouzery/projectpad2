use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    SearchClicked,
    SearchActiveChanged(bool),
    SearchTextChanged(String),
    SearchTextChangedFromElsewhere((String, gdk::EventKey)),
    EnterOrUpdateSearchProject,
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
            Msg::SearchTextChangedFromElsewhere((txt, _evt)) => {
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
            Msg::EnterOrUpdateSearchProject => {
                self.enter_or_update_search_project();
            }
        }
    }

    fn enter_or_update_search_project(&self) {
        let cur_text = self.search_entry.get_text().to_string();
        if let Some(index) = cur_text.find("prj:") {
            let start_idx = index + 4;
            self.search_entry.set_position(start_idx as i32);
            let end_idx = cur_text[start_idx..]
                .find(" ")
                .map(|i| (start_idx + i) as i32)
                .unwrap_or(-1);
            self.search_entry.select_region(start_idx as i32, end_idx);
        } else if cur_text.is_empty() {
            self.search_entry.set_text("prj:");
            self.search_entry.set_position(4);
        } else {
            self.search_entry.set_text(&format!("{} prj:", cur_text));
            self.search_entry.set_position(cur_text.len() as i32 + 5);
        }
        self.search_toggle
            .block_signal(&self.model.search_toggle_signal.as_ref().unwrap());
        self.search_toggle.set_active(true);
        self.search_entry.set_visible(true);
        self.search_entry.grab_focus_without_selecting();
        self.search_toggle
            .unblock_signal(&self.model.search_toggle_signal.as_ref().unwrap());
    }

    view! {
        #[name="header_bar"]
        gtk::HeaderBar {
            show_close_button: true,
            title: Some("Projectpad"),
            #[name="search_toggle"]
            gtk::ToggleButton {
                image: Some(&gtk::Image::from_icon_name(Some("edit-find-symbolic"), gtk::IconSize::Menu)),
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
                changed(entry) => Msg::SearchTextChanged(entry.get_text().to_string())
            },
        }
    }
}
