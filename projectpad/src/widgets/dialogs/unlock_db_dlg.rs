use super::dialog_helpers;
use super::standard_dialogs;
use crate::sql_thread::SqlFunc;
use crate::widgets::password_field;
use crate::widgets::password_field::Msg as PasswordFieldMsg;
use crate::widgets::password_field::Msg::PublishPassword as PasswordFieldMsgPublishPassword;
use crate::widgets::password_field::PasswordField;
use diesel::prelude::*;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

/// TODO quotes in passwords
pub fn try_unlock_db(db_conn: &SqliteConnection, pass: &str) -> Result<(), String> {
    // https://www.zetetic.net/sqlcipher/sqlcipher-api/#PRAGMA_key
    db_conn
        .execute(&format!(
            "PRAGMA key='{}'; SELECT count(*) FROM sqlite_master;",
            pass
        ))
        .map(|_| ())
        .map_err(|x| x.to_string())
}

type CheckPassResult = Result<(), String>;

#[derive(Msg)]
pub enum Msg {
    OkPressed,
    GotPassword(String),
    CheckedPassword(CheckPassResult),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    _pass_valid_channel: relm::Channel<CheckPassResult>,
    pass_valid_sender: relm::Sender<CheckPassResult>,
}

#[widget]
impl Widget for UnlockDbDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
    }

    fn model(relm: &relm::Relm<Self>, params: (bool, mpsc::Sender<SqlFunc>)) -> Model {
        let (is_new_db, db_sender) = params;
        let stream = relm.stream().clone();
        let (pass_valid_channel, pass_valid_sender) =
            relm::Channel::new(move |r| stream.emit(Msg::CheckedPassword(r)));
        Model {
            db_sender,
            pass_valid_sender,
            _pass_valid_channel: pass_valid_channel,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::OkPressed => {
                self.password_entry
                    .stream()
                    .emit(PasswordFieldMsg::RequestPassword);
            }
            Msg::GotPassword(pass) => {
                let s = self.model.pass_valid_sender.clone();
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |db_conn| {
                        s.send(try_unlock_db(db_conn, &pass)).unwrap();
                    }))
                    .unwrap();
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
            gtk::Label {
                text: "Please enter the database password",
                halign: gtk::Align::Start,
                margin_top: 10,
                cell: {
                    left_attach: 0,
                    top_attach: 0,
                },
            },
            #[name="password_entry"]
            PasswordField(("".to_string(), password_field::ActivatesDefault::Yes)) {
                hexpand: true,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
                PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPassword(pass.clone()),
            },
            #[name="save_password_check"]
            gtk::CheckButton {
                label: "Save password to the OS keyring",
                active: false,
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                },
            },
        }
    }
}
