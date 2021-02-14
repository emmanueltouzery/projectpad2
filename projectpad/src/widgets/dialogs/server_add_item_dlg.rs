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
use crate::sql_thread::SqlFunc;
use crate::widgets::server_poi_contents::ServerItem;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Debug)]
pub enum Msg {
    ShowSecondTab(gtk::Dialog),
    OkPressed,
    ActionCompleted(ServerItem),
    ChangeDialogTitle(&'static str),
}

pub struct Model {
    relm: relm::Relm<ServerAddItemDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    dialog_component: Option<ServerAddEditDialogComponent>,
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
    }};
    }

#[widget]
impl Widget for ServerAddItemDialog {
    fn init_view(&mut self) {
        self.widgets.add_db.join_group(Some(&self.widgets.add_poi));
        self.widgets
            .add_extra_user
            .join_group(Some(&self.widgets.add_poi));
        self.widgets
            .add_website
            .join_group(Some(&self.widgets.add_poi));
        self.widgets
            .add_note
            .join_group(Some(&self.widgets.add_poi));
    }

    fn model(relm: &relm::Relm<Self>, params: (mpsc::Sender<SqlFunc>, i32)) -> Model {
        let (db_sender, server_id) = params;
        Model {
            relm: relm.clone(),
            db_sender,
            server_id,
            dialog_component: None,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ShowSecondTab(ref dialog) => {
                let (widget, title) = if self.widgets.add_poi.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.server_id,
                            ServerPoiAddEditDialog,
                            MsgServerPoiAddEditDialog::ServerPoiUpdated,
                            ServerAddEditDialogComponent::Poi,
                            ServerItem::PointOfInterest,
                        ),
                        "Add Server Point of Interest",
                    )
                } else if self.widgets.add_db.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.server_id,
                            ServerDatabaseAddEditDialog,
                            MsgServerDatabaseAddEditDialog::ServerDbUpdated,
                            ServerAddEditDialogComponent::Db,
                            ServerItem::Database,
                        ),
                        "Add server database",
                    )
                } else if self.widgets.add_extra_user.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.server_id,
                            ServerExtraUserAddEditDialog,
                            MsgServerExtraUserAddEditDialog::ServerUserUpdated,
                            ServerAddEditDialogComponent::User,
                            ServerItem::ExtraUserAccount,
                        ),
                        "Add server extra user",
                    )
                } else if self.widgets.add_website.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.server_id,
                            ServerWebsiteAddEditDialog,
                            MsgServerWebsiteAddEditDialog::ServerWwwUpdated,
                            ServerAddEditDialogComponent::Website,
                            |w_db: Box<(_, _)>| { ServerItem::Website((*w_db).0) },
                        ),
                        "Add server website",
                    )
                } else if self.widgets.add_note.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.server_id,
                            ServerNoteAddEditDialog,
                            MsgServerNoteAddEditDialog::ServerNoteUpdated,
                            ServerAddEditDialogComponent::Note,
                            ServerItem::Note,
                        ),
                        "Add server note",
                    )
                } else {
                    panic!();
                };
                self.model.relm.stream().emit(Msg::ChangeDialogTitle(title));
                self.widgets.tabs_stack.add_named(widget, "dialog");
                widget.show();
                self.widgets.tabs_stack.set_visible_child_name("dialog");
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
                #[name="add_poi"]
                gtk::RadioButton {
                    label: "Add point of interest",
                },
                #[name="add_website"]
                gtk::RadioButton {
                    label: "Add website",
                },
                #[name="add_db"]
                gtk::RadioButton {
                    label: "Add database",
                },
                #[name="add_extra_user"]
                gtk::RadioButton {
                    label: "Add extra user",
                },
                #[name="add_note"]
                gtk::RadioButton {
                    label: "Add note",
                },
            }
        }
    }
}
