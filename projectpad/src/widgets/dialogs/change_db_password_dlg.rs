use super::dialog_helpers;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::password_field;
use crate::widgets::password_field::Msg as PasswordFieldMsg;
use crate::widgets::password_field::Msg::PasswordChanged as PasswordFieldMsgPasswordChanged;
use crate::widgets::password_field::Msg::PublishPassword as PasswordFieldMsgPublishPassword;
use crate::widgets::password_field::PasswordField;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

type OpResult = Result<(), String>;

#[derive(Msg)]
pub enum Msg {
    OkPressed,
    HideInfobar,
    GotApplyButton(gtk::Button),
    CurrentPasswordChange(String),
    CurrentPasswordValid(bool),
    GotCurrentPassword(String),
    GotNewPassword(String),
    GotConfirmNewPassword(String),
    CheckedOldPassword(bool),
    ChangedPass(OpResult),
    SuccessfullyChangedPass,
}

pub struct Model {
    relm: relm::Relm<ChangeDbPasswordDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    _current_pass_valid_channel: relm::Channel<OpResult>,
    current_pass_valid_sender: relm::Sender<OpResult>,
    _pass_valid_channel: relm::Channel<OpResult>,
    pass_valid_sender: relm::Sender<OpResult>,
    _changed_pass_channel: relm::Channel<OpResult>,
    changed_pass_sender: relm::Sender<OpResult>,
    apply_button: Option<gtk::Button>,
    new_password: Option<String>,
    infobar: gtk::InfoBar,
    infobar_label: gtk::Label,
}

pub fn check_db_password(pass: &str) -> OpResult {
    let db_conn =
        SqliteConnection::establish(&projectpadsql::database_path().to_string_lossy()).unwrap();
    projectpadsql::try_unlock_db(&db_conn, pass)
}

fn set_db_password(db_conn: &SqliteConnection, pass: &str) -> Result<(), String> {
    db_conn
        .execute(&format!(
            "PRAGMA rekey='{}';",
            projectpadsql::key_escape_param_value(pass)
        ))
        .map(|_| ())
        .map_err(|x| x.to_string())
}

