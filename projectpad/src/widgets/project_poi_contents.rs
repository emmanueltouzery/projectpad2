use super::project_items_list::ProjectItem;
use super::server_poi_contents::Msg as ServerPoiContentsMsg;
use super::server_poi_contents::ServerPoiContents;
use crate::sql_thread::SqlFunc;
use gdk::prelude::*;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    ProjectItemSelected(Option<ProjectItem>),
    ActivateLink(String),
    LabelUnselect,
}

pub struct Model {
    relm: relm::Relm<ProjectPoiContents>,
    db_sender: mpsc::Sender<SqlFunc>,
    cur_project_item: Option<ProjectItem>,
    project_note_contents: Option<String>,
    pass_popover: Option<gtk::Popover>,
}

const CHILD_NAME_SERVER: &str = "server";
const CHILD_NAME_NOTE: &str = "note";

#[widget]
impl Widget for ProjectPoiContents {
    fn init_view(&mut self) {}

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        Model {
            relm: relm.clone(),
            db_sender,
            cur_project_item: None,
            project_note_contents: None,
            pass_popover: None,
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
                    // i'd initialize the popover in the init & reuse it,
                    // but i can't get the toplevel there, probably things
                    // are not fully initialized yet.
                    let popover = gtk::Popover::new(Some(
                        &self
                            .contents_stack
                            .get_toplevel()
                            .and_then(|w| w.dynamic_cast::<gtk::Window>().ok())
                            .unwrap()
                            .get_child()
                            .unwrap(),
                    ));
                    popover.set_position(gtk::PositionType::Bottom);
                    self.model.pass_popover = Some(popover.clone());
                    let display = gdk::Display::get_default().unwrap();
                    let seat = display.get_default_seat().unwrap();
                    let mouse_device = seat.get_pointer().unwrap();
                    let window = display.get_default_group();
                    let (_, dev_x, dev_y, _) = window.get_device_position(&mouse_device);
                    let (_, o_x, o_y) = self.contents_stack.get_window().unwrap().get_origin();
                    let (x, y) = (dev_x - o_x, dev_y - o_y);
                    popover.set_pointing_to(&gtk::Rectangle {
                        x,
                        y,
                        width: 50,
                        height: 15,
                    });
                    let rlm = self.model.relm.clone();
                    popover.connect_closed(move |_| {
                        // this is a workaround, without that if you close
                        // the popover by clicking on the label, the label's
                        // text gets selected (select all)
                        rlm.stream().emit(Msg::LabelUnselect);
                    });
                    popover.popup();
                    // popover.grab_focus();

                    // then display the popover 'copy' and 'reveal'
                    // reveal presumably shows & hides a gtk infobar
                    // https://stackoverflow.com/questions/52101062/vala-hide-gtk-infobar-after-a-few-seconds
                    println!("activate pass {}", &uri[7..]);
                }
            }
            Msg::LabelUnselect => self.note_label.select_region(0, 0),
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
                #[name="note_label"]
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
