use super::dialogs::standard_dialogs;
use super::search_view::PROJECT_FILTER_PREFIX;
use crate::icons::Icon;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

const SHORTCUTS_UI: &str = include_str!("shortcuts.ui");

#[derive(Msg)]
pub enum Msg {
    DisplayAbout,
    DisplayShortcuts,
    SearchClicked,
    SearchActiveChanged(bool),
    SearchTextChanged(String),
    SearchTextChangedFromElsewhere((String, gdk::EventKey)),
    EnterOrUpdateSearchProject,
}

pub struct Model {
    relm: relm::Relm<WinTitleBar>,
    search_toggle_signal: Option<glib::SignalHandlerId>,
    menu_popover: gtk::Popover,
}

#[widget]
impl Widget for WinTitleBar {
    fn init_view(&mut self) {
        let relm = self.model.relm.clone();
        self.model.search_toggle_signal = Some(self.search_toggle.connect_toggled(move |_| {
            relm.stream().emit(Msg::SearchClicked);
        }));
        self.init_menu_popover();
    }

    fn init_menu_popover(&mut self) {
        let vbox = gtk::BoxBuilder::new()
            .margin(10)
            .orientation(gtk::Orientation::Vertical)
            .build();
        let shortcuts_btn = gtk::ModelButtonBuilder::new()
            .label("Keyboard Shortcuts")
            .build();
        relm::connect!(
            self.model.relm,
            &shortcuts_btn,
            connect_clicked(_),
            Msg::DisplayShortcuts
        );
        vbox.add(&shortcuts_btn);
        let about_btn = gtk::ModelButtonBuilder::new()
            .label("About Projectpad")
            .build();
        relm::connect!(
            self.model.relm,
            &about_btn,
            connect_clicked(_),
            Msg::DisplayAbout
        );
        vbox.add(&about_btn);
        vbox.show_all();
        self.model.menu_popover.add(&vbox);
        self.menu_button.set_popover(Some(&self.model.menu_popover));
    }

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {
            relm: relm.clone(),
            search_toggle_signal: None,
            menu_popover: gtk::Popover::new(None::<&gtk::MenuButton>),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::DisplayAbout => Self::display_about(),
            Msg::DisplayShortcuts => self.display_shortcuts(),
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

    fn display_about() {
        let dlg = gtk::AboutDialogBuilder::new()
            .name("Projectpad")
            .version(env!("CARGO_PKG_VERSION"))
            .logo_icon_name(Icon::APP_ICON.name())
            .website("https://github.com/emmanueltouzery/projectpad2/")
            .comments("Manage secret credentials and server information that you need to handle as a software developer")
            .build();
        dlg.run();
        dlg.close();
    }

    fn display_shortcuts(&self) {
        let win = gtk::Builder::from_string(SHORTCUTS_UI)
            .get_object::<gtk::Window>("shortcuts")
            .unwrap();
        win.set_title("Keyboard Shortcuts");
        win.set_transient_for(Some(&standard_dialogs::get_main_window(
            self.header_bar.clone().upcast::<gtk::Widget>(),
        )));
        win.show();
    }

    fn enter_or_update_search_project(&self) {
        let cur_text = self.search_entry.get_text().to_string();
        if let Some(index) = cur_text.find(PROJECT_FILTER_PREFIX) {
            let start_idx = index + PROJECT_FILTER_PREFIX.len();
            self.search_entry.set_position(start_idx as i32);
            let end_idx = cur_text[start_idx..]
                .find(" ")
                .map(|i| (start_idx + i) as i32)
                .unwrap_or(-1);
            self.search_entry.select_region(start_idx as i32, end_idx);
        } else if cur_text.is_empty() {
            self.search_entry.set_text(PROJECT_FILTER_PREFIX);
            self.search_entry
                .set_position(PROJECT_FILTER_PREFIX.len() as i32);
        } else {
            self.search_entry
                .set_text(&format!("{} {}", cur_text, PROJECT_FILTER_PREFIX));
            self.search_entry
                .set_position(cur_text.len() as i32 + PROJECT_FILTER_PREFIX.len() as i32 + 1);
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
            #[name="menu_button"]
            gtk::MenuButton {
                image: Some(&gtk::Image::from_icon_name(Some("open-menu-symbolic"), gtk::IconSize::Menu)),
                child: {
                    pack_type: gtk::PackType::End
                },
            },
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
