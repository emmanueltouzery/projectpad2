use crate::icons::Icon;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{Server, ServerAccessType, ServerType};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;

#[derive(Msg, Debug)]
pub enum Msg {
    GotGroups(Vec<String>),
    RemoveAuthFile,
    SaveAuthFile,
    AuthFilePicked,
}

pub struct Model {
    relm: relm::Relm<ServerAddEditDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    _channel: relm::Channel<Vec<String>>,
    sender: relm::Sender<Vec<String>>,
    groups_store: gtk::ListStore,
    project_id: i32,
    description: String,
    is_retired: bool,
    address: String,
    text: String,
    group_name: Option<String>,
    username: String,
    password: String,
    server_type: ServerType,
    server_access_type: ServerAccessType,
    auth_key_filename: Option<String>,
    auth_key: Option<Vec<u8>>,
}

#[widget]
impl Widget for ServerAddEditDialog {
    fn init_view(&mut self) {
        self.init_server_type();
        self.init_server_access_type();
        self.init_group();
        self.update_auth_file();
    }

    fn init_server_type(&self) {
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

    fn init_server_access_type(&self) {
        self.server_access_type
            .append(Some(&ServerAccessType::SrvAccessSsh.to_string()), "SSH");
        self.server_access_type.append(
            Some(&ServerAccessType::SrvAccessRdp.to_string()),
            "Remote Desktop (RDP)",
        );
        self.server_access_type
            .append(Some(&ServerAccessType::SrvAccessWww.to_string()), "Website");
        self.server_access_type.append(
            Some(&ServerAccessType::SrvAccessSshTunnel.to_string()),
            "SSH tunnel",
        );
        self.server_access_type
            .set_active_id(Some(&self.model.server_access_type.to_string()));
    }

    pub fn get_project_group_names(
        sql_conn: &diesel::SqliteConnection,
        project_id: i32,
    ) -> Vec<String> {
        use projectpadsql::schema::project_point_of_interest::dsl as ppoi;
        use projectpadsql::schema::server::dsl as srv;
        let mut server_group_names: Vec<Option<String>> = srv::server
            .filter(
                srv::project_id
                    .eq(project_id)
                    .and(srv::group_name.is_not_null()),
            )
            .order(srv::group_name.asc())
            .select(srv::group_name)
            .load(sql_conn)
            .unwrap();
        let mut prj_poi_group_names = ppoi::project_point_of_interest
            .filter(
                ppoi::project_id
                    .eq(project_id)
                    .and(ppoi::group_name.is_not_null()),
            )
            .order(ppoi::group_name.asc())
            .select(ppoi::group_name)
            .load(sql_conn)
            .unwrap();
        server_group_names.append(&mut prj_poi_group_names);
        let mut server_group_names_no_options: Vec<_> =
            server_group_names.into_iter().map(|n| n.unwrap()).collect();
        server_group_names_no_options.sort();
        server_group_names_no_options.dedup();
        server_group_names_no_options
    }

    fn init_group(&self) {
        let s = self.model.sender.clone();
        let pid = self.model.project_id;
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                s.send(Self::get_project_group_names(sql_conn, pid))
                    .unwrap();
            }))
            .unwrap();
        let completion = gtk::EntryCompletion::new();
        completion.set_model(Some(&self.model.groups_store));
        completion.set_text_column(0);
        self.group
            .get_child()
            .unwrap()
            .dynamic_cast::<gtk::Entry>()
            .unwrap()
            .set_completion(Some(&completion));
    }

    fn update_auth_file(&self) {
        self.auth_key_stack
            .set_visible_child_name(if self.model.auth_key_filename.is_some() {
                "file"
            } else {
                "no_file"
            });
    }

    // TODO probably could take an Option<&Server> and drop some cloning
    // I take the project_id because I may not get a server to get the
    // project_id from.
    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, i32, Option<Server>),
    ) -> Model {
        let (db_sender, project_id, server) = params;
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |groups: Vec<String>| {
            stream.emit(Msg::GotGroups(groups));
        });
        Model {
            relm: relm.clone(),
            db_sender,
            _channel: channel,
            sender,
            groups_store: gtk::ListStore::new(&[glib::Type::String]),
            project_id,
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
            group_name: server.as_ref().and_then(|s| s.group_name.clone()),
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
            server_access_type: server
                .as_ref()
                .map(|s| s.access_type)
                .unwrap_or(ServerAccessType::SrvAccessSsh),
            auth_key_filename: server.as_ref().and_then(|s| s.auth_key_filename.clone()),
            auth_key: server.as_ref().and_then(|s| s.auth_key.clone()),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::GotGroups(groups) => {
                for group in groups {
                    let iter = self.model.groups_store.append();
                    self.model
                        .groups_store
                        .set_value(&iter, 0, &glib::Value::from(&group));
                    self.group.append_text(&group);
                }

                if let Some(t) = self.model.group_name.as_deref() {
                    self.group
                        .get_child()
                        .unwrap()
                        .dynamic_cast::<gtk::Entry>()
                        .unwrap()
                        .set_text(t);
                }
            }
            Msg::RemoveAuthFile => {
                self.model.auth_key_filename = None;
                self.update_auth_file();
            }
            Msg::AuthFilePicked => {
                // doing Some(x.unwrap()) because I assume that I get a Some here
                // i want it to blow if that's not the case
                self.model.auth_key_filename = Some(
                    Path::new(&self.auth_key.get_filename().unwrap())
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                );
                self.update_auth_file();
            }
            Msg::SaveAuthFile => {
                let win = self
                    .grid
                    .get_toplevel()
                    .and_then(|w| w.dynamic_cast::<gtk::Window>().ok());
                // https://stackoverflow.com/questions/54487052/how-do-i-add-a-save-button-to-the-gtk-filechooser-dialog
                let dialog = gtk::FileChooserDialog::new(
                    None,
                    win.as_ref(),
                    gtk::FileChooserAction::SelectFolder,
                );
                let auth_key = self.model.auth_key.clone();
                let auth_key_filename = self.model.auth_key_filename.clone();
                dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                dialog.add_button("Save", gtk::ResponseType::Ok);
                dialog.connect_response(move |d, r| {
                    d.close();
                    let mut fname = None;
                    if r == gtk::ResponseType::Ok {
                        if let Some(filename) = d.get_filename() {
                            fname = Some(filename);
                        }
                    }
                    if let Some(fname) = fname {
                        if let Err(e) = Self::write_auth_key(&auth_key, &auth_key_filename, fname) {
                            Self::display_error("Error writing the file", e);
                        }
                    }
                });
                dialog.show_all();
            }
        }
    }

    fn display_error(msg: &str, e: std::io::Error) {
        let dlg = gtk::MessageDialogBuilder::new()
            .buttons(gtk::ButtonsType::Ok)
            .message_type(gtk::MessageType::Error)
            .text(msg)
            .secondary_text(&e.to_string())
            .build();
        dlg.connect_response(|d, r| d.close());
        dlg.show_all();
    }

    fn write_auth_key(
        auth_key: &Option<Vec<u8>>,
        auth_key_filename: &Option<String>,
        folder: PathBuf,
    ) -> std::io::Result<()> {
        if let (Some(data), Some(fname)) = (auth_key, auth_key_filename) {
            let mut file = File::create(folder.join(fname))?;
            file.write_all(&data)
        } else {
            Ok(())
        }
    }

    view! {
        #[name="grid"]
        gtk::Grid {
            margin_start: 30,
            margin_end: 30,
            margin_top: 10,
            margin_bottom: 5,
            row_spacing: 5,
            column_spacing: 10,
            gtk::Label {
                text: "Description",
                halign: gtk::Align::End,
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
                text: "Is retired",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 1,
                },
            },
            gtk::CheckButton {
                label: "",
                active: self.model.is_retired,
                cell: {
                    left_attach: 1,
                    top_attach: 1,
                },
            },
            gtk::Label {
                text: "Address",
                halign: gtk::Align::End,
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
                text: "Text",
                halign: gtk::Align::End,
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
                text: "Group",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 4,
                },
            },
            #[name="group"]
            gtk::ComboBoxText({has_entry: true}) {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 4,
                },
            },
            gtk::Label {
                text: "Username",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 5,
                },
            },
            #[name="username_entry"]
            gtk::Entry {
                hexpand: true,
                text: &self.model.username,
                cell: {
                    left_attach: 1,
                    top_attach: 5,
                },
            },
            gtk::Label {
                text: "Password",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 6,
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
                    top_attach: 6,
                },
            },
            gtk::Label {
                text: "Authentication key",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 7,
                },
            },
            #[name="auth_key_stack"]
            gtk::Stack {
                cell: {
                    left_attach: 1,
                    top_attach: 7,
                },
                // visible_child_name: if self.model.auth_key_filename.is_some() { "file" } else { "no_file" },
                // if there is no file, a file picker...
                #[name="auth_key"]
                gtk::FileChooserButton({action: gtk::FileChooserAction::Open}) {
                    child: {
                        name: Some("no_file")
                    },
                    hexpand: true,
                    selection_changed(_) => Msg::AuthFilePicked,
                },
                // if there is a file, a label with the filename,
                // and a button to remove the file
                gtk::Box {
                    orientation: gtk::Orientation::Horizontal,
                    child: {
                        name: Some("file")
                    },
                    gtk::Label {
                        hexpand: true,
                        text: self.model.auth_key_filename.as_deref().unwrap_or_else(|| "")
                    },
                    gtk::Button {
                        always_show_image: true,
                        image: Some(&gtk::Image::from_icon_name(
                            Some("document-save-symbolic"), gtk::IconSize::Menu)),
                        button_press_event(_, _) => (Msg::SaveAuthFile, Inhibit(false)),
                    },
                    gtk::Button {
                        always_show_image: true,
                        image: Some(&gtk::Image::from_icon_name(
                            // Some(Icon::TRASH.name()), gtk::IconSize::Menu)),
                            Some("edit-delete-symbolic"), gtk::IconSize::Menu)),
                        button_press_event(_, _) => (Msg::RemoveAuthFile, Inhibit(false)),
                    },
                },
            },
            gtk::Label {
                text: "Server type",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 8,
                },
            },
            #[name="server_type"]
            gtk::ComboBoxText {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 8,
                },
            },
            gtk::Label {
                text: "Access type",
                halign: gtk::Align::End,
                cell: {
                    left_attach: 0,
                    top_attach: 9,
                },
            },
            #[name="server_access_type"]
            gtk::ComboBoxText {
                hexpand: true,
                cell: {
                    left_attach: 1,
                    top_attach: 9,
                },
            },
        }
    }
}
