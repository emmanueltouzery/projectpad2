use super::super::keyring_helpers;
use super::dialog_helpers;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::password_field;
use crate::widgets::password_field::Msg as PasswordFieldMsg;
use crate::widgets::password_field::Msg::PublishPassword as PasswordFieldMsgPublishPassword;
use crate::widgets::password_field::PasswordField;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

type CheckPassResult = Result<(), String>;

#[derive(Msg)]
pub enum Msg {
    OkPressed,
    GotPassword(String),
    GotConfirmPassword(String),
    CheckedPassword(CheckPassResult),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    is_new_db: bool,
    error_label: gtk::Label,
    _pass_valid_channel: relm::Channel<CheckPassResult>,
    pass_valid_sender: relm::Sender<CheckPassResult>,
    password: Option<String>,
}

#[widget]
impl Widget for UnlockDbDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.widgets.grid);

        self.model.error_label.show();
        self.widgets
            .error_infobar
            .content_area()
            .add(&self.model.error_label);
    }

    fn model(relm: &relm::Relm<Self>, params: (bool, mpsc::Sender<SqlFunc>)) -> Model {
        let (is_new_db, db_sender) = params;
        let stream = relm.stream().clone();
        let (pass_valid_channel, pass_valid_sender) =
            relm::Channel::new(move |r| stream.emit(Msg::CheckedPassword(r)));
        Model {
            db_sender,
            is_new_db,
            error_label: gtk::Label::builder().label("").build(),
            pass_valid_sender,
            _pass_valid_channel: pass_valid_channel,
            password: None,
        }
    }

    fn show_error(&self, msg: &str) {
        self.model.error_label.set_text(msg);
        self.widgets.error_infobar.set_visible(true);
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::OkPressed => {
                self.streams
                    .password_entry
                    .emit(PasswordFieldMsg::RequestPassword);
            }
            Msg::GotPassword(pass) => {
                if pass.is_empty() {
                    // prevent empty passwords in projectpad2, because moving between
                    // plaintext and encrypted sqlcipher databases is non-trivial
                    // https://www.zetetic.net/sqlcipher/sqlcipher-api/index.html#sqlcipher_export
                    self.show_error("The password must not be empty");
                } else {
                    self.model.password = Some(pass);
                    self.streams
                        .password_confirm_entry
                        .emit(PasswordFieldMsg::RequestPassword);
                }
            }
            Msg::GotConfirmPassword(pass) => {
                if self.model.is_new_db && Some(&pass) != self.model.password.as_ref() {
                    self.show_error("Passwords don't match");
                } else {
                    let s = self.model.pass_valid_sender.clone();
                    let is_save_to_keyring = self.widgets.save_password_check.is_active();
                    let p = self.model.password.as_ref().unwrap().clone();
                    self.model
                        .db_sender
                        .send(SqlFunc::new(move |db_conn| {
                            let r = projectpadsql::try_unlock_db(db_conn, &p);
                            if r.is_ok() && is_save_to_keyring {
                                if let Err(msg) = keyring_helpers::set_pass_in_keyring(&p) {
                                    standard_dialogs::display_error_str(
                                        "Error saving the password to the keyring",
                                        Some(msg),
                                    );
                                }
                            }
                            s.send(r).unwrap();
                        }))
                        .unwrap();
                }
            }
            Msg::CheckedPassword(Err(msg)) => {
                standard_dialogs::display_error_str("Error checking the password", Some(msg));
            }
            Msg::CheckedPassword(Ok(_)) => {}
        }
    }

    view! {
        #[name="grid"]
        gtk::Grid {
            #[name="error_infobar"]
            gtk::InfoBar {
                message_type: gtk::MessageType::Error,
                cell: {
                    left_attach: 0,
                    top_attach: 0,
                    width: 2,
                },
                visible: false,
            },
            gtk::Label {
                text: "Please enter the database password",
                halign: gtk::Align::Start,
                visible: !self.model.is_new_db,
                margin_top: 10,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                    width: 2,
                },
            },
            gtk::Label {
                text: "Projectpad needs a password to encrypt your database, please enter one to continue.",
                halign: gtk::Align::Start,
                visible: self.model.is_new_db,
                margin_top: 10,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                    width: 2,
                },
            },
            gtk::Label {
                text: "Password",
                halign: gtk::Align::End,
                visible: self.model.is_new_db,
                margin_top: 20,
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                },
            },
            #[name="password_entry"]
            PasswordField(("".to_string(), password_field::ActivatesDefault::Yes)) {
                hexpand: true,
                margin_top: 15,
                cell: {
                    left_attach: 1,
                    top_attach: 2,
                },
                PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPassword(pass.clone()),
            },
            gtk::Label {
                text: "Confirm password",
                halign: gtk::Align::End,
                visible: self.model.is_new_db,
                margin_top: 10,
                cell: {
                    left_attach: 0,
                    top_attach: 3,
                },
            },
            #[name="password_confirm_entry"]
            PasswordField(("".to_string(), password_field::ActivatesDefault::Yes)) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 3,
                },
                visible: self.model.is_new_db,
                PasswordFieldMsgPublishPassword(ref pass) => Msg::GotConfirmPassword(pass.clone()),
            },
            #[name="save_password_check"]
            gtk::CheckButton {
                label: "Save password to the OS keyring",
                active: false,
                margin_top: 10,
                cell: {
                    left_attach: 0,
                    top_attach: 4,
                    width: 2,
                },
            },
        }
    }
}
