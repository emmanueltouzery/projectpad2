use super::dialog_helpers;
use super::server_database_add_edit_dlg;
use super::server_database_add_edit_dlg::Msg as MsgServerDatabaseAddEditDialog;
use super::server_database_add_edit_dlg::ServerDatabaseAddEditDialog;
use super::server_extra_user_add_edit_dlg;
use super::server_extra_user_add_edit_dlg::Msg as MsgServerExtraUserAddEditDialog;
use super::server_extra_user_add_edit_dlg::ServerExtraUserAddEditDialog;
use super::server_note_add_edit_dlg;
use super::server_note_add_edit_dlg::Msg as MsgServerNoteAddEditDialog;
use super::server_note_add_edit_dlg::ServerNoteAddEditDialog;
use super::server_poi_add_edit_dlg;
use super::server_poi_add_edit_dlg::Msg as MsgServerPoiAddEditDialog;
use super::server_poi_add_edit_dlg::ServerPoiAddEditDialog;
use super::server_website_add_edit_dlg;
use super::server_website_add_edit_dlg::Msg as MsgServerWebsiteAddEditDialog;
use super::server_website_add_edit_dlg::ServerWebsiteAddEditDialog;
use super::ServerAddEditDialogComponent;
use crate::icons::*;
use crate::sql_thread::SqlFunc;
use crate::widgets::server_poi_contents::ServerItem;
use crate::widgets::title_subtitle_btn::Msg::Clicked;
use crate::widgets::title_subtitle_btn::TitleSubtitleBtn;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Debug)]
pub enum Msg {
    DialogSet(gtk::Dialog),
    OkPressed,
    ActionCompleted(ServerItem),
    ChangeDialogTitle(&'static str),
    AddServerPoi,
    AddDatabase,
    AddExtraUser,
    AddWebsite,
    AddNote,
}

pub struct Model {
    relm: relm::Relm<ServerAddItemDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    dialog_component: Option<ServerAddEditDialogComponent>,
    dialog: Option<gtk::Dialog>,
}

// i would really like to use a function not a macro here, but
// because of the relm::connect! i don't see many other options...
macro_rules! plug_second_tab {
    ($self: ident, $dialog: ident, $parent: expr, $dialog_type:tt,
     $event: path, $component_ctor: path, $to_server_item: expr,) => {{
        let dialog_params = dialog_helpers::prepare_dialog_param(
                $self.model.db_sender.clone(),
                $parent,
                None,
        );
        $dialog.add_accel_group(&dialog_params.3);
        let dialog_contents = relm::init::<$dialog_type>(dialog_params)
                .expect("error initializing add edit modal");
        relm::connect!(
            dialog_contents@$event(ref x),
            $self.model.relm,
            Msg::ActionCompleted($to_server_item(x.clone()))
        );
        $self.model.dialog_component = Some($component_ctor(dialog_contents));
        $self.model
            .dialog_component
            .as_ref()
            .unwrap()
            .get_widget()
            .clone()
            .upcast::<gtk::Widget>()
    }};
    }

#[widget]
impl Widget for ServerAddItemDialog {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, params: (mpsc::Sender<SqlFunc>, i32)) -> Model {
        let (db_sender, server_id) = params;
        Model {
            relm: relm.clone(),
            db_sender,
            server_id,
            dialog_component: None,
            dialog: None,
        }
    }

