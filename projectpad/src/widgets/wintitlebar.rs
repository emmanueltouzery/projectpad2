use super::dialogs::preferences::Msg as PreferencesMsg;
use super::dialogs::preferences::Preferences;
use super::dialogs::standard_dialogs;
use super::search_view::PROJECT_FILTER_PREFIX;
use crate::config::Config;
use crate::icons::Icon;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use relm::{init, Component, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

const SHORTCUTS_UI: &str = include_str!("shortcuts.ui");

#[derive(Msg)]
pub enum Msg {
    DisplayPreferences,
    DisplayShortcuts,
    DisplayHelp,
    DisplayAbout,
    ConfigUpdated(Box<Config>),
    SearchEnable,
    SearchClicked,
    SearchActiveChanged(bool),
    SearchTextChanged(String),
    SearchTextChangedFromElsewhere((String, gdk::EventKey)),
    EnterOrUpdateSearchProject,
}

pub struct Model {
    relm: relm::Relm<WinTitleBar>,
    db_sender: mpsc::Sender<SqlFunc>,
    search_toggle_signal: Option<glib::SignalHandlerId>,
    menu_popover: gtk::Popover,
    prefs_win: Option<Component<Preferences>>,
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

        let left_align_menu = |menu: &gtk::ModelButton| {
            menu.get_child()
                .and_then(|c| c.dynamic_cast::<gtk::Label>().ok())
                .unwrap()
                .set_xalign(0.0)
        };

        let preferences_btn = gtk::ModelButtonBuilder::new().label("Preferences").build();
        left_align_menu(&preferences_btn);
        relm::connect!(
            self.model.relm,
            &preferences_btn,
            connect_clicked(_),
            Msg::DisplayPreferences
        );
        vbox.add(&preferences_btn);

        let shortcuts_btn = gtk::ModelButtonBuilder::new()
            .label("Keyboard Shortcuts")
            .build();
        left_align_menu(&shortcuts_btn);
        relm::connect!(
            self.model.relm,
            &shortcuts_btn,
            connect_clicked(_),
            Msg::DisplayShortcuts
        );
        vbox.add(&shortcuts_btn);

        let help_btn = gtk::ModelButtonBuilder::new()
            .label("Help")
            .hexpand(true)
            .build();
        left_align_menu(&help_btn);
        relm::connect!(
            self.model.relm,
            &help_btn,
            connect_clicked(_),
            Msg::DisplayHelp
        );
        vbox.add(&help_btn);

        let about_btn = gtk::ModelButtonBuilder::new()
            .label("About Projectpad")
            .hexpand(true)
            .build();
        left_align_menu(&about_btn);
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

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        Model {
            relm: relm.clone(),
            db_sender,
            search_toggle_signal: None,
            menu_popover: gtk::Popover::new(None::<&gtk::MenuButton>),
            prefs_win: None,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::DisplayPreferences => self.display_preferences(),
            Msg::DisplayShortcuts => self.display_shortcuts(),
            Msg::DisplayAbout => Self::display_about(),
            Msg::SearchClicked => {
                let new_visible = self.search_toggle.get_active();
                self.search_entry.grab_focus();
                self.model
                    .relm
                    .stream()
                    .emit(Msg::SearchActiveChanged(new_visible));
            }
            Msg::SearchEnable => {
                self.search_toggle.set_active(true);
                self.model.relm.stream().emit(Msg::SearchClicked);
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
            Msg::DisplayHelp => {
                if let Err(e) = gtk::show_uri_on_window(
                    Some(&standard_dialogs::get_main_window(
                        self.header_bar.clone().upcast::<gtk::Widget>(),
                    )),
                    "https://github.com/emmanueltouzery/projectpad2/wiki/Help",
                    0,
                ) {
                    eprintln!("Error showing help: {:?}", e);
                }
            }
            Msg::ConfigUpdated(_) => {
                // this is meant for win... we emit here, not interested by it ourselves
            }
        }
    }

    fn display_preferences(&mut self) {
        let main_win =
            standard_dialogs::get_main_window(self.header_bar.clone().upcast::<gtk::Widget>());
        self.model.prefs_win = Some(
            init::<Preferences>((main_win.clone(), self.model.db_sender.clone()))
                .expect("error initializing the preferences window"),
        );
        let prefs_win = self.model.prefs_win.as_ref().unwrap();
        relm::connect!(prefs_win@PreferencesMsg::ConfigUpdated(ref cfg),
                               self.model.relm, Msg::ConfigUpdated(cfg.clone()));
        prefs_win.widget().set_transient_for(Some(&main_win));
        prefs_win
            .widget()
            .set_position(gtk::WindowPosition::CenterOnParent);
        prefs_win.widget().set_modal(true);
        prefs_win.widget().show();
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
                .find(' ')
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
