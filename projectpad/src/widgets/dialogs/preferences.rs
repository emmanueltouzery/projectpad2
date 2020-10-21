use super::change_db_password_dlg;
use super::change_db_password_dlg::ChangeDbPasswordDialog;
use super::change_db_password_dlg::Msg as MsgChangeDbPassword;
use super::standard_dialogs;
use crate::config::Config;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use relm::{Component, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

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
    GotStorePassInKeyring(bool),
    RemovePasswordFromKeyring,
    ChangeDbPassword,
    KeyPress(gdk::EventKey),
    ConfigUpdated(Box<Config>),
    ChangedPass(gtk::Dialog),
}

pub struct Model {
    relm: relm::Relm<Preferences>,
    db_sender: mpsc::Sender<SqlFunc>,
    prefer_dark_theme: bool,
    header: Component<Header>,
    win: gtk::Window,
    config: Config,
    pass_keyring_sender: relm::Sender<bool>,
    _pass_keyring_channel: relm::Channel<bool>,
    change_db_password_dlg: Option<Component<ChangeDbPasswordDialog>>,
}

#[widget]
impl Widget for Preferences {
    fn init_view(&mut self) {
        self.remove_from_keyring
            .get_style_context()
            .add_class("destructive-action");
        self.load_keyring_pass_state();
        self.section_title1
            .get_style_context()
            .add_class("section_title");
        self.section_title2
            .get_style_context()
            .add_class("section_title");
    }

    fn model(relm: &relm::Relm<Self>, params: (gtk::Window, mpsc::Sender<SqlFunc>)) -> Model {
        let (win, db_sender) = params;
        let config = Config::read_config();
        let header = relm::init(()).expect("header");
        let stream = relm.stream().clone();
        let (_pass_keyring_channel, pass_keyring_sender) =
            relm::Channel::new(move |r: bool| stream.emit(Msg::GotStorePassInKeyring(r)));
        Model {
            relm: relm.clone(),
            db_sender,
            prefer_dark_theme: config.prefer_dark_theme,
            header,
            config,
            win,
            pass_keyring_sender,
            _pass_keyring_channel,
            change_db_password_dlg: None,
        }
    }

    fn load_keyring_pass_state(&self) {
        // abusing a little db_sender here. I need a thread to run blocking
        // stuff, nothing to do with sql, but it serves my purpose.
        let s = self.model.pass_keyring_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |_| {
                s.send(projectpadsql::get_pass_from_keyring().is_some())
                    .unwrap();
            }))
            .unwrap();
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
            Msg::GotStorePassInKeyring(t) => {
                self.remove_from_keyring.set_sensitive(t);
            }
            Msg::DarkThemeToggled(t) => {
                gtk::Settings::get_default()
                    .unwrap()
                    .set_property_gtk_application_prefer_dark_theme(t);
                self.model.config.prefer_dark_theme = t;
                self.update_config();
            }
            Msg::RemovePasswordFromKeyring => {
                projectpadsql::clear_pass_from_keyring().unwrap();
                self.load_keyring_pass_state();
            }
            Msg::ChangeDbPassword => {
                let change_pwd_contents =
                    relm::init::<ChangeDbPasswordDialog>(self.model.db_sender.clone())
                        .expect("error initializing the change db password dialog");
                let dialog = standard_dialogs::modal_dialog(
                    self.prefs_win.clone().upcast::<gtk::Widget>(),
                    600,
                    200,
                    "Change database password".to_string(),
                );
                let d_c = change_pwd_contents.clone();
                let (dialog, component, btn) = standard_dialogs::prepare_custom_dialog(
                    dialog,
                    change_pwd_contents,
                    move |_| {
                        d_c.emit(change_db_password_dlg::Msg::OkPressed);
                    },
                );
                btn.set_label("Apply");
                let d = dialog.clone();
                relm::connect!(component@MsgChangeDbPassword::SuccessfullyChangedPass,
                               self.model.relm, Msg::ChangedPass(d.clone()));
                self.model.change_db_password_dlg = Some(component);
                dialog.show();
            }
            Msg::KeyPress(key) => {
                if key.get_keyval() == gdk::keys::constants::Escape {
                    self.prefs_win.close();
                }
            }
            Msg::ChangedPass(dialog) => {
                dialog.close();
                self.model.change_db_password_dlg = None;
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
                margin_start: 30,
                margin_end: 30,
                margin_bottom: 6,
                spacing: 6,
                #[name="section_title1"]
                gtk::Label {
                    text: "User interface",
                    xalign: 0.0
                },
                gtk::CheckButton {
                    label: "Prefer dark theme",
                    active: self.model.prefer_dark_theme,
                    toggled(t) => Msg::DarkThemeToggled(t.get_active())
                },
                #[name="section_title2"]
                gtk::Label {
                    text: "Database password",
                    xalign: 0.0
                },
                #[name="remove_from_keyring"]
                gtk::Button {
                    label: "Remove password from keyring",
                    halign: gtk::Align::Start,
                    sensitive: false,
                    clicked => Msg::RemovePasswordFromKeyring
                },
                #[name="change_password"]
                gtk::Button {
                    label: "Change database password",
                    halign: gtk::Align::Start,
                    clicked => Msg::ChangeDbPassword
                },
            },
            key_press_event(_, key) => (Msg::KeyPress(key.clone()), Inhibit(false)), // just for the ESC key.. surely there's a better way..
        }
    }
}
