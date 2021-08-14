use super::dialog_helpers;
use super::project_note_add_edit_dlg;
use super::project_note_add_edit_dlg::Msg as MsgProjectNoteAddEditDialog;
use super::project_note_add_edit_dlg::ProjectNoteAddEditDialog;
use super::project_poi_add_edit_dlg;
use super::project_poi_add_edit_dlg::Msg as MsgProjectPoiAddEditDialog;
use super::project_poi_add_edit_dlg::ProjectPoiAddEditDialog;
use super::server_add_edit_dlg;
use super::server_add_edit_dlg::Msg as MsgServerAddEditDialog;
use super::server_add_edit_dlg::ServerAddEditDialog;
use super::server_link_add_edit_dlg;
use super::server_link_add_edit_dlg::Msg as MsgServerLinkAddEditDialog;
use super::server_link_add_edit_dlg::ServerLinkAddEditDialog;
use super::standard_dialogs;
use super::ProjectAddEditDialogComponent;
use crate::export::ServerImportExportClipboard;
use crate::import;
use crate::sql_thread::SqlFunc;
use crate::widgets::project_items_list::ProjectItem;
use diesel::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::EnvironmentType;
use projectpadsql::models::Server;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::path::PathBuf;
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    ShowSecondTab(gtk::Dialog),
    ChangeDialogTitle(&'static str),
    OkPressed,
    ActionCompleted(Box<ProjectItem>), // large enum variant => box it
    ServerImportApplied,
}

// String for details, because I can't pass Error across threads
type PasteResult = Result<Server, String>;

pub struct Model {
    relm: relm::Relm<ProjectAddItemDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    project_id: i32,
    environment_type: EnvironmentType,
    dialog_component: Option<ProjectAddEditDialogComponent>,

    _paste_processed_channel: relm::Channel<PasteResult>,
    paste_processed_sender: relm::Sender<PasteResult>,
}

#[widget]
impl Widget for ProjectAddItemDialog {
    fn init_view(&mut self) {
        self.widgets
            .add_project_poi
            .join_group(Some(&self.widgets.add_server));
        self.widgets
            .add_project_note
            .join_group(Some(&self.widgets.add_server));
        self.widgets
            .add_server_link
            .join_group(Some(&self.widgets.add_server));
        self.widgets
            .add_server_clipboard
            .join_group(Some(&self.widgets.add_server));

        self.widgets.add_server_clipboard.set_sensitive(false);
        if self.read_server_import_export_clipboard().is_some() {
            self.widgets.add_server_clipboard.set_sensitive(true);
        }
    }

