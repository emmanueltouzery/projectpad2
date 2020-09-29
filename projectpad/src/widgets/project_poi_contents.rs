use super::project_items_list::ProjectItem;
use super::search_bar;
use super::search_bar::SearchBar;
use super::server_poi_contents::Msg as ServerPoiContentsMsg;
use super::server_poi_contents::Msg::RequestDisplayServerItem as ServerPoiContentsRequestDisplayServerItem;
use super::server_poi_contents::Msg::ViewNote as ServerPoiContentsMsgViewNote;
use super::server_poi_contents::ServerItem;
use super::server_poi_contents::ServerPoiContents;
use crate::notes::ItemDataInfo;
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
    ViewServerNote(ServerNote),
    ServerNoteBack,
    TextViewMoveCursor(f64, f64),
    TextViewEventAfter(gdk::Event),
    RequestDisplayServerItem(ServerItem),
    NoteKeyRelease(gdk::EventKey),
}

pub struct Model {
    relm: relm::Relm<ProjectPoiContents>,
    db_sender: mpsc::Sender<SqlFunc>,
    cur_project_item: Option<ProjectItem>,
    pass_popover: Option<gtk::Popover>,
    note_links: Vec<ItemDataInfo>,
    note_passwords: Vec<ItemDataInfo>,
    hand_cursor: Option<gdk::Cursor>,
    text_cursor: Option<gdk::Cursor>,
    search_bar: relm::Component<SearchBar>,
}

const CHILD_NAME_SERVER: &str = "server";
const CHILD_NAME_NOTE: &str = "note";

#[widget]
impl Widget for ProjectPoiContents {
    fn init_view(&mut self) {
        let display = self.note_textview.get_display();
        self.model.hand_cursor = gdk::Cursor::from_name(&display, "pointer");
        self.model.text_cursor = gdk::Cursor::from_name(&display, "text");
        self.server_note_title
            .get_style_context()
            .add_class("server_note_title");
        let search_bar_widget = self.model.search_bar.widget();
        self.note_overlay.add_overlay(search_bar_widget);
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        Model {
            relm: relm.clone(),
            db_sender,
            cur_project_item: None,
            pass_popover: None,
            note_links: vec![],
            note_passwords: vec![],
            hand_cursor: None,
            text_cursor: None,
            search_bar: relm::init::<SearchBar>(()).expect("searchbar init"),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectItemSelected(pi) => {
                self.model.cur_project_item = pi;
                self.server_contents
                    .emit(match &self.model.cur_project_item.as_ref() {
                        Some(ProjectItem::Server(srv)) => {
                            ServerPoiContentsMsg::ServerSelected(Some(srv.clone()))
                        }
                        Some(ProjectItem::ServerLink(srv_l)) => {
                            ServerPoiContentsMsg::ServerLinkSelected(srv_l.clone())
                        }
                        _ => ServerPoiContentsMsg::ServerSelected(None),
                    });
                if let Some(pi) = self.model.cur_project_item.clone() {
                    match pi {
                        ProjectItem::ProjectNote(ref note) => {
                            self.display_note(&note.contents);
                        }
                        _ => {}
                    }
                }
                self.contents_stack
                    .set_visible_child_name(match self.model.cur_project_item {
                        Some(ProjectItem::ProjectNote(_)) => CHILD_NAME_NOTE,
                        _ => CHILD_NAME_SERVER, // server is a list of items, handles None well (no items)
                    });
            }
            Msg::ViewServerNote(n) => {
                self.display_note(&n.contents);
                self.server_note_title.set_text(&n.title);
                self.server_note_back.set_visible(true);
                self.contents_stack.set_visible_child_name(CHILD_NAME_NOTE);
            }
            Msg::ServerNoteBack => {
                // self.model.note_contents = None;
                self.server_note_title.set_text("");
                self.server_note_back.set_visible(false);
                self.contents_stack
                    .set_visible_child_name(CHILD_NAME_SERVER);
            }
            Msg::TextViewMoveCursor(x, y) => {
                let (bx, by) = self.note_textview.window_to_buffer_coords(
                    gtk::TextWindowType::Widget,
                    x as i32,
                    y as i32,
                );
                if let Some(iter) = self.note_textview.get_iter_at_location(bx, by) {
                    if Self::iter_is_link_or_password(&iter) {
                        self.text_note_set_cursor(&self.model.hand_cursor);
                    } else {
                        self.text_note_set_cursor(&self.model.text_cursor);
                    }
                } else {
                    self.text_note_set_cursor(&self.model.text_cursor);
                }
            }
            Msg::TextViewEventAfter(evt) => {
                if let Some(iter) = self.text_note_event_get_position_if_click_or_tap(&evt) {
                    if Self::iter_is_link_or_password(&iter) {
                        let offset = iter.get_offset();
                        if let Some(link) = self
                            .model
                            .note_links
                            .iter()
                            .find(|l| l.start_offset <= offset && l.end_offset > offset)
                        {
                            if let Result::Err(e) = gtk::show_uri_on_window(
                                None::<&gtk::Window>,
                                &link.data,
                                evt.get_time(),
                            ) {
                                eprintln!("Error opening url in browser: {:?}", e);
                            }
                        } else if let Some(pass) = self
                            .model
                            .note_passwords
                            .iter()
                            .find(|l| l.start_offset <= offset && l.end_offset > offset)
                        {
                            let p = pass.data.clone();
                            self.password_popover(&p);
                        }
                    }
                }
            }
            Msg::NoteKeyRelease(key) => {
                if !(key.get_state() & gdk::ModifierType::CONTROL_MASK).is_empty()
                    && key.get_keyval().to_unicode() == Some('f')
                {
                    self.model.search_bar.emit(search_bar::Msg::Reveal(true));
                } else if key.get_keyval() == gdk::keys::constants::Escape {
                    self.model.search_bar.emit(search_bar::Msg::Reveal(false));
                    self.note_scroll.grab_focus();
                }
            }
            // meant for my parent
            Msg::RequestDisplayServerItem(_) => {}
        }
    }