#[widget]
impl Widget for ChangeDbPasswordDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
        self.grid.set_margin_bottom(20);
        self.current_pwd_header
            .get_style_context()
            .add_class("section_title");
        self.new_pwd_header
            .get_style_context()
            .add_class("section_title");
        self.init_infobar_overlay();
    }

    fn init_infobar_overlay(&self) {
        self.infobar_overlay.add_overlay(&self.model.infobar);
        self.infobar_overlay
            .set_overlay_pass_through(&self.model.infobar, true);
    }

    fn show_error(&self, msg: &str) {
        self.model.infobar_label.set_text(&msg);
        self.model.infobar.set_revealed(true);
        relm::timeout(self.model.relm.stream(), 1500, || Msg::HideInfobar);
    }

    fn clear_error(&self) {
        self.model.infobar.set_revealed(false);
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (pass_valid_channel, pass_valid_sender) =
            relm::Channel::new(move |r: OpResult| stream.emit(Msg::CheckedOldPassword(r.is_ok())));
        let stream2 = relm.stream().clone();
        let (changed_pass_channel, changed_pass_sender) =
            relm::Channel::new(move |r: OpResult| stream2.emit(Msg::ChangedPass(r)));
        let stream3 = relm.stream().clone();
        let (current_pass_valid_channel, current_pass_valid_sender) =
            relm::Channel::new(move |r: OpResult| {
                stream3.emit(Msg::CurrentPasswordValid(r.is_ok()))
            });
        let infobar = gtk::InfoBarBuilder::new()
            .revealed(false)
            .message_type(gtk::MessageType::Info)
            .valign(gtk::Align::Start)
            .build();

        let infobar_label = gtk::LabelBuilder::new().label("").build();
        infobar_label.show();
        infobar.get_content_area().add(&infobar_label);
        infobar.show();
        Model {
            relm: relm.clone(),
            db_sender,
            current_pass_valid_sender,
            _current_pass_valid_channel: current_pass_valid_channel,
            pass_valid_sender,
            _pass_valid_channel: pass_valid_channel,
            changed_pass_sender,
            _changed_pass_channel: changed_pass_channel,
            new_password: None,
            infobar,
            infobar_label,
            apply_button: None,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::OkPressed => {
                self.current_password_entry
                    .stream()
                    .emit(PasswordFieldMsg::RequestPassword);
            }
            Msg::HideInfobar => self.clear_error(),
            Msg::CurrentPasswordChange(pass) => {
                let s = self.model.current_pass_valid_sender.clone();
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |_| {
                        s.send(check_db_password(&pass)).unwrap();
                    }))
                    .unwrap();
            }
            Msg::GotCurrentPassword(pass) => {
                let s = self.model.pass_valid_sender.clone();
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |_| {
                        s.send(check_db_password(&pass)).unwrap();
                    }))
                    .unwrap();
            }
            Msg::CheckedOldPassword(false) => {
                self.show_error("Wrong current database password");
            }
            Msg::GotApplyButton(btn) => {
                btn.set_label("Apply");
                btn.set_sensitive(false);
                self.model.apply_button = Some(btn);
            }
            Msg::CurrentPasswordValid(valid) => {
                if let Some(btn) = &self.model.apply_button {
                    btn.set_sensitive(valid);
                }
            }
            Msg::CheckedOldPassword(true) => {
                self.clear_error();
                self.new_password_entry
                    .stream()
                    .emit(PasswordFieldMsg::RequestPassword);
            }
            Msg::GotNewPassword(pass) => {
                self.model.new_password = Some(pass);
                self.confirm_password_entry
                    .stream()
                    .emit(PasswordFieldMsg::RequestPassword);
            }
            Msg::GotConfirmNewPassword(pass) => {
                if Some(&pass) != self.model.new_password.as_ref() {
                    self.show_error("New and confirm new passwords don't match");
                } else {
                    if let Some(btn) = &self.model.apply_button {
                        btn.set_sensitive(false);
                    }
                    self.clear_error();
                    let s = self.model.changed_pass_sender.clone();
                    self.model
                        .db_sender
                        .send(SqlFunc::new(move |db_conn| {
                            let r = set_db_password(db_conn, &pass);
                            let r1 =
                                if r.is_ok() && projectpadsql::get_pass_from_keyring().is_some() {
                                    projectpadsql::set_pass_in_keyring(&pass)
                                } else {
                                    r
                                };
                            s.send(r1).unwrap();
                        }))
                        .unwrap();
                }
            }
            Msg::ChangedPass(Err(msg)) => {
                if let Some(btn) = &self.model.apply_button {
                    btn.set_sensitive(true);
                }
                standard_dialogs::display_error_str(
                    "Error changing the database password",
                    Some(msg),
                );
            }
            Msg::ChangedPass(Ok(_)) => {
                self.model.relm.stream().emit(Msg::SuccessfullyChangedPass);
            }
            Msg::SuccessfullyChangedPass => {}
        }
    }

    view! {
        #[name="infobar_overlay"]
        gtk::Overlay {
            #[name="grid"]
            gtk::Grid {
                #[name="current_pwd_header"]
                gtk::Label {
                    text: "Current database password",
                    halign: gtk::Align::Start,
                    margin_top: 10,
                    cell: {
                        left_attach: 0,
                        top_attach: 1,
                        width: 2,
                    },
                },
                gtk::Label {
                    text: "Current Password",
                    halign: gtk::Align::End,
                    margin_top: 20,
                    cell: {
                        left_attach: 0,
                        top_attach: 2,
                    },
                },
                #[name="current_password_entry"]
                PasswordField(("".to_string(), password_field::ActivatesDefault::Yes)) {
                    hexpand: true,
                    margin_top: 15,
                    cell: {
                        left_attach: 1,
                        top_attach: 2,
                    },
                    PasswordFieldMsgPublishPassword(ref pass) => Msg::GotCurrentPassword(pass.clone()),
                    PasswordFieldMsgPasswordChanged(ref pass) => Msg::CurrentPasswordChange(pass.clone()),
                },
                #[name="new_pwd_header"]
                gtk::Label {
                    text: "New database password",
                    halign: gtk::Align::Start,
                    margin_top: 10,
                    cell: {
                        left_attach: 0,
                        top_attach: 3,
                        width: 2,
                    },
                },
                gtk::Label {
                    text: "New Password",
                    halign: gtk::Align::End,
                    margin_top: 20,
                    cell: {
                        left_attach: 0,
                        top_attach: 4,
                    },
                },
                #[name="new_password_entry"]
                PasswordField(("".to_string(), password_field::ActivatesDefault::Yes)) {
                    hexpand: true,
                    margin_top: 15,
                    cell: {
                        left_attach: 1,
                        top_attach: 4,
                    },
                    PasswordFieldMsgPublishPassword(ref pass) => Msg::GotNewPassword(pass.clone()),
                },
                gtk::Label {
                    text: "Confirm new password",
                    halign: gtk::Align::End,
                    margin_top: 10,
                    cell: {
                        left_attach: 0,
                        top_attach: 5,
                    },
                },
                #[name="confirm_password_entry"]
                PasswordField(("".to_string(), password_field::ActivatesDefault::Yes)) {
                    hexpand: true,
                    cell: {
                        left_attach: 1,
                        top_attach: 5,
                    },
                    PasswordFieldMsgPublishPassword(ref pass) => Msg::GotConfirmNewPassword(pass.clone()),
                },
            }
        }
    }
}
