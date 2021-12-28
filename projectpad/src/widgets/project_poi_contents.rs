use super::project_items_list::ProjectItem;
use super::search_bar;
use super::search_bar::Msg as SearchBarMsg;
use super::search_bar::SearchBar;
use super::server_poi_contents;
use super::server_poi_contents::Msg as ServerPoiContentsMsg;
use super::server_poi_contents::Msg::RequestDisplayServerItem as ServerPoiContentsRequestDisplayServerItem;
use super::server_poi_contents::Msg::ShowInfoBar as ServerPoiContentsShowInfoBar;
use super::server_poi_contents::Msg::ViewNote as ServerPoiContentsMsgViewNote;
use super::server_poi_contents::ServerItem;
use super::server_poi_contents::ServerPoiContents;
use super::wintitlebar::left_align_menu;
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
    GotHeaderBarHeight(i32),
    ProjectItemSelected(Box<Option<ProjectItem>>),
    ViewServerNote(ServerNote),
    ServerNoteBack,
    TextViewMoveCursor(f64, f64),
    TextViewEventAfter(gdk::Event),
    TextViewButtonPressEvent(gdk::EventButton),
    TextViewPopulatePopup(gtk::Widget),
    RequestDisplayServerItem(ServerItem),
    NoteSearchChange(String),
    NoteSearchPrevious,
    NoteSearchNext,
    KeyboardCtrlF,
    KeyboardCtrlN,
    KeyboardCtrlP,
    KeyboardEscape,
    ShowInfoBar(String),
    ScrollToServerItem(ServerItem),
    NoteScroll,
    NoteCopyCode,
    OpenSingleWebsiteLink,
}

pub struct Model {
    relm: relm::Relm<ProjectPoiContents>,
    db_sender: mpsc::Sender<SqlFunc>,
    headerbar_height: Option<i32>,
    cur_project_item: Option<ProjectItem>,
    pass_popover: Option<gtk::Popover>,
    note_links: Vec<ItemDataInfo>,
    note_passwords: Vec<ItemDataInfo>,
    hand_cursor: Option<gdk::Cursor>,
    text_cursor: Option<gdk::Cursor>,
    search_bar: relm::Component<SearchBar>,
    note_search_text: Option<String>,
    click_pos: Option<(f64, f64)>,
    copy_btn: gtk::Button,
    copy_btn_iter_offset: Option<i32>,
}

const CHILD_NAME_SERVER: &str = "server";
const CHILD_NAME_NOTE: &str = "note";

