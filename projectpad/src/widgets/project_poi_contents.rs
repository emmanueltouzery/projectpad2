use super::project_items_list::ProjectItem;
use super::server_poi_contents::Msg as ServerPoiContentsMsg;
use super::server_poi_contents::ServerPoiContents;
use crate::sql_thread::SqlFunc;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    ProjectItemSelected(Option<ProjectItem>),
    ActivateLink(String),
}

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    cur_project_item: Option<ProjectItem>,
    project_note_contents: Option<String>,
}

const CHILD_NAME_SERVER: &str = "server";
const CHILD_NAME_NOTE: &str = "note";

#[widget]
impl Widget for ProjectPoiContents {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        Model {
            db_sender,
            cur_project_item: None,
            project_note_contents: None,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectItemSelected(pi) => {
                self.model.cur_project_item = pi;
                self.server_contents
                    .emit(ServerPoiContentsMsg::ServerSelected(
                        self.model
                            .cur_project_item
                            .as_ref()
                            .and_then(|pi| match pi {
                                ProjectItem::Server(srv) => Some(srv.clone()),
                                _ => None,
                            }),
                    ));
                self.model.project_note_contents =
                    self.model
                        .cur_project_item
                        .as_ref()
                        .and_then(|pi| match pi {
                            ProjectItem::ProjectNote(ref note) => Some(
                                crate::notes::note_markdown_to_pango_markup(note.contents.as_ref()),
                            ),
                            _ => None,
                        });
                self.contents_stack
                    .set_visible_child_name(match self.model.cur_project_item {
                        Some(ProjectItem::ProjectNote(_)) => CHILD_NAME_NOTE,
                        _ => CHILD_NAME_SERVER, // server is a list of items, handles None well (no items)
                    });
            }
            Msg::ActivateLink(uri) => {
                if uri.starts_with("pass://") {
                    println!("activate pass {}", &uri[7..]);
                }
            }
        }
    }

    view! {
        #[name="contents_stack"]
        gtk::Stack {
            #[name="server_contents"]
            ServerPoiContents(self.model.db_sender.clone()) {
                child: {
                    name: Some(CHILD_NAME_SERVER)
                }
            },
            gtk::ScrolledWindow {
                child: {
                    name: Some(CHILD_NAME_NOTE)
                },
                gtk::Label {
                    margin_top: 10,
                    margin_start: 10,
                    margin_end: 10,
                    margin_bottom: 10,
                    xalign: 0.0,
                    yalign: 0.0,
                    selectable: true,
                    markup: self.model.project_note_contents
                                      .as_ref().map(|c| c.as_str()).unwrap_or(""),
                    activate_link(_, uri) => (Msg::ActivateLink(uri.to_string()), Inhibit(false))
                }
            }
        }
    }
}
