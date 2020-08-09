use gtk::prelude::*;
use projectpadsql::models::{Server, ServerAccessType, ServerType};
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg, Debug)]
pub enum Msg {}

pub struct Model {
    relm: relm::Relm<ServerAddEditDialog>,
    description: String,
    is_retired: bool,
    address: String,
    text: String,
    username: String,
    password: String,
    server_type: ServerType,
}

#[widget]
impl Widget for ServerAddEditDialog {
    fn init_view(&mut self) {
        self.server_type
            .append(Some(&ServerType::SrvApplication.to_string()), "Application");
        self.server_type
            .append(Some(&ServerType::SrvDatabase.to_string()), "Database");
        self.server_type.append(
            Some(&ServerType::SrvHttpOrProxy.to_string()),
            "HTTP server or proxy",
        );
        self.server_type
            .append(Some(&ServerType::SrvMonitoring.to_string()), "Monitoring");
        self.server_type
            .append(Some(&ServerType::SrvReporting.to_string()), "Reporting");
        self.server_type
            .set_active_id(Some(&self.model.server_type.to_string()));
    }

    // TODO probably could take an Option<&Server> and drop some cloning
    fn model(relm: &relm::Relm<Self>, server: Option<Server>) -> Model {
        Model {
            relm: relm.clone(),
            description: server
                .as_ref()
                .map(|s| s.desc.clone())
                .unwrap_or_else(|| "".to_string()),
            is_retired: server.as_ref().map(|s| s.is_retired).unwrap_or(false),
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
            server_type: server
                .as_ref()
                .map(|s| s.server_type)
                .unwrap_or(ServerType::SrvApplication),
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
            gtk::CheckButton {
                label: "Is retired",
                active: self.model.is_retired,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                    width: 2,
                },
            },
            gtk::Label {
                text: "Address:",
                cell: {
                    left_attach: 0,
                    top_attach: 2,
                },
            },
            #[name="address_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.address,
                cell: {
                    left_attach: 1,
                    top_attach: 2,
                },
            },
            gtk::Label {
                text: "Text:",
                cell: {
                    left_attach: 0,
                    top_attach: 3,
                },
            },
            #[name="text_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.text,
                cell: {
                    left_attach: 1,
                    top_attach: 3,
                },
            },
            gtk::Label {
                text: "Username:",
                cell: {
                    left_attach: 0,
                    top_attach: 4,
                },
            },
            #[name="username_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.username,
                cell: {
                    left_attach: 1,
                    top_attach: 4,
                },
            },
            gtk::Label {
                text: "Password:",
                cell: {
                    left_attach: 0,
                    top_attach: 5,
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
                    top_attach: 5,
                },
            },
            gtk::Label {
                text: "Server type:",
                cell: {
                    left_attach: 0,
                    top_attach: 6,
                },
            },
            #[name="server_type"]
            gtk::ComboBoxText {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 6,
                },
            },
        }
    }
}