#[widget]
impl Widget for ProjectPoiContents {
    fn init_view(&mut self) {
        let display = self.widgets.note_textview.get_display();
        self.model.hand_cursor = gdk::Cursor::from_name(&display, "pointer");
        self.model.text_cursor = gdk::Cursor::from_name(&display, "text");
        let search_bar = &self.model.search_bar;
        relm::connect!(
            search_bar@SearchBarMsg::SearchChanged(ref s),
            self.model.relm,
            Msg::NoteSearchChange(s.clone()));
        relm::connect!(
            search_bar@SearchBarMsg::SearchNext,
            self.model.relm,
            Msg::NoteSearchNext);
        relm::connect!(
            search_bar@SearchBarMsg::SearchPrevious,
            self.model.relm,
            Msg::NoteSearchPrevious);
        let search_bar_widget = self.model.search_bar.widget();
        self.widgets
            .note_search_overlay
            .add_overlay(search_bar_widget);
        self.widgets
            .note_search_overlay
            .add_overlay(&self.model.copy_btn);
        let note_vadj = self.widgets.note_scroll.get_vadjustment().unwrap();
        relm::connect!(
            self.model.relm,
            note_vadj,
            connect_value_changed(_),
            Msg::NoteScroll
        );
        relm::connect!(
            self.model.relm,
            self.model.copy_btn,
            connect_clicked(_),
            Msg::NoteCopyCode
        );
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let copy_btn = gtk::ButtonBuilder::new()
            .label("Copy code block")
            .always_show_image(true)
            .image(&gtk::Image::from_icon_name(
                Some("edit-copy-symbolic"),
                gtk::IconSize::Menu,
            ))
            .hexpand(false)
            .vexpand(false)
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Start)
            .opacity(0.6)
            .build();
        Model {
            relm: relm.clone(),
            db_sender,
            headerbar_height: None,
            cur_project_item: None,
            pass_popover: None,
            note_links: vec![],
            note_passwords: vec![],
            hand_cursor: None,
            text_cursor: None,
            search_bar: relm::init::<SearchBar>(()).expect("searchbar init"),
            note_search_text: None,
            click_pos: None,
            copy_btn,
            copy_btn_iter_offset: None,
        }
    }

    fn is_displaying_note(&self) -> bool {
        self.widgets
            .contents_stack
            .get_visible_child_name()
            .filter(|s| s.as_str() == CHILD_NAME_NOTE)
            .is_some()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::GotHeaderBarHeight(h) => {
                self.model.headerbar_height = Some(h);
            }
            Msg::ProjectItemSelected(pi) => {
                self.model.cur_project_item = *pi;
                self.streams
                    .server_contents
                    .emit(match &self.model.cur_project_item.as_ref() {
                        Some(ProjectItem::Server(srv)) => {
                            ServerPoiContentsMsg::ServerSelected(Some(srv.clone()))
                        }
                        Some(ProjectItem::ServerLink(srv_l)) => {
                            ServerPoiContentsMsg::ServerLinkSelected(srv_l.clone())
                        }
                        _ => ServerPoiContentsMsg::ServerSelected(None),
                    });
                if let Some(ProjectItem::ProjectNote(ref note)) =
                    self.model.cur_project_item.clone()
                {
                    self.display_note(&note.contents);
                }
                self.widgets.contents_stack.set_visible_child_name(
                    match self.model.cur_project_item {
                        Some(ProjectItem::ProjectNote(_)) => CHILD_NAME_NOTE,
                        _ => CHILD_NAME_SERVER, // server is a list of items, handles None well (no items)
                    },
                );
            }
            Msg::ViewServerNote(n) => {
                self.display_note(&n.contents);
                self.widgets.server_note_title.set_text(&n.title);
                self.widgets.server_note_back.set_visible(true);
                self.widgets
                    .contents_stack
                    .set_visible_child_name(CHILD_NAME_NOTE);
            }
            Msg::ServerNoteBack => {
                // self.model.note_contents = None;
                self.widgets.server_note_title.set_text("");
                self.widgets.server_note_back.set_visible(false);
                self.widgets
                    .contents_stack
                    .set_visible_child_name(CHILD_NAME_SERVER);
            }
            Msg::TextViewMoveCursor(x, y) => {
                self.textview_move_cursor(x, y);
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
            Msg::TextViewButtonPressEvent(event) => {
                self.model.click_pos = Some(event.get_position());
            }
            Msg::TextViewPopulatePopup(widget) => {
                self.textview_populate_popup(widget);
            }
            Msg::ScrollToServerItem(si) => {
                self.streams
                    .server_contents
                    .emit(ServerPoiContentsMsg::ScrollTo(
                        server_poi_contents::ScrollTarget::ServerItem(si),
                    ));
            }
            Msg::NoteScroll => {
                self.model.copy_btn.hide();
            }
            Msg::NoteCopyCode => {
                if let Some(offset) = self.model.copy_btn_iter_offset {
                    let buffer = self.widgets.note_textview.get_buffer().unwrap();
                    let start = buffer.get_iter_at_offset(offset);
                    let tag_table = buffer.get_tag_table().unwrap();
                    let tag_code = tag_table.lookup(crate::notes::TAG_CODE).unwrap();
                    let mut end = buffer.get_iter_at_offset(offset);
                    end.forward_to_tag_toggle(Some(&tag_code));
                    if let Some(txt) = buffer.get_text(&start, &end, false) {
                        Self::copy_to_clipboard(
                            &self.model.relm,
                            &self.widgets.note_textview,
                            &txt,
                        );
                    }
                }
            }
            Msg::KeyboardCtrlF => {
                if self.is_displaying_note() {
                    self.model.search_bar.emit(search_bar::Msg::Reveal(true));
                }
            }
            Msg::KeyboardCtrlN => {
                if self.is_displaying_note() {
                    search_bar::note_search_next(
                        &self.widgets.note_textview,
                        &self.model.note_search_text,
                    );
                }
            }
            Msg::KeyboardCtrlP => {
                if self.is_displaying_note() {
                    search_bar::note_search_previous(
                        &self.widgets.note_textview,
                        &self.model.note_search_text,
                    );
                }
            }
            Msg::KeyboardEscape => {
                if self.is_displaying_note() {
                    self.model.search_bar.emit(search_bar::Msg::Reveal(false));
                }
            }
            Msg::NoteSearchChange(text) => {
                search_bar::note_search_change(&self.widgets.note_textview, &text);
                self.model.note_search_text = Some(text);
            }
            Msg::NoteSearchNext => {
                search_bar::note_search_next(
                    &self.widgets.note_textview,
                    &self.model.note_search_text,
                );
            }
            Msg::NoteSearchPrevious => {
                search_bar::note_search_previous(
                    &self.widgets.note_textview,
                    &self.model.note_search_text,
                );
            }
            Msg::OpenSingleWebsiteLink => {
                if matches!(&self.model.cur_project_item, Some(ProjectItem::Server(_))) {
                    self.streams
                        .server_contents
                        .emit(ServerPoiContentsMsg::OpenSingleWebsiteLink);
                }
            }
            // meant for my parent
            Msg::ShowInfoBar(_) => {}
            // meant for my parent
            Msg::RequestDisplayServerItem(_) => {}
        }
    }

    fn textview_move_cursor(&mut self, x: f64, y: f64) {
        let (bx, by) = self.widgets.note_textview.window_to_buffer_coords(
            gtk::TextWindowType::Widget,
            x as i32,
            y as i32,
        );
        if let Some(iter) = self.widgets.note_textview.get_iter_at_location(bx, by) {
            if Self::iter_is_link_or_password(&iter) {
                self.text_note_set_cursor(&self.model.hand_cursor);
            } else if let Some(iter) = self.widgets.note_textview.get_iter_at_location(bx, by) {
                let is_code = Self::iter_matches_tags(&iter, &[crate::notes::TAG_CODE]);
                if is_code {
                    self.textview_move_cursor_over_code(iter);
                }
            } else {
                self.text_note_set_cursor(&self.model.text_cursor);
            }
        } else {
            self.text_note_set_cursor(&self.model.text_cursor);
        }
    }

    fn textview_move_cursor_over_code(&mut self, iter: gtk::TextIter) {
        let buffer = self.widgets.note_textview.get_buffer().unwrap();
        let offset = iter.get_offset();
        let tag_table = buffer.get_tag_table().unwrap();
        let tag_code = tag_table.lookup(crate::notes::TAG_CODE).unwrap();
        let mut start = buffer.get_iter_at_offset(offset);
        start.backward_to_tag_toggle(Some(&tag_code));
        let mut end = buffer.get_iter_at_offset(offset);
        end.forward_to_tag_toggle(Some(&tag_code));
        if end.get_offset() - start.get_offset() < 40 {
            // we are only interested in larger text blocks
            self.model.copy_btn.hide();
            return;
        }
        let location = self.widgets.note_textview.get_iter_location(&start);
        let (_x, orig_y) = self.widgets.note_textview.buffer_to_window_coords(
            gtk::TextWindowType::Text,
            location.x,
            location.y,
        );

        // the button will be positioned more on the right of the textview
        let btn_x = self.widgets.note_textview.get_allocation().width
            - self.model.copy_btn.get_allocation().width
            - 10;

        // does the location where i would put the button contain code?
        let does_btn_location_covers_code = self
            .widgets
            .note_textview
            .get_iter_at_location(btn_x, location.y)
            .filter(|btn_iter| Self::iter_matches_tags(btn_iter, &[crate::notes::TAG_CODE]))
            .is_some();
        let y = if does_btn_location_covers_code {
            // yes => move the button to be a little higher
            orig_y - self.model.copy_btn.get_allocation().height
        } else {
            orig_y
        };
        if y > 0 {
            self.model.copy_btn_iter_offset = Some(start.get_offset());
            self.model.copy_btn.set_margin_top(y);
            self.model.copy_btn.set_margin_start(btn_x);
            self.model.copy_btn.show_all();
        }
    }

    fn textview_populate_popup(&mut self, widget: gtk::Widget) {
        if let Ok(menu) = widget.downcast::<gtk::Menu>() {
            if let Some((x, y)) = self.model.click_pos {
                let (bx, by) = self.widgets.note_textview.window_to_buffer_coords(
                    gtk::TextWindowType::Widget,
                    x as i32,
                    y as i32,
                );
                if let Some(iter) = self.widgets.note_textview.get_iter_at_location(bx, by) {
                    let is_code = Self::iter_matches_tags(&iter, &[crate::notes::TAG_CODE]);
                    if is_code {
                        let copy_btn = gtk::MenuItemBuilder::new().label("Copy code block").build();
                        let textview = self.widgets.note_textview.clone();
                        let relm = self.model.relm.clone();
                        let buffer = self.widgets.note_textview.get_buffer().unwrap();
                        let tag_table = buffer.get_tag_table().unwrap();
                        let tag_code = tag_table.lookup(crate::notes::TAG_CODE).unwrap();
                        copy_btn.connect_activate(move |_| {
                            let offset = iter.get_offset();
                            let mut start = buffer.get_iter_at_offset(offset);
                            start.backward_to_tag_toggle(Some(&tag_code));
                            let mut end = buffer.get_iter_at_offset(offset);
                            end.forward_to_tag_toggle(Some(&tag_code));
                            if let Some(txt) = buffer.get_text(&start, &end, false) {
                                Self::copy_to_clipboard(&relm, &textview, &txt);
                            }
                        });
                        copy_btn.show_all();
                        menu.add(&copy_btn);
                    }
                }
            }
        }
    }

    // inspired by the gtk3-demo TextView/Hypertext code
    fn text_note_event_get_position_if_click_or_tap(
        &self,
        evt: &gdk::Event,
    ) -> Option<gtk::TextIter> {
        let is_click = evt.get_event_type() == gdk::EventType::ButtonRelease
            && evt.get_button() == Some(gdk::BUTTON_PRIMARY);
        let is_tap = evt.get_event_type() == gdk::EventType::TouchEnd;
        if is_click || is_tap {
            evt.get_coords().and_then(|(x, y)| {
                let (bx, by) = self.widgets.note_textview.window_to_buffer_coords(
                    gtk::TextWindowType::Widget,
                    x as i32,
                    y as i32,
                );
                self.widgets.note_textview.get_iter_at_location(bx, by)
            })
        } else {
            None
        }
    }

    fn text_note_set_cursor(&self, cursor: &Option<gdk::Cursor>) {
        if let Some(w) =
            gtk::TextViewExt::get_window(&self.widgets.note_textview, gtk::TextWindowType::Text)
        {
            w.set_cursor(cursor.as_ref());
        }
    }

    fn iter_is_link_or_password(iter: &gtk::TextIter) -> bool {
        Self::iter_matches_tags(iter, &[crate::notes::TAG_LINK, crate::notes::TAG_PASSWORD])
    }

    fn iter_matches_tags(iter: &gtk::TextIter, tags: &[&str]) -> bool {
        iter.get_tags().iter().any(|t| {
            if let Some(prop_name) = t.get_property_name() {
                let prop_name_str = prop_name.as_str();
                tags.contains(&prop_name_str)
            } else {
                false
            }
        })
    }

    fn display_note(&mut self, note_contents: &str) {
        self.model.copy_btn.hide();
        if let Some(hadj) = self.widgets.note_scroll.get_hadjustment() {
            hadj.set_value(0.0);
        }
        if let Some(vadj) = self.widgets.note_scroll.get_vadjustment() {
            vadj.set_value(0.0);
        }
        let note_buffer_info = crate::notes::note_markdown_to_text_buffer(
            note_contents,
            &crate::notes::build_tag_table(),
        );
        self.model.note_links = note_buffer_info.links;
        self.model.note_passwords = note_buffer_info.passwords;
        self.widgets
            .note_textview
            .set_buffer(Some(&note_buffer_info.buffer));
        for anchor in &note_buffer_info.separator_anchors {
            let sep = gtk::SeparatorBuilder::new()
                .margin(15)
                .width_request(350)
                .build();
            sep.show();
            self.widgets.note_textview.add_child_at_anchor(&sep, anchor);
        }
    }

    fn password_popover(&mut self, password: &str) {
        // i'd initialize the popover in the init & reuse it,
        // but i can't get the toplevel there, probably things
        // are not fully initialized yet.
        let popover = gtk::Popover::new(Some(
            &self
                .widgets
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
        let window = self
            .widgets
            .contents_stack
            .get_toplevel()
            .unwrap()
            .get_window()
            .unwrap();
        let (_, dev_x, dev_y, _) = window.get_device_position(&mouse_device);
        popover.set_pointing_to(&gtk::Rectangle {
            x: dev_x - 40,
            y: dev_y - self.model.headerbar_height.unwrap_or(0),
            width: 50,
            height: 15,
        });
        let popover_vbox = gtk::BoxBuilder::new()
            .margin(10)
            .orientation(gtk::Orientation::Vertical)
            .build();
        let popover_copy_btn = gtk::ModelButtonBuilder::new()
            .label("Copy password")
            .build();
        let textview = self.widgets.note_textview.clone();
        let p = password.to_string();
        let r = self.model.relm.clone();
        popover_copy_btn.connect_clicked(move |_| {
            Self::copy_to_clipboard(&r, &textview, &p);
        });
        left_align_menu(&popover_copy_btn);
        popover_vbox.add(&popover_copy_btn);
        let popover_reveal_btn = gtk::ModelButtonBuilder::new()
            .label("Reveal password")
            .build();
        let p2 = password.to_string();
        let r2 = self.model.relm.clone();
        popover_reveal_btn.connect_clicked(move |_| {
            r2.stream()
                .emit(Msg::ShowInfoBar(format!("The password is: {}", p2.clone())));
        });
        left_align_menu(&popover_reveal_btn);
        popover_vbox.add(&popover_reveal_btn);
        popover_vbox.show_all();
        popover.add(&popover_vbox);
        popover.popup();

        // then 'reveal'
        // reveal presumably shows & hides a gtk infobar
        // https://stackoverflow.com/questions/52101062/vala-hide-gtk-infobar-after-a-few-seconds
    }

    fn copy_to_clipboard(relm: &relm::Relm<Self>, textview: &gtk::TextView, text: &str) {
        if let Some(clip) = gtk::Clipboard::get_default(&textview.get_display()) {
            clip.set_text(text);
            relm.stream()
                .emit(Msg::ShowInfoBar("Copied to the clipboard".to_string()));
        }
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
                ServerPoiContentsRequestDisplayServerItem(ref si) => Msg::RequestDisplayServerItem(si.clone()),
                ServerPoiContentsShowInfoBar(ref msg) => Msg::ShowInfoBar(msg.clone()),
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
                    #[style_class="server_note_title"]
                    gtk::Label {
                    }
                },
                #[name="note_search_overlay"]
                gtk::Overlay {
                    child: {
                        expand: true,
                    },
                    #[style_class="note_frame"]
                    gtk::Frame {
                        #[name="note_scroll"]
                        gtk::ScrolledWindow {
                            #[name="note_textview"]
                            gtk::TextView {
                                editable: false,
                                cursor_visible: false,
                                top_margin: 5,
                                bottom_margin: 5,
                                left_margin: 5,
                                right_margin: 5,
                                motion_notify_event(_, event) => (Msg::TextViewMoveCursor(event.get_position().0, event.get_position().1), Inhibit(false)),
                                event_after(_, event) => Msg::TextViewEventAfter(event.clone()),
                                button_press_event(_, event) => (Msg::TextViewButtonPressEvent(event.clone()), Inhibit(false)),
                                populate_popup(_, menu) => Msg::TextViewPopulatePopup(menu.clone()),
                            }
                        }
                    },
                }
            }
        }
    }
}
