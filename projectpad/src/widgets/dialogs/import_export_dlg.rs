use super::dialog_helpers;
use crate::import;
use crate::sql_thread::SqlFunc;
use crate::widgets::password_field;
use crate::widgets::password_field::Msg::PublishPassword as PasswordFieldMsgPublishPassword;
use crate::widgets::password_field::PasswordField;
use diesel::connection::Connection;
use gtk::prelude::*;
use relm::{Component, Widget};
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum HeaderMsg {
    CancelAction,
    NextAction,
    EnableNext(bool),
}

#[widget]
impl Widget for Header {
    fn init_view(&mut self) {
        self.next_btn
            .get_style_context()
            .add_class("suggested-action");
    }

    fn model() {}

    fn update(&mut self, event: HeaderMsg) {
        match event {
            HeaderMsg::CancelAction => {}
            HeaderMsg::NextAction => {}
            HeaderMsg::EnableNext(enable) => self.next_btn.set_sensitive(enable),
        }
    }

    view! {
        gtk::HeaderBar {
            title: Some("Import Export"),
            gtk::Button {
                label: "Cancel",
                clicked => HeaderMsg::CancelAction,
            },
            #[name="next_btn"]
            gtk::Button {
                label: "Next",
                child: {
                    pack_type: gtk::PackType::End
                },
                clicked => HeaderMsg::NextAction,
            }
        }
    }
}

#[derive(Msg)]
pub enum Msg {
    Close,
    NextClicked,
    GotPassword(String),
    ImportFileSet,
}

pub enum WizardState {
    Start,
    ImportPickFile,
    ExportPickFile,
}

pub struct Model {
    relm: relm::Relm<ImportExportDialog>,
    header: Component<Header>,
    db_sender: mpsc::Sender<SqlFunc>,
    wizard_state: WizardState,
}

const CHILD_NAME_IMPORT: &str = "import";

#[widget]
impl Widget for ImportExportDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.grid);
        self.grid.set_margin_top(20);
        self.export_file_radio
            .join_group(Some(&self.import_file_radio));
        let h = &self.model.header;
        relm::connect!(h@HeaderMsg::NextAction, self.model.relm, Msg::NextClicked);
        relm::connect!(h@HeaderMsg::CancelAction, self.model.relm, Msg::Close);
        let filter = gtk::FileFilter::new();
        filter.add_pattern("*.7z");
        self.import_picker_btn.set_filter(&filter);
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let header = relm::init(()).expect("header");
        Model {
            relm: relm.clone(),
            db_sender,
            header,
            wizard_state: WizardState::Start,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Close => self.import_win.close(),
            Msg::NextClicked => match self.model.wizard_state {
                WizardState::Start => {
                    self.model
                        .header
                        .stream()
                        .emit(HeaderMsg::EnableNext(false));
                    self.wizard_stack.set_visible_child_name(CHILD_NAME_IMPORT);
                    self.model.wizard_state = WizardState::ImportPickFile;
                }
                WizardState::ImportPickFile => {
                    self.password_entry
                        .stream()
                        .emit(password_field::Msg::RequestPassword);
                }
                WizardState::ExportPickFile => {}
            },
            Msg::ImportFileSet => {
                self.model.header.stream().emit(HeaderMsg::EnableNext(true));
            }
            Msg::GotPassword(pass) => match self.model.wizard_state {
                WizardState::ImportPickFile => {
                    self.do_import(pass);
                }
                _ => panic!(),
            },
        }
    }

    fn do_import(&self, pass: String) {
        match self.import_picker_btn.get_filename() {
            None => eprintln!("Please pick a file to import"), // shouldn't happen, but i don't want to crash
            Some(fname) => {
                self.model.db_sender.send(SqlFunc::new(move |sql_conn| {
                    if let Err(e) = sql_conn.transaction(|| {
                        import::do_import(sql_conn, &fname.to_string_lossy(), &pass)
                    }) {
                        eprintln!("import failed: {:?}", e);
                    }
                }));
            }
        }
    }

    view! {
        #[name="import_win"]
        gtk::Window {
            titlebar: Some(self.model.header.widget()),
            property_default_width: 600,
            property_default_height: 200,
            #[name="wizard_stack"]
            gtk::Stack {
                gtk::Box {
                    margin_top: 15,
                    margin_start: 15,
                    margin_end: 15,
                    margin_bottom: 15,
                    spacing: 10,
                    orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        text: "You can export any project to a single data file. The file can then be \
                               shared. The exported file is an encrypted 7zip file which can be either \
                               imported back in another projectpad instance, or used directly by the \
                               recipient as a textual description of the exported project. The \
                               7zip contains a human-readable YAML file.",
                        line_wrap: true,
                    },
                    #[name="import_file_radio"]
                    gtk::RadioButton {
                        label: "Import file",
                    },
                    #[name="export_file_radio"]
                    gtk::RadioButton {
                        label: "Export file",
                    },
                },
                #[name="grid"]
                gtk::Grid {
                    child: {
                        name: Some(CHILD_NAME_IMPORT)
                    },
                    gtk::Label {
                        text: "Pick a .7z projectpad file to import",
                        halign: gtk::Align::End,
                        cell: {
                            left_attach: 0,
                            top_attach: 1,
                        }
                    },
                    #[name="import_picker_btn"]
                    gtk::FileChooserButton {
                        title: "Pick a .7z projectpad file to import",
                        hexpand: true,
                        cell: {
                            left_attach: 1,
                            top_attach: 1,
                        },
                        file_set => Msg::ImportFileSet,
                    },
                    gtk::Label {
                        text: "Password",
                        halign: gtk::Align::End,
                        margin_top: 20,
                        cell: {
                            left_attach: 0,
                            top_attach: 2,
                        },
                    },
                    #[name="password_entry"]
                    PasswordField(("".to_string(), password_field::ActivatesDefault::Yes)) {
                        hexpand: true,
                        cell: {
                            left_attach: 1,
                            top_attach: 2,
                        },
                        PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPassword(pass.clone()),
                    },
                }
            }
        }
    }
}
