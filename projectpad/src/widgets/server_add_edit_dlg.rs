use gtk::prelude::*;
use projectpadsql::models::{Server, ServerAccessType};
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg, Debug)]
pub enum Msg {}

pub struct Model {
    relm: relm::Relm<ServerAddEditDialog>,
    description: String,
    address: String,
    text: String,
    username: String,
    password: String,
}

#[widget]
impl Widget for ServerAddEditDialog {
    fn init_view(&mut self) {}

    // TODO probably could take an Option<&Server> and drop some cloning
    fn model(relm: &relm::Relm<Self>, server: Option<Server>) -> Model {
        Model {
            relm: relm.clone(),
            description: server
                .as_ref()
                .map(|s| s.desc.clone())
                .unwrap_or_else(|| "".to_string()),
            address: server
                .as_ref()
                .map(|s| s.ip.clone())
                .unwrap_or_else(|| "".to_string()),
            text: server
                .as_ref()
                .map(|s| s.text.clone())
                .unwrap_or_else(|| "".to_string()),
            username: server
                .as_ref()
                .map(|s| s.username.clone())
                .unwrap_or_else(|| "".to_string()),
            password: server
                .as_ref()
                .map(|s| s.password.clone())
                .unwrap_or_else(|| "".to_string()),
        }
    }

    fn update(&mut self, msg: Msg) {}

    view! {
        gtk::Grid {
            margin_start: 30,
            margin_end: 30,
            margin_top: 10,
            margin_bottom: 5,
            row_spacing: 5,
            column_spacing: 10,
            gtk::Label {
                text: "Description:",
                cell: {
                    left_attach: 0,
                    top_attach: 0,
                },
            },
            #[name="desc_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.description,
                cell: {
                    left_attach: 1,
                    top_attach: 0,
                },
            },
            gtk::Label {
                text: "Address:",
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
            },
            #[name="address_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.address,
                cell: {
                    left_attach: 1,
                    top_attach: 1,
                },
            },
            gtk::Label {
                text: "Text:",
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                },
            },
            #[name="text_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.text,
                cell: {
                    left_attach: 1,
                    top_attach: 2,
                },
            },
            gtk::Label {
                text: "Username:",
                cell: {
                    left_attach: 0,
                    top_attach: 3,
                },
            },
            #[name="username_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.username,
                cell: {
                    left_attach: 1,
                    top_attach: 3,
                },
            },
            gtk::Label {
                text: "Password:",
                cell: {
                    left_attach: 0,
                    top_attach: 4,
                },
            },
            #[name="password_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.password,
                visibility: false,
                input_purpose: gtk::InputPurpose::Password,
                cell: {
                    left_attach: 1,
                    top_attach: 4,
                },
            },
        }
    }
}
