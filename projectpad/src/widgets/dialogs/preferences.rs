use crate::config::Config;
use gtk::prelude::*;
use relm::{Component, Widget};
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum HeaderMsg {}

#[widget]
impl Widget for Header {
    fn model() {}

    fn update(&mut self, _event: HeaderMsg) {}

    view! {
        gtk::HeaderBar {
            title: Some("Preferences"),
            show_close_button: true,
        }
    }
}

#[derive(Msg)]
pub enum Msg {
    DarkThemeToggled(bool),
    ConfigUpdated(Box<Config>),
}

pub struct Model {
    relm: relm::Relm<Preferences>,
    prefer_dark_theme: bool,
    header: Component<Header>,
    win: gtk::Window,
    config: Config,
}

#[widget]
impl Widget for Preferences {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, win: gtk::Window) -> Model {
        let config = Config::read_config();
        let header = relm::init(()).expect("header");
        Model {
            relm: relm.clone(),
            prefer_dark_theme: config.prefer_dark_theme,
            header,
            config,
            win,
        }
    }

    fn update_config(&self) {
        self.model.config.save_config(&self.model.win);
        self.model
            .relm
            .stream()
            .emit(Msg::ConfigUpdated(Box::new(self.model.config.clone())));
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::DarkThemeToggled(t) => {
                gtk::Settings::get_default()
                    .unwrap()
                    .set_property_gtk_application_prefer_dark_theme(t);
                self.model.config.prefer_dark_theme = t;
                self.update_config();
            }
            Msg::ConfigUpdated(_) => {
                // meant for my parent, not for me
            }
        }
    }

    view! {
        #[name="prefs_win"]
        gtk::Window {
            titlebar: Some(self.model.header.widget()),
            property_default_width: 600,
            property_default_height: 200,
            gtk::Box {
                orientation: gtk::Orientation::Vertical,
                margin_top: 10,
                margin_start: 6,
                margin_end: 6,
                margin_bottom: 6,
                spacing: 6,
                gtk::CheckButton {
                    label: "Prefer dark theme",
                    active: self.model.prefer_dark_theme,
                    toggled(t) => Msg::DarkThemeToggled(t.get_active())
                },
            },
            // key_press_event(_, key) => (Msg::KeyPress(key.clone()), Inhibit(false)), // just for the ESC key.. surely there's a better way..
        }
    }
}