    // inspired by the gtk3-demo TextView/Hypertext code
    fn text_note_event_get_position_if_click_or_tap(
        &self,
        evt: &gdk::Event,
    ) -> Option<gtk::TextIter> {
        let is_click =
            evt.get_event_type() == gdk::EventType::ButtonRelease && evt.get_button() == Some(1); // GDK_BUTTON_PRIMARY; https://github.com/gtk-rs/gtk/issues/1044
        let is_tap = evt.get_event_type() == gdk::EventType::TouchEnd;
        if is_click || is_tap {
            evt.get_coords().and_then(|(x, y)| {
                let (bx, by) = self.note_textview.window_to_buffer_coords(
                    gtk::TextWindowType::Widget,
                    x as i32,
                    y as i32,
                );
                self.note_textview.get_iter_at_location(bx, by)
            })
        } else {
            None
        }
    }

    fn text_note_set_cursor(&self, cursor: &Option<gdk::Cursor>) {
        gtk::TextViewExt::get_window(&self.note_textview, gtk::TextWindowType::Text)
            .unwrap()
            .set_cursor(cursor.as_ref());
    }

    fn iter_is_link_or_password(iter: &gtk::TextIter) -> bool {
        iter.get_tags()
            .iter()
            .find(|t| {
                if let Some(prop_name) = t.get_property_name() {
                    let prop_name_str = prop_name.as_str();
                    prop_name_str == crate::notes::TAG_LINK
                        || prop_name_str == crate::notes::TAG_PASSWORD
                } else {
                    false
                }
            })
            .is_some()
    }

    fn display_note(&mut self, note_contents: &str) {
        let note_buffer_info = crate::notes::note_markdown_to_text_buffer(
            note_contents.as_ref(),
            &crate::notes::build_tag_table(),
        );
        self.model.note_links = note_buffer_info.links;
        self.model.note_passwords = note_buffer_info.passwords;
        self.note_textview
            .set_buffer(Some(&note_buffer_info.buffer));
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
        let popover_vbox = gtk::BoxBuilder::new()
            .margin(10)
            .orientation(gtk::Orientation::Vertical)
            .build();
        let popover_btn = gtk::ModelButtonBuilder::new()
            .label("Copy password")
            .build();
        let textview = self.note_textview.clone();
        let p = password.to_string();
        popover_btn.connect_clicked(move |_| {
            if let Some(clip) = gtk::Clipboard::get_default(&textview.get_display()) {
                clip.set_text(&p);
            }
        });
        popover_vbox.add(&popover_btn);
        popover_vbox.show_all();
        popover.add(&popover_vbox);
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
                ServerPoiContentsMsgViewNote(ref n) => Msg::ViewServerNote(n.clone()),
                ServerPoiContentsRequestDisplayServerItem(ref si) => Msg::RequestDisplayServerItem(si.clone())
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
                #[name="note_overlay"]
                gtk::Overlay {
                    child: {
                        expand: true,
                    },
                    key_release_event(_, key) => (Msg::NoteKeyRelease(key.clone()), Inhibit(false)),
                    #[name="note_scroll"]
                    gtk::ScrolledWindow {
                        #[name="note_textview"]
                        gtk::TextView {
                            margin_top: 10,
                            margin_start: 10,
                            margin_end: 10,
                            margin_bottom: 10,
                            editable: false,
                            cursor_visible: false,
                            motion_notify_event(_, event) => (Msg::TextViewMoveCursor(event.get_position().0, event.get_position().1), Inhibit(false)),
                            event_after(_, event) => Msg::TextViewEventAfter(event.clone())
                        }
                    },
                }
            }
        }
    }
}
