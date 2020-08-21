use super::server_poi_add_edit_dlg;
use super::server_poi_add_edit_dlg::Msg as MsgServerPoiAddEditDialog;
use super::server_poi_add_edit_dlg::ServerPoiAddEditDialog;
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
}

pub struct Model {
    relm: relm::Relm<ServerAddItemDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    server_id: i32,
    server_poi_add_edit_dialog: Option<relm::Component<ServerPoiAddEditDialog>>,
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
            server_poi_add_edit_dialog: None,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ShowSecondTab => {
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
                let widget = dialog_contents.widget();
                self.tabs_stack.add_named(widget, "dialog");
                widget.show();
                self.tabs_stack.set_visible_child_name("dialog");
                self.model.server_poi_add_edit_dialog = Some(dialog_contents);
            }
            Msg::OkPressed => self
                .model
                .server_poi_add_edit_dialog
                .as_ref()
                .unwrap()
                .stream()
                .emit(server_poi_add_edit_dlg::Msg::OkPressed),
            // meant for my parent
            Msg::ActionCompleted => {}
        }
    }

    view! {
        #[name="tabs_stack"]
        gtk::Stack {
            gtk::Box {
                margin_top: 10,
                margin_start: 10,
                margin_end: 10,
                margin_bottom: 10,
                spacing: 3,
                orientation: gtk::Orientation::Vertical,
                gtk::RadioButton {
                    label: "Add point of interest",
                },
            }
        }
    }
}
