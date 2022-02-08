use super::dialog_helpers;
use super::standard_dialogs;
use crate::export;
use crate::import;
use crate::sql_thread::SqlFunc;
use crate::widgets::password_field;
use crate::widgets::password_field::Msg::PublishPassword as PasswordFieldMsgPublishPassword;
use crate::widgets::password_field::PasswordField;
use diesel::connection::Connection;
use diesel::prelude::*;
use gtk::prelude::*;
use itertools::Itertools;
use projectpadsql::models::Project;
use relm::{Component, Widget};
use relm_derive::{widget, Msg};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path;
use std::sync::mpsc;

#[derive(Msg)]
pub enum HeaderMsg {
    CancelAction,
    NextAction,
    EnableNext(bool),
}

#[widget]
impl Widget for Header {
    fn init_view(&mut self) {}

    fn model() {}

    fn update(&mut self, event: HeaderMsg) {
        match event {
            HeaderMsg::CancelAction => {}
            HeaderMsg::NextAction => {}
            HeaderMsg::EnableNext(enable) => self.widgets.next_btn.set_sensitive(enable),
        }
    }

    view! {
        gtk::HeaderBar {
            title: Some("Import / Export"),
            gtk::Button {
                label: "Cancel",
                clicked => HeaderMsg::CancelAction,
            },
            #[name="next_btn"]
            #[style_class="suggested-action"]
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
    KeyPress(gdk::EventKey),
    Close,
    NextClicked,
    GotPassword(String),
    GotPasswordConfirm(String),
    FilePicked,
    ImportResult(Result<(), String>),
    ExportResult(Result<HashSet<String>, String>),
    ImportApplied,
    GotProjectList(Result<Vec<Project>, String>),
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
    import_error_label: gtk::Label,
    export_error_label: gtk::Label,
    displayed_projects: Vec<Project>,
    _import_result_channel: relm::Channel<Result<(), String>>,
    import_result_sender: relm::Sender<Result<(), String>>,
    _export_result_channel: relm::Channel<Result<HashSet<String>, String>>,
    export_result_sender: relm::Sender<Result<HashSet<String>, String>>,
    _projectlist_channel: relm::Channel<Result<Vec<Project>, String>>,
    projectlist_sender: relm::Sender<Result<Vec<Project>, String>>,
    export_pass: Option<String>,
}

const CHILD_NAME_IMPORT: &str = "import";
const CHILD_NAME_EXPORT: &str = "export";

#[widget]
impl Widget for ImportExportDialog {
    fn init_view(&mut self) {
        dialog_helpers::style_grid(&self.widgets.grid);
        self.widgets.grid.set_margin_top(20);
        dialog_helpers::style_grid(&self.widgets.grid_export);
        self.widgets.grid_export.set_margin_top(20);
        self.widgets
            .export_file_radio
            .join_group(Some(&self.widgets.import_file_radio));
        let h = &self.model.header;
        relm::connect!(h@HeaderMsg::NextAction, self.model.relm, Msg::NextClicked);
        relm::connect!(h@HeaderMsg::CancelAction, self.model.relm, Msg::Close);
        let filter = gtk::FileFilter::new();
        filter.add_pattern("*.7z");
        self.widgets.import_picker_btn.set_filter(&filter);

        self.model.import_error_label.show();
        self.widgets
            .import_error_infobar
            .content_area()
            .add(&self.model.import_error_label);

        self.model.export_error_label.show();
        self.widgets
            .export_error_infobar
            .content_area()
            .add(&self.model.export_error_label);
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let header = relm::init(()).expect("header");
        let stream = relm.stream().clone();
        let (_import_result_channel, import_result_sender) =
            relm::Channel::new(move |r| stream.emit(Msg::ImportResult(r)));
        let stream2 = relm.stream().clone();
        let (_projectlist_channel, projectlist_sender) =
            relm::Channel::new(move |r| stream2.emit(Msg::GotProjectList(r)));
        let stream3 = relm.stream().clone();
        let (_export_result_channel, export_result_sender) =
            relm::Channel::new(move |r| stream3.emit(Msg::ExportResult(r)));
        Model {
            relm: relm.clone(),
            db_sender,
            header,
            wizard_state: WizardState::Start,
            import_error_label: gtk::builders::LabelBuilder::new()
                .label("")
                .ellipsize(pango::EllipsizeMode::End)
                .build(),
            export_error_label: gtk::builders::LabelBuilder::new()
                .label("")
                .ellipsize(pango::EllipsizeMode::End)
                .build(),
            displayed_projects: vec![],
            import_result_sender,
            _import_result_channel,
            export_result_sender,
            _export_result_channel,
            projectlist_sender,
            _projectlist_channel,
            export_pass: None,
        }
    }

    fn show_import_error(&self, msg: &str) {
        self.model.import_error_label.set_text(msg);
        self.model.import_error_label.set_tooltip_text(Some(msg));
        self.widgets.import_error_infobar.set_visible(true);
    }

    fn show_export_error(&self, msg: &str) {
        self.model.export_error_label.set_text(msg);
        self.model.export_error_label.set_tooltip_text(Some(msg));
        self.widgets.export_error_infobar.set_visible(true);
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::KeyPress(key) => {
                if key.keyval() == gdk::keys::constants::Escape {
                    self.widgets.import_win.close();
                }
            }
            Msg::Close => self.widgets.import_win.close(),
            Msg::NextClicked => match self.model.wizard_state {
                WizardState::Start => {
                    if self.widgets.import_file_radio.is_active() {
                        self.model
                            .header
                            .stream()
                            .emit(HeaderMsg::EnableNext(false));
                        self.widgets
                            .wizard_stack
                            .set_visible_child_name(CHILD_NAME_IMPORT);
                        self.model.wizard_state = WizardState::ImportPickFile;
                    } else {
                        self.fetch_project_list();
                        self.widgets
                            .wizard_stack
                            .set_visible_child_name(CHILD_NAME_EXPORT);
                        self.model.wizard_state = WizardState::ExportPickFile;
                    }
                }
                WizardState::ImportPickFile => {
                    self.streams
                        .import_password_entry
                        .emit(password_field::Msg::RequestPassword);
                }
                WizardState::ExportPickFile => {
                    self.streams
                        .export_password_entry
                        .emit(password_field::Msg::RequestPassword);
                }
            },
            Msg::FilePicked => {
                self.model.header.stream().emit(HeaderMsg::EnableNext(true));
            }
            Msg::GotPassword(pass) => match self.model.wizard_state {
                WizardState::ImportPickFile => {
                    self.do_import(pass);
                }
                WizardState::ExportPickFile => {
                    self.model.export_pass = Some(pass);
                    self.streams
                        .export_password_confirm_entry
                        .emit(password_field::Msg::RequestPassword);
                }
                _ => unreachable!(),
            },
            Msg::GotPasswordConfirm(pass) => {
                if Some(pass.as_str()) != self.model.export_pass.as_deref() {
                    self.show_export_error("New and confirm passwords don't match");
                    return;
                }
                match self.model.wizard_state {
                    WizardState::ExportPickFile => {
                        self.open_export_save_dialog(pass);
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }
            Msg::ImportResult(Result::Ok(())) => {
                self.widgets.import_win.close();
                self.model.relm.stream().emit(Msg::ImportApplied);
            }
            Msg::ImportResult(Result::Err(e)) => {
                self.show_import_error(&format!("Import failed: {}", e));
                self.model.header.stream().emit(HeaderMsg::EnableNext(true));
            }
            Msg::ExportResult(Result::Ok(missing_dep_project_names)) => {
                if missing_dep_project_names.is_empty() {
                    self.widgets.import_win.close();
                } else {
                    self.model.header.stream().emit(HeaderMsg::EnableNext(true));
                    standard_dialogs::display_error_str(
                        "Warning: some dependent projects were not exported",
                        Some(format!(
                            "Projects: {}",
                            missing_dep_project_names.iter().join(", ")
                        )),
                    );
                }
            }
            Msg::ExportResult(Result::Err(e)) => {
                self.show_export_error(&format!("Export failed: {}", e));
                self.model.header.stream().emit(HeaderMsg::EnableNext(true));
            }
            Msg::ImportApplied => {}
            Msg::GotProjectList(Err(e)) => self.show_export_error(&e),
            Msg::GotProjectList(Ok(project_names)) => self.populate_project_list(project_names),
        }
    }

    fn open_export_save_dialog(&self, pass: String) {
        let dialog = gtk::builders::FileChooserNativeBuilder::new()
            .action(gtk::FileChooserAction::Save)
            .title("Export to...")
            .modal(true)
            .build();
        let filter = gtk::FileFilter::new();
        filter.add_pattern("*.7z");
        dialog.set_filter(&filter);
        if dialog.run() == gtk::ResponseType::Accept {
            match dialog.filename() {
                Some(fname) if fname.extension() == Some(OsStr::new("7z")) => {
                    self.do_export(fname, pass)
                }
                // need to make sure the user picks a filename ending in .7z, or we get
                // a subtle issue in the flatpak: when you enter filename /a/b/c in the
                // file picker, flatpak gives us access to /a/b/c and NOTHING ELSE.
                // attempting to write to /a/b/c.7z will fail, and we do want to have
                // the extension...
                _ => {
                    standard_dialogs::display_error("Please pick a file name ending with .7z", None)
                }
            }
        }
    }

    fn fetch_project_list(&self) {
        let sender = self.model.projectlist_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                sender
                    .send(
                        prj::project
                            .order(prj::name.asc())
                            .load(sql_conn)
                            .map_err(|e| format!("Error loading projects: {:?}", e)),
                    )
                    .unwrap();
            }))
            .unwrap();
    }