    fn move_to_second_tab(&mut self, widget: &gtk::Widget, title: &'static str) {
        self.model.relm.stream().emit(Msg::ChangeDialogTitle(title));
        self.widgets.tabs_stack.add_named(widget, "dialog");
        widget.set_valign(gtk::Align::Center);
        widget.show();
        self.widgets.tabs_stack.set_visible_child_name("dialog");
        // TODO ideally i'd like to shrink the dialog vertically as the new
        // tab may be less tall than the previous one. But I didn't manage to
        // achieve that.
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::DialogSet(d) => {
                self.model.dialog = Some(d);
            }
            Msg::AddServerPoi => {
                let dlg = self.model.dialog.as_ref().unwrap();
                let widget = plug_second_tab!(
                    self,
                    dlg,
                    self.model.server_id,
                    ServerPoiAddEditDialog,
                    MsgServerPoiAddEditDialog::ServerPoiUpdated,
                    ServerAddEditDialogComponent::Poi,
                    ServerItem::PointOfInterest,
                );
                self.move_to_second_tab(&widget, "Add Server Point of Interest")
            }
            Msg::AddDatabase => {
                let dlg = self.model.dialog.as_ref().unwrap();
                let widget = plug_second_tab!(
                    self,
                    dlg,
                    self.model.server_id,
                    ServerDatabaseAddEditDialog,
                    MsgServerDatabaseAddEditDialog::ServerDbUpdated,
                    ServerAddEditDialogComponent::Db,
                    ServerItem::Database,
                );
                self.move_to_second_tab(&widget, "Add server database")
            }
            Msg::AddExtraUser => {
                let dlg = self.model.dialog.as_ref().unwrap();
                let widget = plug_second_tab!(
                    self,
                    dlg,
                    self.model.server_id,
                    ServerExtraUserAddEditDialog,
                    MsgServerExtraUserAddEditDialog::ServerUserUpdated,
                    ServerAddEditDialogComponent::User,
                    ServerItem::ExtraUserAccount,
                );
                self.move_to_second_tab(&widget, "Add server extra user")
            }
            Msg::AddWebsite => {
                let dlg = self.model.dialog.as_ref().unwrap();
                let widget = plug_second_tab!(
                    self,
                    dlg,
                    self.model.server_id,
                    ServerWebsiteAddEditDialog,
                    MsgServerWebsiteAddEditDialog::ServerWwwUpdated,
                    ServerAddEditDialogComponent::Website,
                    |w_db: Box<(_, _)>| { ServerItem::Website((*w_db).0) },
                );
                self.move_to_second_tab(&widget, "Add server website")
            }
            Msg::AddNote => {
                let dlg = self.model.dialog.as_ref().unwrap();
                let widget = plug_second_tab!(
                    self,
                    dlg,
                    self.model.server_id,
                    ServerNoteAddEditDialog,
                    MsgServerNoteAddEditDialog::ServerNoteUpdated,
                    ServerAddEditDialogComponent::Note,
                    ServerItem::Note,
                );
                self.move_to_second_tab(&widget, "Add server note")
            }
            Msg::OkPressed => match self.model.dialog_component.as_ref() {
                Some(ServerAddEditDialogComponent::Poi(poi_c)) => {
                    poi_c.stream().emit(server_poi_add_edit_dlg::Msg::OkPressed)
                }
                Some(ServerAddEditDialogComponent::Db(poi_d)) => poi_d
                    .stream()
                    .emit(server_database_add_edit_dlg::Msg::OkPressed),
                Some(ServerAddEditDialogComponent::Website(www_d)) => www_d
                    .stream()
                    .emit(server_website_add_edit_dlg::Msg::OkPressed),
                Some(ServerAddEditDialogComponent::User(user_d)) => user_d
                    .stream()
                    .emit(server_extra_user_add_edit_dlg::Msg::OkPressed),
                Some(ServerAddEditDialogComponent::Note(note_d)) => note_d
                    .stream()
                    .emit(server_note_add_edit_dlg::Msg::OkPressed),
                x => eprintln!("Got ok but wrong component? {}", x.is_some()),
            },
            // meant for my parent
            Msg::ChangeDialogTitle(_) => {}
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
                    Icon::POINT_OF_INTEREST,
                    "Add point of interest",
                    "a command to run or a relevant file or folder located on that server."
                    ) {
                    Clicked => Msg::AddServerPoi,
                },
                TitleSubtitleBtn(
                    Icon::HTTP,
                    "Add website",
                    "a service (website or not) that's reachable over the network that lives \
                                       on that server."
                    ) {
                    Clicked => Msg::AddWebsite,
                },
                TitleSubtitleBtn(
                    Icon::DATABASE,
                    "Add database",
                    "a database that lives on that server."
                    ) {
                    Clicked => Msg::AddDatabase,
                },
                TitleSubtitleBtn(
                    Icon::USER,
                    "Add extra user",
                    "username and password or \
                        authentication key, somehow tied to this server."
                    ) {
                    Clicked => Msg::AddExtraUser,
                },
                TitleSubtitleBtn(
                    Icon::NOTE,
                    "Add note",
                    "markdown-formatted text containing free-form text."
                    ) {
                    Clicked => Msg::AddNote,
                },
            }
        }
    }
}
