use super::super::keyring_helpers;
use gtk::traits::SettingsExt;
use super::super::password_field;
use super::super::password_field::Msg as PasswordFieldMsg;
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
pub enum Msg {
    DarkThemeToggled(bool),
    GotStorePassInKeyring(bool),
    RemovePasswordFromKeyring,
    RemovePasswordFromKeyringConfigCheckPass(String),
    RemovePasswordFromKeyringUserResponse(gtk::ResponseType),
    ChangeDbPassword,
    KeyPress(gdk::EventKey),
    ConfigUpdated(Box<Config>),
    ChangedPass(gtk::Dialog),
}

pub struct Model {
    relm: relm::Relm<Preferences>,
    db_sender: mpsc::Sender<SqlFunc>,
    prefer_dark_theme: bool,
    win: gtk::Window,
    config: Config,
    confirm_dialog: Option<gtk::MessageDialog>,
    confirm_ok_btn: Option<gtk::Widget>,
    pass_keyring_sender: relm::Sender<bool>,
    _pass_keyring_channel: relm::Channel<bool>,
    change_db_password_dlg: Option<Component<ChangeDbPasswordDialog>>,
    remove_pass_from_keyring_spinner: gtk::Spinner,
}

#[widget]
impl Widget for Preferences {
    fn init_view(&mut self) {
        self.load_keyring_pass_state();
        let remove_pass_btn_contents = gtk::builders::BoxBuilder::new().build();
        self.model.remove_pass_from_keyring_spinner.start();
        remove_pass_btn_contents.add(&self.model.remove_pass_from_keyring_spinner);
        remove_pass_btn_contents.add(
            &gtk::builders::LabelBuilder::new()
                .label("Remove password from keyring")
                .build(),
        );
        remove_pass_btn_contents.show_all();
        self.widgets
            .remove_from_keyring
            .add(&remove_pass_btn_contents);
        let db_folder_pathbuf = projectpadsql::config_path();
        let db_folder_path = db_folder_pathbuf.to_string_lossy();
        let db_pathbuf = projectpadsql::database_path();
        let db_path = db_pathbuf.to_string_lossy();
        self.widgets.db_location_label.set_markup(&format!(
            "The database file is in <a href=\"file://{}\">{}</a>",
            &db_folder_path, &db_path
        ));
    }

    fn model(relm: &relm::Relm<Self>, params: (gtk::Window, mpsc::Sender<SqlFunc>)) -> Model {
        let (win, db_sender) = params;
        let config = Config::read_config();
        let stream = relm.stream().clone();
        let (_pass_keyring_channel, pass_keyring_sender) =
            relm::Channel::new(move |r: bool| stream.emit(Msg::GotStorePassInKeyring(r)));
        Model {
            relm: relm.clone(),
            db_sender,
            prefer_dark_theme: config.prefer_dark_theme,
            config,
            win,
            pass_keyring_sender,
            _pass_keyring_channel,
            change_db_password_dlg: None,
            remove_pass_from_keyring_spinner: gtk::builders::SpinnerBuilder::new().build(),
            confirm_dialog: None,
            confirm_ok_btn: None,
        }
    }

