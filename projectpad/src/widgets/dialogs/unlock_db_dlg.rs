use super::dialog_helpers;
use crate::widgets::password_field::Msg as PasswordFieldMsg;
use crate::widgets::password_field::Msg::PublishPassword as PasswordFieldMsgPublishPassword;
use crate::widgets::password_field::PasswordField;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    OkPressed,
    GotPassword(String),
}

pub struct Model {}

#[widget]
impl Widget for UnlockDbDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
    }

    fn model(relm: &relm::Relm<Self>, is_new_db: bool) -> Model {
        Model {}
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::OkPressed => {
                self.password_entry
                    .stream()
                    .emit(PasswordFieldMsg::RequestPassword);
            }
            Msg::GotPassword(pass) => {}
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
            PasswordField("".to_string()) {
                hexpand: true,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
                PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPassword(pass.clone())
            },
            #[name="save_password_check"]
            gtk::CheckButton {
                label: "Save password to your OS keyring",
                active: false,
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                },
            },
        }
    }
}
