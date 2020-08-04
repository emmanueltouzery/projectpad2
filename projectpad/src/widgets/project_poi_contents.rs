use super::project_items_list::ProjectItem;
use super::server_poi_contents::Msg as ServerPoiContentsMsg;
use super::server_poi_contents::Msg::ViewNote as ServerPoiContentsMsgViewNote;
use super::server_poi_contents::ServerPoiContents;
use crate::sql_thread::SqlFunc;
use gdk::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::ServerNote;
use relm::Widget;
use relm_derive::{widget, Msg};
use std::sync::mpsc;

#[derive(Msg)]
pub enum Msg {
    ProjectItemSelected(Option<ProjectItem>),
    UpdateNoteScroll(f64),
    ActivateLink(String),
    NoteLabelReset,
    ViewServerNote(ServerNote),
    ServerNoteBack,
}

pub struct Model {
    relm: relm::Relm<ProjectPoiContents>,
    note_label_adj_value: f64,
    db_sender: mpsc::Sender<SqlFunc>,
    cur_project_item: Option<ProjectItem>,
    note_contents: Option<String>,
    pass_popover: Option<gtk::Popover>,
}

const CHILD_NAME_SERVER: &str = "server";
const CHILD_NAME_NOTE: &str = "note";

#[widget]
impl Widget for ProjectPoiContents {
    fn init_view(&mut self) {
        self.server_note_title
            .get_style_context()
            .add_class("server_note_title");
        let adj = self.note_scroll.get_vadjustment().unwrap().clone();
        let relm = self.model.relm.clone();
        self.note_scroll
            .get_vadjustment()
            .unwrap()
            .connect_value_changed(move |_| {
                relm.stream().emit(Msg::UpdateNoteScroll(adj.get_value()));
            });
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        Model {
            relm: relm.clone(),
            note_label_adj_value: 0.0,
            db_sender,
            cur_project_item: None,
            note_contents: None,
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
                self.model.note_contents =
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
            Msg::UpdateNoteScroll(val) => {
                if self.model.note_label_adj_value - val > 200.0 && val < 15.0 {
                    // when you click on a password, the scrollbar is reset. I'm not sure why,
                    // it may be the gtk code trying to open the link (like http://) & failing.
                    // workarounding it for now. if there's a too large change at once and we
                    // get to the beginning of the scroll area all at once, ignore the change
                } else {
                    self.model.note_label_adj_value = val;
                }
            }
            Msg::ActivateLink(uri) => {
                if uri.starts_with("pass://") {
                    self.password_popover(&uri[7..]);
                }
            }
            Msg::NoteLabelReset => {
                self.note_label.select_region(0, 0);
                self.note_scroll
                    .get_vadjustment()
                    .unwrap()
                    .set_value(self.model.note_label_adj_value);
            }
            Msg::ViewServerNote(n) => {
                self.model.note_contents = Some(crate::notes::note_markdown_to_pango_markup(
                    n.contents.as_ref(),
                ));
                self.server_note_title.set_text(&n.title);
                self.server_note_back.set_visible(true);
                self.contents_stack.set_visible_child_name(CHILD_NAME_NOTE);
            }
            Msg::ServerNoteBack => {
                self.model.note_contents = None;
                self.server_note_title.set_text("");
                self.server_note_back.set_visible(false);
                self.contents_stack
                    .set_visible_child_name(CHILD_NAME_SERVER);
            }
        }
    }

    fn password_popover(&mut self, password: &str) {
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
        popover.set_pointing_to(&gtk::Rectangle {
            x: dev_x - o_x,
            y: dev_y - o_y,
            width: 50,
            height: 15,
        });
        let rlm = self.model.relm.clone();
        popover.connect_closed(move |_| {
            // this is a workaround, without that if you close
            // the popover by clicking on the label, the label's
            // text gets selected (select all), and the scrollbar
            // gets reset too => override both things
            rlm.stream().emit(Msg::NoteLabelReset);
        });
        let popover_vbox = gtk::BoxBuilder::new()
            .margin(10)
            .orientation(gtk::Orientation::Vertical)
            .build();
        let popover_btn = gtk::ModelButtonBuilder::new()
            .label("Copy password")
            .build();
        let lbl = self.note_label.clone();
        let p = password.to_string();
        popover_btn.connect_clicked(move |_| {
            if let Some(clip) = gtk::Clipboard::get_default(&lbl.get_display()) {
                clip.set_text(&p);
            }
        });
        popover_vbox.add(&popover_btn);
        popover_vbox.show_all();
        popover.add(&popover_vbox);
        // workaround for the label scrolling up to the top when
        // we open the popover
        self.model.relm.stream().emit(Msg::NoteLabelReset);
        popover.popup();

        // then 'reveal'
        // reveal presumably shows & hides a gtk infobar
        // https://stackoverflow.com/questions/52101062/vala-hide-gtk-infobar-after-a-few-seconds
    }

    view! {
        #[name="contents_stack"]
        gtk::Stack {
            #[name="server_contents"]
            ServerPoiContents(self.model.db_sender.clone()) {
                child: {
                    name: Some(CHILD_NAME_SERVER)
                },
                ServerPoiContentsMsgViewNote(ref n) => Msg::ViewServerNote(n.clone())
            },
            gtk::Box {
                child: {
                    name: Some(CHILD_NAME_NOTE),
                },
                orientation: gtk::Orientation::Vertical,
                #[name="server_note_back"]
                gtk::Box {
                    visible: false,
                    gtk::Button {
                        image: Some(&gtk::Image::from_icon_name(Some("go-previous-symbolic"), gtk::IconSize::Menu)),
                        button_press_event(_, _) => (Msg::ServerNoteBack, Inhibit(false)),
                    },
                    #[name="server_note_title"]
                    gtk::Label {
                    }
                },
                #[name="note_scroll"]
                gtk::ScrolledWindow {
                    child: {
                        expand: true,
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
                        markup: self.model.note_contents
                                          .as_ref().map(|c| c.as_str()).unwrap_or(""),
                        activate_link(_, uri) => (Msg::ActivateLink(uri.to_string()), Inhibit(false))
                    }
                }
            }
        }
    }
}