    fn load_keyring_pass_state(&self) {
        // abusing a little db_sender here. I need a thread to run blocking
        // stuff, nothing to do with sql, but it serves my purpose.
        let s = self.model.pass_keyring_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |_| {
                s.send(keyring_helpers::get_pass_from_keyring().is_some())
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
                self.model.remove_pass_from_keyring_spinner.stop();
                self.model
                    .remove_pass_from_keyring_spinner
                    .set_visible(false);
                self.widgets.remove_from_keyring.set_sensitive(t);
            }
            Msg::DarkThemeToggled(t) => {
                gtk::Settings::default()
                    .unwrap()
                    .set_gtk_application_prefer_dark_theme(t);
                self.model.config.prefer_dark_theme = t;
                self.update_config();
            }
            Msg::RemovePasswordFromKeyring => {
                self.remove_pass_from_keyring();
            }
            Msg::RemovePasswordFromKeyringConfigCheckPass(entered_pass) => {
                if let Some(ok_btn) = &self.model.confirm_ok_btn {
                    ok_btn.set_sensitive(
                        change_db_password_dlg::check_db_password(&entered_pass).is_ok(),
                    );
                }
            }
            Msg::RemovePasswordFromKeyringUserResponse(resp) => {
                if let Some(dlg) = &self.model.confirm_dialog {
                    if resp == gtk::ResponseType::Yes {
                        keyring_helpers::clear_pass_from_keyring().unwrap();
                        self.load_keyring_pass_state();
                    }
                    dlg.close();
                    self.model.confirm_dialog = None;
                    self.model.confirm_ok_btn = None;
                }
            }
            Msg::ChangeDbPassword => {
                let change_pwd_contents =
                    relm::init::<ChangeDbPasswordDialog>(self.model.db_sender.clone())
                        .expect("error initializing the change db password dialog");
                let dialog = standard_dialogs::modal_dialog(
                    self.widgets.prefs_win.clone().upcast::<gtk::Widget>(),
                    600,
                    200,
                    "Change database password".to_string(),
                );
                let d_c = change_pwd_contents.stream();
                let (dialog, component, btn) = standard_dialogs::prepare_custom_dialog(
                    dialog,
                    change_pwd_contents,
                    move |_| {
                        d_c.emit(change_db_password_dlg::Msg::OkPressed);
                    },
                );
                let d = dialog.clone();
                relm::connect!(component@MsgChangeDbPassword::SuccessfullyChangedPass,
                               self.model.relm, Msg::ChangedPass(d.clone()));
                component
                    .stream()
                    .emit(MsgChangeDbPassword::GotApplyButton(btn));
                self.model.change_db_password_dlg = Some(component);
                dialog.show();
            }
            Msg::KeyPress(key) => {
                if key.keyval() == gdk::keys::constants::Escape {
                    self.widgets.prefs_win.close();
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

    fn remove_pass_from_keyring(&mut self) {
        let dialog = gtk::MessageDialog::new(
            Some(&self.widgets.prefs_win),
            gtk::DialogFlags::all(),
            gtk::MessageType::Warning,
            gtk::ButtonsType::None,
            "Are you sure to remove the keyring password?",
        );
        let entry = relm::init::<password_field::PasswordField>((
            "".to_string(),
            password_field::ActivatesDefault::Yes,
        ))
        .expect("prefs password field");

        dialog.add_button("No", gtk::ResponseType::No);
        let yes_btn = dialog.add_button("Yes", gtk::ResponseType::Yes);
        yes_btn.set_sensitive(false);

        let entry_w = entry.widget();
        entry_w.set_margin_start(25);
        entry_w.set_margin_end(25);
        dialog.content_area().add(entry_w);
        dialog.set_secondary_text(Some("Please enter the current password to confirm"));

        relm::connect!(
                entry@PasswordFieldMsg::PasswordChanged(ref p),
                self.model.relm,
                Msg::RemovePasswordFromKeyringConfigCheckPass(p.clone()));
        relm::connect!(
            self.model.relm,
            dialog,
            connect_response(_, r_id),
            Msg::RemovePasswordFromKeyringUserResponse(r_id)
        );

        self.model.confirm_dialog = Some(dialog.clone());
        self.model.confirm_ok_btn = Some(yes_btn);

        dialog.show();
    }

    view! {
        #[name="prefs_win"]
        gtk::Window {
            titlebar: view! {
                gtk::HeaderBar {
                    title: Some("Preferences"),
                    show_close_button: true,
                }
            },
            default_width: 600,
            default_height: 200,
            gtk::Box {
                orientation: gtk::Orientation::Vertical,
                margin_top: 10,
                margin_start: 30,
                margin_end: 30,
                margin_bottom: 20,
                spacing: 6,
                #[style_class="section_title"]
                gtk::Label {
                    text: "User interface",
                    xalign: 0.0,
                },
                gtk::CheckButton {
                    label: "Prefer dark theme",
                    active: self.model.prefer_dark_theme,
                    toggled(t) => Msg::DarkThemeToggled(t.is_active()),
                },
                #[style_class="section_title"]
                gtk::Label {
                    text: "Database password",
                    xalign: 0.0,
                },
                #[name="remove_from_keyring"]
                #[style_class="destructive-action"]
                gtk::Button {
                    halign: gtk::Align::Start,
                    sensitive: false,
                    clicked => Msg::RemovePasswordFromKeyring,
                },
                #[name="change_password"]
                gtk::Button {
                    label: "Change database password",
                    halign: gtk::Align::Start,
                    clicked => Msg::ChangeDbPassword,
                },
                #[style_class="section_title"]
                gtk::Label {
                    text: "Database file",
                    xalign: 0.0,
                },
                #[name="db_location_label"]
                gtk::Label {
                    xalign: 0.0,
                    ellipsize: pango::EllipsizeMode::Middle,
                }
            },
            key_press_event(_, key) => (Msg::KeyPress(key.clone()), Inhibit(false)), // just for the ESC key.. surely there's a better way..
        }
    }
}