    fn read_server_import_export_clipboard(&self) -> Option<ServerImportExportClipboard> {
        if let Some(clip) = gtk::Clipboard::get_default(&self.widgets.tabs_stack.get_display()) {
            if clip.wait_is_text_available() {
                if let Some(txt) = clip.wait_for_text() {
                    return serde_yaml::from_str::<ServerImportExportClipboard>(&txt).ok();
                }
            }
        }
        None
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, i32, EnvironmentType),
    ) -> Model {
        let (db_sender, project_id, environment_type) = params;
        let stream = relm.stream().clone();
        let (_paste_processed_channel, paste_processed_sender) =
            relm::Channel::new(move |r: PasteResult| match r {
                Ok(srv) => stream.emit(Msg::ActionCompleted(Box::new(ProjectItem::Server(srv)))),
                Err(e) => standard_dialogs::display_error_str("Error pasting server", Some(e)),
            });
        Model {
            relm: relm.clone(),
            db_sender,
            project_id,
            environment_type,
            dialog_component: None,
            _paste_processed_channel,
            paste_processed_sender,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ShowSecondTab(ref dialog) if self.widgets.add_server_clipboard.get_active() => {
                if let Some(server_import) = self.read_server_import_export_clipboard() {
                    let project_id = self.model.project_id;
                    let environment_type = self.model.environment_type;
                    let s = self.model.paste_processed_sender.clone();
                    self.model
                        .db_sender
                        .send(SqlFunc::new(move |sql_conn| {
                            use projectpadsql::schema::server::dsl as srv;
                            let import_res = import::import_server_attach(
                                sql_conn,
                                |attach_key| {
                                    server_import
                                        .extra_files
                                        .get(&PathBuf::from(attach_key))
                                        .map(|v| Result::Ok(v.clone()))
                                },
                                project_id,
                                environment_type,
                                None,
                                &server_import.server_data,
                            );
                            match import_res {
                                Err(e) => s
                                    .send(Err(format!("Error pasting server: {:?}", e)))
                                    .unwrap(),
                                Ok((srv_id, unprocessed_websites)) => {
                                    for unprocessed_website in unprocessed_websites {
                                        if let Err(e) = import::import_server_website(
                                            sql_conn,
                                            &unprocessed_website,
                                        ) {
                                            s.send(Err(format!(
                                                "Error pasting server website: {:?}",
                                                e
                                            )))
                                            .unwrap();
                                            return;
                                        }
                                    }
                                    let srv = srv::server
                                        .filter(srv::id.eq(srv_id))
                                        .first::<Server>(sql_conn)
                                        .unwrap();
                                    s.send(Ok(srv)).unwrap();
                                }
                            }
                        }))
                        .unwrap();
                }
            }
            Msg::ShowSecondTab(ref dialog) => {
                let (widget, title) = if self.widgets.add_server.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.project_id,
                            ServerAddEditDialog,
                            MsgServerAddEditDialog::ServerUpdated,
                            ProjectAddEditDialogComponent::Server,
                            |s| Box::new(ProjectItem::Server(s)),
                        ),
                        "Add Server",
                    )
                } else if self.widgets.add_project_poi.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.project_id,
                            ProjectPoiAddEditDialog,
                            MsgProjectPoiAddEditDialog::PoiUpdated,
                            ProjectAddEditDialogComponent::ProjectPoi,
                            |p| Box::new(ProjectItem::ProjectPointOfInterest(p)),
                        ),
                        "Add Project POI",
                    )
                } else if self.widgets.add_project_note.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.project_id,
                            ProjectNoteAddEditDialog,
                            MsgProjectNoteAddEditDialog::ProjectNoteUpdated,
                            ProjectAddEditDialogComponent::ProjectNote,
                            |n| Box::new(ProjectItem::ProjectNote(n)),
                        ),
                        "Add Project note",
                    )
                } else if self.widgets.add_server_link.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.project_id,
                            ServerLinkAddEditDialog,
                            MsgServerLinkAddEditDialog::ServerLinkUpdated,
                            ProjectAddEditDialogComponent::ServerLink,
                            |l| Box::new(ProjectItem::ServerLink(l)),
                        ),
                        "Add server link",
                    )
                } else {
                    panic!();
                };
                match self.model.dialog_component.as_ref() {
                    Some(ProjectAddEditDialogComponent::ServerLink(lnk)) => {
                        lnk.stream()
                            .emit(MsgServerLinkAddEditDialog::SetEnvironmentType(
                                self.model.environment_type,
                            ))
                    }
                    Some(ProjectAddEditDialogComponent::Server(srv)) => {
                        srv.stream()
                            .emit(MsgServerAddEditDialog::SetEnvironmentType(
                                self.model.environment_type,
                            ))
                    }
                    _ => {}
                };
                self.model.relm.stream().emit(Msg::ChangeDialogTitle(title));
                self.widgets.tabs_stack.add_named(widget, "dialog");
                widget.show();
                self.widgets.tabs_stack.set_visible_child_name("dialog");
            }
            // meant for my parent
            Msg::ChangeDialogTitle(_) => {}
            Msg::OkPressed => match self.model.dialog_component.as_ref() {
                Some(ProjectAddEditDialogComponent::ProjectPoi(poi_c)) => poi_c
                    .stream()
                    .emit(project_poi_add_edit_dlg::Msg::OkPressed),
                Some(ProjectAddEditDialogComponent::Server(srv_c)) => {
                    srv_c.stream().emit(server_add_edit_dlg::Msg::OkPressed)
                }
                Some(ProjectAddEditDialogComponent::ProjectNote(srv_c)) => srv_c
                    .stream()
                    .emit(project_note_add_edit_dlg::Msg::OkPressed),
                Some(ProjectAddEditDialogComponent::ServerLink(srv_c)) => srv_c
                    .stream()
                    .emit(server_link_add_edit_dlg::Msg::OkPressed),
                x => eprintln!("Got ok but wrong component? {}", x.is_some()),
            },
            // meant for my parent
            Msg::ActionCompleted(_) => {}
            Msg::ServerImportApplied => {}
        }
    }

    view! {
        #[name="tabs_stack"]
        gtk::Stack {
            gtk::Box {
                margin_top: 15,
                margin_start: 15,
                margin_end: 15,
                margin_bottom: 15,
                spacing: 10,
                orientation: gtk::Orientation::Vertical,
                #[name="add_server"]
                gtk::RadioButton {
                    label: "Add server",
                },
                #[name="add_project_poi"]
                gtk::RadioButton {
                    label: "Add point of interest",
                },
                #[name="add_project_note"]
                gtk::RadioButton {
                    label: "Add project note",
                },
                #[name="add_server_link"]
                gtk::RadioButton {
                    label: "Add server link",
                },
                #[name="add_server_clipboard"]
                gtk::RadioButton {
                    label: "Add server from clipboard",
                },
            }
        }
    }
}
