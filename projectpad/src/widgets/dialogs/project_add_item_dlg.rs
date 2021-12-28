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
use super::ProjectAddEditDialogComponent;
use crate::icons::Icon;
use crate::sql_thread::SqlFunc;
use crate::widgets::project_items_list::ProjectItem;
use crate::widgets::title_subtitle_btn::Msg::Clicked;
use crate::widgets::title_subtitle_btn::TitleSubtitleBtn;
use gtk::prelude::*;
use projectpadsql::models::EnvironmentType;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    DialogSet(gtk::Dialog),
    AddServer,
    AddProjectPoi,
    AddProjectNote,
    AddServerLink,
    ChangeDialogTitle(&'static str),
    OkPressed,
    ActionCompleted(Box<ProjectItem>), // large enum variant => box it
}

pub struct Model {
    relm: relm::Relm<ProjectAddItemDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    dialog: Option<gtk::Dialog>,
    project_id: i32,
    environment_type: EnvironmentType,
    dialog_component: Option<ProjectAddEditDialogComponent>,
}

#[widget]
impl Widget for ProjectAddItemDialog {
    fn init_view(&mut self) {}

    fn model(
        relm: &relm::Relm<Self>,
        params: (mpsc::Sender<SqlFunc>, i32, EnvironmentType),
    ) -> Model {
        let (db_sender, project_id, environment_type) = params;
        Model {
            relm: relm.clone(),
            db_sender,
            project_id,
            environment_type,
            dialog: None,
            dialog_component: None,
        }
    }

    fn move_to_second_tab(&mut self, widget: &gtk::Widget, title: &'static str) {
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
        // TODO ideally i'd like to shrink the dialog vertically as the new
        // tab may be less tall than the previous one. But I didn't manage to
        // achieve that. So I center the component vertically at least.
        widget.set_valign(gtk::Align::Center);
        widget.show();
        self.widgets.tabs_stack.set_visible_child_name("dialog");
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::DialogSet(d) => {
                self.model.dialog = Some(d);
            }
            Msg::AddServer => {
                let dlg = self.model.dialog.as_ref().unwrap();
                let widget = plug_second_tab!(
                    self,
                    dlg,
                    self.model.project_id,
                    ServerAddEditDialog,
                    MsgServerAddEditDialog::ServerUpdated,
                    ProjectAddEditDialogComponent::Server,
                    |s| Box::new(ProjectItem::Server(s)),
                );
                self.move_to_second_tab(&widget, "Add Server");
            }
            Msg::AddProjectPoi => {
                let dlg = self.model.dialog.as_ref().unwrap();
                let widget = plug_second_tab!(
                    self,
                    dlg,
                    self.model.project_id,
                    ProjectPoiAddEditDialog,
                    MsgProjectPoiAddEditDialog::PoiUpdated,
                    ProjectAddEditDialogComponent::ProjectPoi,
                    |p| Box::new(ProjectItem::ProjectPointOfInterest(p)),
                );
                self.move_to_second_tab(&widget, "Add Project POI");
            }
            Msg::AddProjectNote => {
                let dlg = self.model.dialog.as_ref().unwrap();
                let widget = plug_second_tab!(
                    self,
                    dlg,
                    self.model.project_id,
                    ProjectNoteAddEditDialog,
                    MsgProjectNoteAddEditDialog::ProjectNoteUpdated,
                    ProjectAddEditDialogComponent::ProjectNote,
                    |n| Box::new(ProjectItem::ProjectNote(n)),
                );
                self.move_to_second_tab(&widget, "Add Project note");
            }
            Msg::AddServerLink => {
                let dlg = self.model.dialog.as_ref().unwrap();
                let widget = plug_second_tab!(
                    self,
                    dlg,
                    self.model.project_id,
                    ServerLinkAddEditDialog,
                    MsgServerLinkAddEditDialog::ServerLinkUpdated,
                    ProjectAddEditDialogComponent::ServerLink,
                    |l| Box::new(ProjectItem::ServerLink(l)),
                );
                self.move_to_second_tab(&widget, "Add server link");
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
                TitleSubtitleBtn(
                    Icon::SERVER,
                    "Add server",
                    "machines or virtual machines, with their own IP.",
                    ) {
                    Clicked => Msg::AddServer,
                },
                TitleSubtitleBtn(
                    Icon::POINT_OF_INTEREST,
                    "Add point of interest",
                    "commands to run or relevant files or folders.",
                    ) {
                    Clicked => Msg::AddProjectPoi,
                },
                TitleSubtitleBtn(
                    Icon::NOTE,
                    "Add project note",
                    "markdown-formatted text containing free-form text."
                    ) {
                    Clicked => Msg::AddProjectNote,
                },
                TitleSubtitleBtn(
                    Icon::SERVER_LINK,
                    "Add server link",
                    "when a server is shared, we can enter it just once and 'link' to it."
                    ) {
                    Clicked => Msg::AddServerLink,
                },
            }
        }
    }
}
