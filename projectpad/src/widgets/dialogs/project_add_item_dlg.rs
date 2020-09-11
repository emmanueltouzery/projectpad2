use super::dialog_helpers;
use super::project_poi_add_edit_dlg::Msg as MsgProjectPoiAddEditDialog;
use super::project_poi_add_edit_dlg::ProjectPoiAddEditDialog;
use super::server_add_edit_dlg::Msg as MsgServerAddEditDialog;
use super::server_add_edit_dlg::ServerAddEditDialog;
use super::ProjectAddEditDialogComponent;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    ShowSecondTab(gtk::Dialog),
    ChangeDialogTitle(&'static str),
    ActionCompleted,
}

pub struct Model {
    relm: relm::Relm<ProjectAddItemDialog>,
    db_sender: mpsc::Sender<SqlFunc>,
    project_id: i32,
    dialog_component: Option<ProjectAddEditDialogComponent>,
}

#[widget]
impl Widget for ProjectAddItemDialog {
    fn init_view(&mut self) {
        self.add_project_poi.join_group(Some(&self.add_server));
    }

    fn model(relm: &relm::Relm<Self>, params: (mpsc::Sender<SqlFunc>, i32)) -> Model {
        let (db_sender, project_id) = params;
        Model {
            relm: relm.clone(),
            db_sender,
            project_id,
            dialog_component: None,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ShowSecondTab(ref dialog) => {
                let (widget, title) = if self.add_server.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.project_id,
                            ServerAddEditDialog,
                            MsgServerAddEditDialog::ServerUpdated,
                            ProjectAddEditDialogComponent::Server,
                        ),
                        "Add Server",
                    )
                } else if self.add_project_poi.get_active() {
                    (
                        plug_second_tab!(
                            self,
                            dialog,
                            self.model.project_id,
                            ProjectPoiAddEditDialog,
                            MsgProjectPoiAddEditDialog::PoiUpdated,
                            ProjectAddEditDialogComponent::ProjectPoi,
                        ),
                        "Add Project POI",
                    )
                } else {
                    panic!();
                };
                self.model.relm.stream().emit(Msg::ChangeDialogTitle(title));
                self.tabs_stack.add_named(widget, "dialog");
                widget.show();
                self.tabs_stack.set_visible_child_name("dialog");
            }
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
            }
        }
    }
}
