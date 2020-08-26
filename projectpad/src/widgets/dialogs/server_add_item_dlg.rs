use super::server_database_add_edit_dlg;
use super::server_database_add_edit_dlg::Msg as MsgServerDatabaseAddEditDialog;
use super::server_database_add_edit_dlg::ServerDatabaseAddEditDialog;
use super::server_poi_add_edit_dlg;
use super::server_poi_add_edit_dlg::Msg as MsgServerPoiAddEditDialog;
use super::server_poi_add_edit_dlg::ServerPoiAddEditDialog;
use super::AddEditDialogComponent;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg, Debug)]
pub enum Msg {
    ShowSecondTab,
    OkPressed,
    ActionCompleted,
    ChangeDialogTitle(&'static str),
}

pub struct Model {
    relm: relm::Relm<ServerAddItemDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    dialog_component: Option<AddEditDialogComponent>,
}

#[widget]
impl Widget for ServerAddItemDialog {
    fn init_view(&mut self) {
        self.add_db.join_group(Some(&self.add_poi));
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
            Msg::ShowSecondTab => {
                let (widget, title) = if self.add_poi.get_active() {
                    let dialog_contents = relm::init::<ServerPoiAddEditDialog>((
                        self.model.db_sender.clone(),
                        self.model.server_id,
                        None,
                    ))
                    .expect("error initializing the server poi add edit modal");
                    relm::connect!(
                        dialog_contents@MsgServerPoiAddEditDialog::ServerPoiUpdated(_),
                        self.model.relm,
                        Msg::ActionCompleted
                    );
                    self.model.dialog_component =
                        Some(AddEditDialogComponent::Poi(dialog_contents));
                    (
                        self.model
                            .dialog_component
                            .as_ref()
                            .unwrap()
                            .un_poi()
                            .unwrap()
                            .widget(),
                        "Add Server Point of Interest",
                    )
                } else if self.add_db.get_active() {
                    let dialog_contents = relm::init::<ServerDatabaseAddEditDialog>((
                        self.model.db_sender.clone(),
                        self.model.server_id,
                        None,
                    ))
                    .expect("error initializing the server db add edit modal");
                    relm::connect!(
                        dialog_contents@MsgServerDatabaseAddEditDialog::ServerDbUpdated(_),
                        self.model.relm,
                        Msg::ActionCompleted
                    );
                    self.model.dialog_component = Some(AddEditDialogComponent::Db(dialog_contents));
                    (
                        self.model
                            .dialog_component
                            .as_ref()
                            .unwrap()
                            .un_db()
                            .unwrap()
                            .widget(),
                        "Add server database",
                    )
                } else {
                    panic!();
                };
                self.model.relm.stream().emit(Msg::ChangeDialogTitle(title));
                self.tabs_stack.add_named(widget, "dialog");
                widget.show();
                self.tabs_stack.set_visible_child_name("dialog");
            }
            Msg::OkPressed => match self.model.dialog_component.as_ref() {
                Some(AddEditDialogComponent::Poi(poi_c)) => {
                    poi_c.stream().emit(server_poi_add_edit_dlg::Msg::OkPressed)
                }
                Some(AddEditDialogComponent::Db(poi_d)) => poi_d
                    .stream()
                    .emit(server_database_add_edit_dlg::Msg::OkPressed),
                x => eprintln!("Got ok but wrong component? {}", x.is_some()),
            },
            // meant for my parent
            Msg::ChangeDialogTitle(_) => {}
            // meant for my parent
            Msg::ActionCompleted => {}
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
                spacing: 8,
                orientation: gtk::Orientation::Vertical,
                #[name="add_poi"]
                gtk::RadioButton {
                    label: "Add point of interest",
                },
                #[name="add_db"]
                gtk::RadioButton {
                    label: "Add database",
                },
            }
        }
    }
}