    fn populate_project_list(&mut self, projects: Vec<Project>) {
        self.model.displayed_projects = projects;
        for child in self.widgets.project_list.children() {
            self.widgets.project_list.remove(&child);
        }
        for project in &self.model.displayed_projects {
            self.widgets.project_list.add(
                &gtk::builders::LabelBuilder::new()
                    .label(&project.name)
                    .xalign(0.0)
                    .margin(5)
                    .build(),
            );
        }
        self.widgets
            .project_list
            .select_row(self.widgets.project_list.row_at_index(0).as_ref());
        self.widgets.project_list.show_all();
    }

    fn do_import(&self, pass: String) {
        self.model
            .header
            .stream()
            .emit(HeaderMsg::EnableNext(false));
        match self.widgets.import_picker_btn.filename() {
            None => {
                // shouldn't happen, but i don't want to crash
                self.show_import_error("Please pick a file to import");
            }
            Some(fname) => {
                let import_result_sender = self.model.import_result_sender.clone();
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        import_result_sender
                            .send(
                                sql_conn
                                    .transaction(|| {
                                        import::do_import(sql_conn, &fname.to_string_lossy(), &pass)
                                    })
                                    .map_err(|e| e.to_string()),
                            )
                            .unwrap();
                    }))
                    .unwrap();
            }
        }
    }

    fn do_export(&self, fname: path::PathBuf, pass: String) {
        let selected_projects: Vec<_> = self
            .widgets
            .project_list
            .selected_rows()
            .into_iter()
            .flat_map(|row| {
                self.model
                    .displayed_projects
                    .get(row.index() as usize)
                    .cloned()
            })
            .collect();
        self.model
            .header
            .stream()
            .emit(HeaderMsg::EnableNext(false));
        let s = self.model.export_result_sender.clone();
        self.model
            .db_sender
            .send(SqlFunc::new(move |sql_conn| {
                s.send(
                    export::export_projects(sql_conn, &selected_projects, &fname, &pass)
                        .map_err(|e| e.to_string()),
                )
                .unwrap();
            }))
            .unwrap();
    }

    view! {
        #[name="import_win"]
        gtk::Window {
            titlebar: Some(self.model.header.widget()),
            default_width: 600,
            default_height: 300, // need more height for the export due to the project list
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
                        label: "Export to file",
                    },
                },
                #[name="grid"]
                gtk::Grid {
                    child: {
                        name: Some(CHILD_NAME_IMPORT)
                    },
                    #[name="import_error_infobar"]
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
                        file_set => Msg::FilePicked,
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
                    #[name="import_password_entry"]
                    PasswordField(("".to_string(), password_field::ActivatesDefault::Yes)) {
                        hexpand: true,
                        cell: {
                            left_attach: 1,
                            top_attach: 2,
                        },
                        PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPassword(pass.clone()),
                    },
                },
                #[name="grid_export"]
                gtk::Grid {
                    child: {
                        name: Some(CHILD_NAME_EXPORT)
                    },
                    #[name="export_error_infobar"]
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
                        text: "Projects to export:",
                        cell: {
                            left_attach: 0,
                            top_attach: 1,
                        }
                    },
                    gtk::ScrolledWindow {
                        cell: {
                            left_attach: 0,
                            top_attach: 2,
                            width: 2,
                        },
                        vexpand: true,
                        #[name="project_list"]
                        gtk::ListBox {
                            hexpand: true,
                            selection_mode: gtk::SelectionMode::Multiple,
                            // https://gitlab.gnome.org/GNOME/gtk/-/issues/497
                            activate_on_single_click: false,
                        },
                    },
                    gtk::Label {
                        text: "Password",
                        halign: gtk::Align::End,
                        cell: {
                            left_attach: 0,
                            top_attach: 3,
                        },
                    },
                    #[name="export_password_entry"]
                    PasswordField(("".to_string(), password_field::ActivatesDefault::Yes)) {
                        hexpand: true,
                        cell: {
                            left_attach: 1,
                            top_attach: 3,
                        },
                        PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPassword(pass.clone()),
                    },
                    gtk::Label {
                        text: "Password confirm",
                        halign: gtk::Align::End,
                        cell: {
                            left_attach: 0,
                            top_attach: 4,
                        },
                    },
                    #[name="export_password_confirm_entry"]
                    PasswordField(("".to_string(), password_field::ActivatesDefault::Yes)) {
                        hexpand: true,
                        cell: {
                            left_attach: 1,
                            top_attach: 4,
                        },
                        PasswordFieldMsgPublishPassword(ref pass) => Msg::GotPasswordConfirm(pass.clone()),
                    },
                }
            },
            key_press_event(_, key) => (Msg::KeyPress(key.clone()), Inhibit(false)), // just for the ESC key.. surely there's a better way..
        }
    }
}
