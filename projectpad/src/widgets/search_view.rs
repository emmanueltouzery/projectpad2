// for a summary as to how I came to that approach of using a
// DrawingArea to render the search results, you can view this
// discussion:
// https://discourse.gnome.org/t/lazy-scrollable-list/3774

use super::dialogs::dialog_helpers;
use super::dialogs::project_add_edit_dlg::Msg as MsgProjectAddEditDialog;
use super::dialogs::project_add_edit_dlg::ProjectAddEditDialog;
use super::dialogs::project_note_add_edit_dlg;
use super::dialogs::project_note_add_edit_dlg::Msg as MsgProjectNoteAddEditDialog;
use super::dialogs::project_poi_add_edit_dlg;
use super::dialogs::project_poi_add_edit_dlg::Msg as MsgProjectPoiAddEditDialog;
use super::dialogs::server_add_edit_dlg;
use super::dialogs::server_add_edit_dlg::Msg as MsgServerAddEditDialog;
use super::dialogs::server_database_add_edit_dlg::Msg as MsgServerDbAddEditDialog;
use super::dialogs::server_extra_user_add_edit_dlg::Msg as MsgServerExtraUserAddEditDialog;
use super::dialogs::server_link_add_edit_dlg;
use super::dialogs::server_link_add_edit_dlg::Msg as MsgServerLinkAddEditDialog;
use super::dialogs::server_note_add_edit_dlg::Msg as MsgServerNoteAddEditDialog;
use super::dialogs::server_poi_add_edit_dlg::Msg as MsgServerPoiAddEditDialog;
use super::dialogs::server_website_add_edit_dlg::Msg as MsgServerWebsiteAddEditDialog;
use super::dialogs::{ProjectAddEditDialogComponent, ServerAddEditDialogComponent};
use super::project_items_list::ProjectItem;
use super::project_poi_header;
pub use super::search_engine::SearchItemsType;
use super::search_engine::{run_search_filter, search_parse, SearchResult};
use super::search_view_render;
use super::server_item_list_item;
use super::server_poi_contents::ServerItem;
use crate::sql_thread::SqlFunc;
use gdk::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerAccessType, ServerDatabase,
    ServerExtraUserAccount, ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;

pub const SEARCH_RESULT_WIDGET_HEIGHT: i32 = 75;
const SCROLLBAR_WHEEL_DY: f64 = 20.0;

pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Area {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Area {
        Area {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    fn to_rect(&self) -> gtk::Rectangle {
        gtk::Rectangle::new(self.x, self.y, self.width, self.height)
    }
}

#[derive(Msg)]
pub enum Msg {
    FilterChanged(Option<String>),
    SelectItem(Option<ProjectPadItem>),
    GotSearchResult(SearchResult),
    MouseScroll(gdk::ScrollDirection, (f64, f64)),
    ScrollChanged,
    CopyClicked(String),
    OpenItem(ProjectPadItem),
    EditItem(ProjectPadItem),
    OpenItemFull(Box<(Project, Option<ProjectItem>, Option<ServerItem>)>), // large variant size hence boxed
    SearchResultsModified,
    RequestSelectedItem,
    SelectedItem((ProjectPadItem, i32, String)),
    KeyPress(gdk::EventKey),
    KeyRelease(gdk::EventKey),
    ShowInfoBar(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectPadItem {
    Project(Project),
    ProjectNote(ProjectNote),
    ProjectPoi(ProjectPointOfInterest),
    ServerLink(ServerLink),
    Server(Server),
    ServerDatabase(ServerDatabase),
    ServerExtraUserAccount(ServerExtraUserAccount),
    ServerNote(ServerNote),
    ServerPoi(ServerPointOfInterest),
    ServerWebsite(ServerWebsite),
}

impl ProjectPadItem {
    fn to_server_item(&self) -> Option<ServerItem> {
        match self {
            Self::ServerDatabase(d) => Some(ServerItem::Database(d.clone())),
            Self::ServerWebsite(w) => Some(ServerItem::Website(w.clone())),
            Self::ServerNote(n) => Some(ServerItem::Note(n.clone())),
            Self::ServerExtraUserAccount(u) => Some(ServerItem::ExtraUserAccount(u.clone())),
            Self::ServerPoi(p) => Some(ServerItem::PointOfInterest(p.clone())),
            _ => None,
        }
    }

    fn to_project_item(&self) -> Option<ProjectItem> {
        match self {
            Self::Server(s) => Some(ProjectItem::Server(s.clone())),
            Self::ServerLink(l) => Some(ProjectItem::ServerLink(l.clone())),
            Self::ProjectNote(n) => Some(ProjectItem::ProjectNote(n.clone())),
            Self::ProjectPoi(p) => Some(ProjectItem::ProjectPointOfInterest(p.clone())),
            _ => None,
        }
    }
}

pub struct Model {
    relm: relm::Relm<SearchView>,
    db_sender: mpsc::Sender<SqlFunc>,
    filter: Option<String>,
    show_shortcuts: Rc<Cell<bool>>,
    search_item_types: SearchItemsType,
    operation_mode: OperationMode,
    sender: relm::Sender<SearchResult>,
    selected_item: Rc<RefCell<Option<ProjectPadItem>>>,
    // as of 2020-07-08 "the drawing module of relm is not ready" -- have to RefCell
    search_items: Rc<RefCell<Vec<ProjectPadItem>>>,
    links: Rc<RefCell<Vec<(Area, String)>>>,
    action_areas: Rc<RefCell<Vec<(Area, ProjectPadItem)>>>,
    item_link_areas: Rc<RefCell<Vec<(Area, ProjectPadItem)>>>,
    item_with_depressed_action: Rc<RefCell<Option<ProjectPadItem>>>,
    action_popover: Option<gtk::Popover>,
    project_add_edit_dialog: Option<(relm::Component<ProjectAddEditDialog>, gtk::Dialog)>,
    project_item_add_edit_dialog: Option<(ProjectAddEditDialogComponent, gtk::Dialog)>,
    server_item_add_edit_dialog: Option<(ServerAddEditDialogComponent, gtk::Dialog)>,
    save_btn: Option<gtk::Button>,
}

#[derive(PartialEq, Clone, Copy)]
pub enum OperationMode {
    ItemActions,
    SelectItem,
}

#[widget]
impl Widget for SearchView {
    fn init_view(&mut self) {
        self.model.action_popover = Some(
            gtk::Popover::builder()
                .relative_to(&self.widgets.search_result_area)
                .position(gtk::PositionType::Bottom)
                .build(),
        );
        let search_result_area_popdown = self.widgets.search_result_area.clone();
        let item_with_depressed_popdown = self.model.item_with_depressed_action.clone();
        self.model
            .action_popover
            .as_ref()
            .unwrap()
            .connect_closed(move |_| {
                item_with_depressed_popdown.borrow_mut().take();
                search_result_area_popdown.queue_draw();
            });
        self.widgets
            .search_result_area
            .set_events(gdk::EventMask::ALL_EVENTS_MASK);
        let si = self.model.search_items.clone();
        let sel = self.model.selected_item.clone();
        let search_scroll = self.widgets.search_scroll.clone();
        let links = self.model.links.clone();
        let action_areas = self.model.action_areas.clone();
        let item_link_areas = self.model.item_link_areas.clone();
        let search_result_area = self.widgets.search_result_area.clone();
        let item_with_depressed = self.model.item_with_depressed_action.clone();
        let show_shortcuts = self.model.show_shortcuts.clone();
        let op_mode = self.model.operation_mode;
        self.widgets
            .search_result_area
            .connect_draw(move |_, context| {
                Self::draw_search_view(
                    context,
                    &links,
                    &action_areas,
                    &item_link_areas,
                    &si,
                    &search_result_area,
                    &search_scroll,
                    &item_with_depressed.borrow(),
                    &sel,
                    op_mode,
                    show_shortcuts.get(),
                );
                Inhibit(false)
            });
        let links_mmove = self.model.links.clone();
        let item_links_mmove = self.model.item_link_areas.clone();
        let search_result_area_mmove = self.widgets.search_result_area.clone();
        let hand_cursor = gdk::Cursor::for_display(
            &self.widgets.search_result_area.display(),
            gdk::CursorType::Hand2,
        );
        self.widgets
            .search_result_area
            .connect_motion_notify_event(move |_, event_motion| {
                let x = event_motion.position().0 as i32;
                let y = event_motion.position().1 as i32;
                let links = links_mmove.borrow();
                let item_links = item_links_mmove.borrow();
                search_result_area_mmove
                    .parent_window()
                    .unwrap()
                    .set_cursor(hand_cursor.as_ref().filter(|_| {
                        links.iter().any(|l| l.0.contains(x, y))
                            || item_links.iter().any(|il| il.0.contains(x, y))
                    }));
                Inhibit(false)
            });
        let links_btnclick = self.model.links.clone();
        let action_areas_btnclick = self.model.action_areas.clone();
        let item_link_areas_btnclick = self.model.item_link_areas.clone();
        let search_result_area_btnclick = self.widgets.search_result_area.clone();
        let popover = self.model.action_popover.as_ref().unwrap().clone();
        let item_with_depressed_btnclick = self.model.item_with_depressed_action.clone();
        let relm = self.model.relm.clone();
        let search_item_types = self.model.search_item_types;
        self.widgets
            .search_result_area
            .connect_button_release_event(move |_, event_click| {
                let x = event_click.position().0 as i32;
                let y = event_click.position().1 as i32;
                let window = search_result_area_btnclick
                    .toplevel()
                    .and_then(|w| w.downcast::<gtk::Window>().ok());
                let links = links_btnclick.borrow();
                let item_links = item_link_areas_btnclick.borrow();
                let action_areas = action_areas_btnclick.borrow();
                if let Some(link) = links.iter().find(|l| l.0.contains(x, y)) {
                    if let Result::Err(err) =
                        gtk::show_uri_on_window(window.as_ref(), &link.1, event_click.time())
                    {
                        eprintln!("Error opening the link: {}", err);
                    }
                } else if op_mode == OperationMode::ItemActions {
                    if let Some(btn) = action_areas.iter().find(|b| b.0.contains(x, y)) {
                        item_with_depressed_btnclick
                            .borrow_mut()
                            .replace(btn.1.clone());

                        Self::fill_popover(&relm, &popover, &btn.1);
                        popover.set_pointing_to(&btn.0.to_rect());
                        popover.popup();
                    }
                    if let Some((_, item)) = item_links.iter().find(|il| il.0.contains(x, y)) {
                        relm.stream().emit(Msg::OpenItem(item.clone()));
                    }
                } else if op_mode == OperationMode::SelectItem {
                    if let Some(btn) = action_areas.iter().find(|b| b.0.contains(x, y)) {
                        let do_replace = match (search_item_types, &btn.1) {
                            (SearchItemsType::All, _) => true,
                            (SearchItemsType::ServersOnly, ProjectPadItem::Server(_)) => true,
                            (SearchItemsType::ServerDbsOnly, ProjectPadItem::ServerDatabase(_)) => {
                                true
                            }
                            _ => false,
                        };
                        if do_replace {
                            relm.stream().emit(Msg::SelectItem(Some(btn.1.clone())));
                        }
                    }
                }
                Inhibit(false)
            });
        self.fetch_search_results(true);
    }

    fn fill_popover(
        relm: &relm::Relm<SearchView>,
        popover: &gtk::Popover,
        projectpad_item: &ProjectPadItem,
    ) {
        let grid_items = if let Some(server_item) = projectpad_item.to_server_item() {
            // TODO could pass in db & stuff
            server_item_list_item::get_server_item_grid_items(&server_item, &None)
        } else if let Some(project_item) = projectpad_item.to_project_item() {
            project_poi_header::get_project_item_fields(&project_item, None, true)
        } else {
            vec![]
        };
        let open_btn = gtk::ModelButton::builder().label("Open").build();
        let ppitem = projectpad_item.clone();
        relm::connect!(
            relm,
            open_btn,
            connect_clicked(_),
            Msg::OpenItem(ppitem.clone())
        );
        let edit_btn = gtk::ModelButton::builder().label("Edit").build();
        let ppitem2 = projectpad_item.clone();
        relm::connect!(
            relm,
            edit_btn,
            connect_clicked(_),
            Msg::EditItem(ppitem2.clone())
        );

        project_poi_header::populate_popover(
            popover,
            &[open_btn, edit_btn],
            &grid_items,
            &move |btn: &gtk::ModelButton, str_val: String| {
                relm::connect!(
                    relm,
                    btn,
                    connect_clicked(_),
                    Msg::CopyClicked(str_val.clone())
                );
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_search_view(
        context: &cairo::Context,
        links: &Rc<RefCell<Vec<(Area, String)>>>,
        action_areas: &Rc<RefCell<Vec<(Area, ProjectPadItem)>>>,
        item_link_areas: &Rc<RefCell<Vec<(Area, ProjectPadItem)>>>,
        si: &Rc<RefCell<Vec<ProjectPadItem>>>,
        search_result_area: &gtk::DrawingArea,
        search_scroll: &gtk::Scrollbar,
        item_with_depressed_action: &Option<ProjectPadItem>,
        sel_item: &Rc<RefCell<Option<ProjectPadItem>>>,
        op_mode: OperationMode,
        show_shortcuts: bool,
    ) {
        let mut links = links.borrow_mut();
        links.clear();
        let mut action_areas = action_areas.borrow_mut();
        action_areas.clear();
        let mut item_link_areas = item_link_areas.borrow_mut();
        item_link_areas.clear();
        let search_items = si.borrow();
        // https://gtk-rs.org/docs/gtk/trait.WidgetExt.html#tymethod.connect_draw
        let y_to_display = search_scroll.value() as i32;
        gtk::render_background(
            &search_result_area.style_context(),
            context,
            0.0,
            0.0,
            search_result_area.allocation().width().into(),
            search_result_area.allocation().height().into(),
        );
        let mut y = 0;
        let mut item_idx = 0;
        let mut cur_server = None;
        while y + SEARCH_RESULT_WIDGET_HEIGHT < y_to_display {
            y += SEARCH_RESULT_WIDGET_HEIGHT;
            item_idx += 1;
            let item = &search_items[item_idx];
            if let ProjectPadItem::Server(srv) = item {
                cur_server = Some(srv);
            }
        }
        search_result_area
            .style_context()
            .add_class("search_result_frame");
        let sel_i: Option<ProjectPadItem> = sel_item.borrow().clone();
        while item_idx < search_items.len()
            && y < y_to_display + search_result_area.allocation().height()
        {
            let item = &search_items[item_idx];
            if let ProjectPadItem::Server(srv) = item {
                cur_server = Some(srv);
            }
            let drawing_context = search_view_render::DrawingContext {
                search_result_area: search_result_area.clone(),
                style_context: search_result_area.style_context().clone(),
                context: context.clone(),
            };
            let padding = drawing_context
                .style_context
                .padding(gtk::StateFlags::NORMAL);
            let mut item_context = search_view_render::ItemContext {
                is_selected: sel_i.as_ref() == Some(item),
                padding,
                y: (y - y_to_display) as f64,
                item_link_areas: &mut item_link_areas,
                links: &mut links,
                action_areas: &mut action_areas,
                item_with_depressed_action: item_with_depressed_action.clone(),
                operation_mode: op_mode,
            };
            search_view_render::draw_child(&drawing_context, &mut item_context, item, cur_server);
            if show_shortcuts && item_idx < 10 {
                super::project_badge::handle_cairo_result(
                    &super::search_view_render::draw_shortcut(
                        (item_idx + 1) % 10,
                        context,
                        search_result_area,
                        y - y_to_display,
                    ),
                )
            }
            y += SEARCH_RESULT_WIDGET_HEIGHT;
            item_idx += 1;
        }
        search_result_area
            .style_context()
            .remove_class("search_result_frame");
    }

    fn model(
        relm: &relm::Relm<Self>,
        params: (
            mpsc::Sender<SqlFunc>,
            Option<String>,
            SearchItemsType,
            OperationMode,
            Option<gtk::Button>,
            Option<ProjectPadItem>,
        ),
    ) -> Model {
        let (db_sender, filter, search_item_types, operation_mode, save_btn, selected_item) =
            params;
        let stream = relm.stream().clone();
        let (_channel, sender) = relm::Channel::new(move |search_r: SearchResult| {
            stream.emit(Msg::GotSearchResult(search_r));
        });
        if let (None, Some(btn)) = (&selected_item, &save_btn) {
            btn.set_sensitive(false);
        }
        Model {
            relm: relm.clone(),
            filter,
            search_item_types,
            show_shortcuts: Rc::new(Cell::new(false)),
            operation_mode,
            db_sender,
            sender,
            search_items: Rc::new(RefCell::new(vec![])),
            links: Rc::new(RefCell::new(vec![])),
            action_areas: Rc::new(RefCell::new(vec![])),
            item_link_areas: Rc::new(RefCell::new(vec![])),
            action_popover: None,
            item_with_depressed_action: Rc::new(RefCell::new(None)),
            project_add_edit_dialog: None,
            project_item_add_edit_dialog: None,
            server_item_add_edit_dialog: None,
            selected_item: Rc::new(RefCell::new(selected_item)),
            save_btn,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::FilterChanged(filter) => {
                self.model.filter = filter;
                self.fetch_search_results(true);
            }
            Msg::SelectItem(item) => {
                if let Some(btn) = self.model.save_btn.as_ref() {
                    btn.set_sensitive(item.is_some());
                }
                self.model.selected_item.replace(item);
                self.widgets.search_result_area.queue_draw();
            }
            Msg::GotSearchResult(search_result) => {
                self.refresh_display(Some(&search_result));
            }
            Msg::MouseScroll(direction, (_dx, dy)) => {
                let old_val = self.widgets.search_scroll.value();
                let new_val = old_val
                    + if direction == gdk::ScrollDirection::Up || dy < 0.0 {
                        -SCROLLBAR_WHEEL_DY
                    } else {
                        SCROLLBAR_WHEEL_DY
                    };
                self.widgets.search_scroll.set_value(new_val);
            }
            Msg::ScrollChanged => self.widgets.search_result_area.queue_draw(),
            Msg::CopyClicked(val) => {
                self.copy_to_clipboard(&val);
            }
            Msg::OpenItem(item) => {
                self.emit_open_item_full(item);
            }
            Msg::OpenItemFull(_item) => {
                // meant for my parent
            }
            Msg::EditItem(item) => self.edit_item(item),
            Msg::SearchResultsModified => {
                if let Some((_, dialog)) = self.model.project_add_edit_dialog.as_ref() {
                    dialog.close();
                    self.model.project_add_edit_dialog = None;
                }
                if let Some((_, dialog)) = self.model.project_item_add_edit_dialog.as_ref() {
                    dialog.close();
                    self.model.project_item_add_edit_dialog = None;
                }
                if let Some((_, dialog)) = self.model.server_item_add_edit_dialog.as_ref() {
                    dialog.close();
                    self.model.server_item_add_edit_dialog = None;
                }
                self.fetch_search_results(false);
            }
            Msg::RequestSelectedItem => {
                let item = self.model.selected_item.borrow().clone();
                match &item {
                    Some(ProjectPadItem::ServerDatabase(db)) => {
                        self.model.relm.stream().emit(Msg::SelectedItem((
                            ProjectPadItem::ServerDatabase(db.clone()),
                            db.id,
                            db.desc.clone(),
                        )))
                    }
                    Some(ProjectPadItem::Server(srv)) => {
                        self.model.relm.stream().emit(Msg::SelectedItem((
                            ProjectPadItem::Server(srv.clone()),
                            srv.id,
                            srv.desc.clone(),
                        )))
                    }
                    _ => {}
                }
            }
            Msg::KeyPress(e) => {
                self.handle_keypress(e);
            }
            Msg::KeyRelease(e) => {
                if self.model.show_shortcuts.get() {
                    self.model.show_shortcuts.set(false);
                    self.widgets.search_result_area.queue_draw();
                }
                if let Some(index) = e
                    .keyval()
                    .to_unicode()
                    .and_then(|letter| letter.to_digit(10))
                    .map(|i| if i == 0 { 9_usize } else { i as usize - 1 })
                {
                    let items = self.model.search_items.borrow();
                    if let Some(item) = items.get(index) {
                        if !(e.state() & gdk::ModifierType::CONTROL_MASK).is_empty() {
                            self.model.relm.stream().emit(Msg::OpenItem(item.clone()));
                        }
                        if !(e.state() & gdk::ModifierType::MOD1_MASK).is_empty() {
                            self.model.relm.stream().emit(Msg::EditItem(item.clone()));
                        }
                    }
                }
            }
            // meant for my parent
            Msg::SelectedItem(_) => {}
            Msg::ShowInfoBar(_) => {}
        }
    }

    fn copy_to_clipboard(&self, val: &str) {
        if let Some(clip) = gtk::Clipboard::default(&self.widgets.search_result_area.display()) {
            clip.set_text(val);
            self.model
                .relm
                .stream()
                .emit(Msg::ShowInfoBar("Copied to the clipboard".to_string()));
        }
    }

    fn handle_keypress(&self, e: gdk::EventKey) {
        let has_ctrl = !(e.state() & gdk::ModifierType::CONTROL_MASK).is_empty();
        if e.keyval() == gdk::keys::constants::Return
            || e.keyval() == gdk::keys::constants::KP_Enter
        {
            let items = self.model.search_items.borrow();
            let level1_items: Vec<_> = items
                .iter()
                .filter(|i| matches!(i, ProjectPadItem::Project(_)))
                .collect();
            let level2_items: Vec<_> = items
                .iter()
                .filter(|i| i.to_project_item().is_some())
                .collect();
            let level3_items: Vec<_> = items
                .iter()
                .filter(|i| i.to_server_item().is_some())
                .collect();
            let open = |i: &ProjectPadItem| self.model.relm.stream().emit(Msg::OpenItem(i.clone()));
            match (&level1_items[..], &level2_items[..], &level3_items[..]) {
                ([fst], [], []) => open(fst),
                ([_], [snd], []) => open(snd),
                ([_], [_], [thrd]) => open(thrd),
                _ => {}
            }
        } else if has_ctrl && e.keyval().to_unicode() == Some('e') {
            let items = self.model.search_items.borrow();
            let urls = items
                .iter()
                .filter_map(|i| match i {
                    ProjectPadItem::Server(srv)
                        if srv.access_type == ServerAccessType::SrvAccessWww
                            && !srv.ip.is_empty() =>
                    {
                        Some(srv.ip.clone())
                    }
                    ProjectPadItem::ServerWebsite(www) if !www.url.is_empty() => {
                        Some(www.url.clone())
                    }
                    _ => None,
                })
                .take(2)
                .collect::<Vec<_>>();
            // is there only one match...
            if let [url] = &urls[..] {
                if let Result::Err(e) = gtk::show_uri_on_window(None::<&gtk::Window>, url, 0) {
                    eprintln!("Error opening link: {}", e);
                }
            }
        } else if has_ctrl && e.keyval().to_unicode() == Some('y') {
            let items = self.model.search_items.borrow();
            let passwords = items
                .iter()
                .filter_map(|i| match i {
                    ProjectPadItem::Server(srv) if !srv.password.is_empty() => {
                        Some(srv.password.clone())
                    }
                    ProjectPadItem::ServerWebsite(www) if !www.password.is_empty() => {
                        Some(www.password.clone())
                    }
                    _ => None,
                })
                .take(2)
                .collect::<Vec<_>>();
            // is there only one match...
            if let [password] = &passwords[..] {
                self.copy_to_clipboard(password);
            }
        } else {
            let new_show_shortcuts = [
                gdk::keys::constants::Control_L,
                gdk::keys::constants::Control_R,
                gdk::keys::constants::Alt_L,
                gdk::keys::constants::Alt_R,
            ]
            .contains(&e.keyval());
            if new_show_shortcuts != self.model.show_shortcuts.get() {
                self.model.show_shortcuts.set(new_show_shortcuts);
                self.widgets.search_result_area.queue_draw();
            }
        }
    }

    fn edit_item(&mut self, item: ProjectPadItem) {
        match item {
            // TODO tried to reduce duplication here, but gave up
            ProjectPadItem::Server(srv) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets
                        .search_result_area
                        .clone()
                        .upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        srv.project_id,
                        Some(srv),
                    ),
                    server_add_edit_dlg::Msg::OkPressed,
                    "Server",
                );
                relm::connect!(
                    component@MsgServerAddEditDialog::ServerUpdated(_),
                    self.model.relm,
                    Msg::SearchResultsModified
                );
                self.model.project_item_add_edit_dialog = Some((
                    ProjectAddEditDialogComponent::Server(component),
                    dialog.clone(),
                ));
                dialog.show();
            }
            ProjectPadItem::ProjectPoi(prj_poi) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets
                        .search_result_area
                        .clone()
                        .upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        prj_poi.project_id,
                        Some(prj_poi),
                    ),
                    project_poi_add_edit_dlg::Msg::OkPressed,
                    "Project point of interest",
                );
                relm::connect!(
                    component@MsgProjectPoiAddEditDialog::PoiUpdated(_),
                    self.model.relm,
                    Msg::SearchResultsModified
                );
                self.model.project_item_add_edit_dialog = Some((
                    ProjectAddEditDialogComponent::ProjectPoi(component),
                    dialog.clone(),
                ));
                dialog.show();
            }
            ProjectPadItem::ProjectNote(prj_note) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets
                        .search_result_area
                        .clone()
                        .upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        prj_note.project_id,
                        Some(prj_note),
                    ),
                    project_note_add_edit_dlg::Msg::OkPressed,
                    "Project note",
                );
                relm::connect!(
                    component@MsgProjectNoteAddEditDialog::ProjectNoteUpdated(_),
                    self.model.relm,
                    Msg::SearchResultsModified
                );
                self.model.project_item_add_edit_dialog = Some((
                    ProjectAddEditDialogComponent::ProjectNote(component),
                    dialog.clone(),
                ));
                dialog.show();
            }
            ProjectPadItem::ServerLink(srv_link) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets
                        .search_result_area
                        .clone()
                        .upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        srv_link.project_id,
                        Some(srv_link),
                    ),
                    server_link_add_edit_dlg::Msg::OkPressed,
                    "Server link",
                );
                relm::connect!(
                    component@MsgServerLinkAddEditDialog::ServerLinkUpdated(_),
                    self.model.relm,
                    Msg::SearchResultsModified
                );
                self.model.project_item_add_edit_dialog = Some((
                    ProjectAddEditDialogComponent::ServerLink(component),
                    dialog.clone(),
                ));
                dialog.show();
            }
            ProjectPadItem::ServerPoi(srv_poi) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets
                        .search_result_area
                        .clone()
                        .upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        srv_poi.server_id,
                        Some(srv_poi),
                    ),
                    MsgServerPoiAddEditDialog::OkPressed,
                    "Server POI",
                );
                relm::connect!(
                    component@MsgServerPoiAddEditDialog::ServerPoiUpdated(_),
                    self.model.relm,
                    Msg::SearchResultsModified
                );
                self.model.server_item_add_edit_dialog =
                    Some((ServerAddEditDialogComponent::Poi(component), dialog.clone()));
                dialog.show();
            }
            ProjectPadItem::ServerDatabase(srv_db) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets
                        .search_result_area
                        .clone()
                        .upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        srv_db.server_id,
                        Some(srv_db),
                    ),
                    MsgServerDbAddEditDialog::OkPressed,
                    "Server Database",
                );
                relm::connect!(
                    component@MsgServerDbAddEditDialog::ServerDbUpdated(_),
                    self.model.relm,
                    Msg::SearchResultsModified
                );
                self.model.server_item_add_edit_dialog =
                    Some((ServerAddEditDialogComponent::Db(component), dialog.clone()));
                dialog.show();
            }
            ProjectPadItem::ServerExtraUserAccount(srv_usr) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets
                        .search_result_area
                        .clone()
                        .upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        srv_usr.server_id,
                        Some(srv_usr),
                    ),
                    MsgServerExtraUserAddEditDialog::OkPressed,
                    "Server Extra User",
                );
                relm::connect!(
                    component@MsgServerExtraUserAddEditDialog::ServerUserUpdated(_),
                    self.model.relm,
                    Msg::SearchResultsModified
                );
                self.model.server_item_add_edit_dialog = Some((
                    ServerAddEditDialogComponent::User(component),
                    dialog.clone(),
                ));
                dialog.show();
            }
            ProjectPadItem::ServerWebsite(srv_www) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets
                        .search_result_area
                        .clone()
                        .upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        srv_www.server_id,
                        Some(srv_www),
                    ),
                    MsgServerWebsiteAddEditDialog::OkPressed,
                    "Server Website",
                );
                relm::connect!(
                    component@MsgServerWebsiteAddEditDialog::ServerWwwUpdated(_),
                    self.model.relm,
                    Msg::SearchResultsModified
                );
                self.model.server_item_add_edit_dialog = Some((
                    ServerAddEditDialogComponent::Website(component),
                    dialog.clone(),
                ));
                dialog.show();
            }
            ProjectPadItem::ServerNote(srv_note) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets
                        .search_result_area
                        .clone()
                        .upcast::<gtk::Widget>(),
                    dialog_helpers::prepare_dialog_param(
                        self.model.db_sender.clone(),
                        srv_note.server_id,
                        Some(srv_note),
                    ),
                    MsgServerNoteAddEditDialog::OkPressed,
                    "Server Note",
                );
                relm::connect!(
                    component@MsgServerNoteAddEditDialog::ServerNoteUpdated(_),
                    self.model.relm,
                    Msg::SearchResultsModified
                );
                self.model.server_item_add_edit_dialog = Some((
                    ServerAddEditDialogComponent::Note(component),
                    dialog.clone(),
                ));
                dialog.show();
            }
            ProjectPadItem::Project(prj) => {
                let (dialog, component, _) = dialog_helpers::prepare_add_edit_item_dialog(
                    self.widgets
                        .search_result_area
                        .clone()
                        .upcast::<gtk::Widget>(),
                    (
                        self.model.db_sender.clone(),
                        Some(prj),
                        gtk::AccelGroup::new(),
                    ),
                    MsgProjectAddEditDialog::OkPressed,
                    "Project",
                );
                relm::connect!(
                    component@MsgProjectAddEditDialog::ProjectUpdated(ref _project),
                    self.model.relm,
                    Msg::SearchResultsModified
                );
                self.model.project_add_edit_dialog = Some((component, dialog.clone()));
                dialog.show();
            }
        }
    }

    fn emit_open_item_full(&self, item: ProjectPadItem) {
        let search_items = self.model.search_items.borrow();
        let project_by_id = |pid| {
            search_items
                .iter()
                .find_map(|si| match si {
                    ProjectPadItem::Project(p) if p.id == pid => Some(p),
                    _ => None,
                })
                .unwrap()
                .clone()
        };
        let server_by_id = |sid| {
            search_items
                .iter()
                .find_map(|si| match si {
                    ProjectPadItem::Server(s) if s.id == sid => Some(s),
                    _ => None,
                })
                .unwrap()
                .clone()
        };

        let data = match item {
            ProjectPadItem::Project(p) => (p, None, None),
            ProjectPadItem::Server(s) => (
                project_by_id(s.project_id),
                Some(ProjectItem::Server(s)),
                None,
            ),
            ProjectPadItem::ProjectNote(n) => (
                project_by_id(n.project_id),
                Some(ProjectItem::ProjectNote(n)),
                None,
            ),
            ProjectPadItem::ProjectPoi(p) => (
                project_by_id(p.project_id),
                Some(ProjectItem::ProjectPointOfInterest(p)),
                None,
            ),
            ProjectPadItem::ServerLink(l) => (
                project_by_id(l.project_id),
                Some(ProjectItem::ServerLink(l)),
                None,
            ),
            ProjectPadItem::ServerPoi(p) => {
                let server = server_by_id(p.server_id);
                (
                    project_by_id(server.project_id),
                    Some(ProjectItem::Server(server)),
                    Some(ServerItem::PointOfInterest(p)),
                )
            }
            ProjectPadItem::ServerWebsite(w) => {
                let server = server_by_id(w.server_id);
                (
                    project_by_id(server.project_id),
                    Some(ProjectItem::Server(server)),
                    Some(ServerItem::Website(w)),
                )
            }
            ProjectPadItem::ServerDatabase(d) => {
                let server = server_by_id(d.server_id);
                (
                    project_by_id(server.project_id),
                    Some(ProjectItem::Server(server)),
                    Some(ServerItem::Database(d)),
                )
            }
            ProjectPadItem::ServerNote(n) => {
                let server = server_by_id(n.server_id);
                (
                    project_by_id(server.project_id),
                    Some(ProjectItem::Server(server)),
                    Some(ServerItem::Note(n)),
                )
            }
            ProjectPadItem::ServerExtraUserAccount(u) => {
                let server = server_by_id(u.server_id);
                (
                    project_by_id(server.project_id),
                    Some(ProjectItem::Server(server)),
                    Some(ServerItem::ExtraUserAccount(u)),
                )
            }
        };
        self.model
            .relm
            .stream()
            .emit(Msg::OpenItemFull(Box::new(data)));
    }

    fn refresh_display(&mut self, search_result: Option<&SearchResult>) {
        // TODO consider the group_by & non-clones of the filter_lisbox branch
        let mut search_items = self.model.search_items.borrow_mut();
        search_items.clear();
        if let Some(search_result) = &search_result {
            for project in &search_result.projects {
                search_items.push(ProjectPadItem::Project(project.clone()));
                for server in search_result
                    .servers
                    .iter()
                    .filter(|s| s.project_id == project.id)
                {
                    search_items.push(ProjectPadItem::Server(server.clone()));
                    for server_website in search_result
                        .server_websites
                        .iter()
                        .filter(|sw| sw.server_id == server.id)
                    {
                        search_items.push(ProjectPadItem::ServerWebsite(server_website.clone()));
                    }
                    for server_note in search_result
                        .server_notes
                        .iter()
                        .filter(|sn| sn.server_id == server.id)
                    {
                        search_items.push(ProjectPadItem::ServerNote(server_note.clone()));
                    }
                    for server_user in search_result
                        .server_extra_users
                        .iter()
                        .filter(|su| su.server_id == server.id)
                    {
                        search_items
                            .push(ProjectPadItem::ServerExtraUserAccount(server_user.clone()));
                    }
                    for server_db in search_result
                        .server_databases
                        .iter()
                        .filter(|sd| sd.server_id == server.id)
                    {
                        search_items.push(ProjectPadItem::ServerDatabase(server_db.clone()));
                    }
                    for server_poi in search_result
                        .server_pois
                        .iter()
                        .filter(|sp| sp.server_id == server.id)
                    {
                        search_items.push(ProjectPadItem::ServerPoi(server_poi.clone()));
                    }
                }
                for server_link in search_result
                    .server_links
                    .iter()
                    .filter(|s| s.project_id == project.id)
                {
                    search_items.push(ProjectPadItem::ServerLink(server_link.clone()));
                }
                for project_note in search_result
                    .project_notes
                    .iter()
                    .filter(|s| s.project_id == project.id)
                {
                    search_items.push(ProjectPadItem::ProjectNote(project_note.clone()));
                }
                for project_poi in search_result
                    .project_pois
                    .iter()
                    .filter(|s| s.project_id == project.id)
                {
                    search_items.push(ProjectPadItem::ProjectPoi(project_poi.clone()));
                }
            }
        }
        let upper = search_items.len() as i32 * SEARCH_RESULT_WIDGET_HEIGHT;
        if search_result.map(|r| r.reset_scroll).unwrap_or(false) {
            self.widgets
                .search_scroll
                .set_adjustment(&gtk::Adjustment::new(
                    0.0,
                    0.0,
                    upper as f64,
                    10.0,
                    60.0,
                    self.widgets.search_result_area.allocation().height() as f64,
                ));
        }
        self.widgets.search_result_area.queue_draw();
    }

    fn fetch_search_results(&self, reset_scroll: bool) {
        match &self.model.filter {
            None => self
                .model
                .sender
                .send(SearchResult {
                    projects: vec![],
                    project_notes: vec![],
                    project_pois: vec![],
                    servers: vec![],
                    server_databases: vec![],
                    server_extra_users: vec![],
                    server_links: vec![],
                    server_notes: vec![],
                    server_pois: vec![],
                    server_websites: vec![],
                    reset_scroll: true,
                })
                .unwrap(),
            Some(filter) => {
                let s = self.model.sender.clone();
                let search_spec = search_parse(filter);
                let f = search_spec.search_pattern;
                let project_pattern = search_spec.project_pattern;
                let search_item_types = self.model.search_item_types;
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        s.send(run_search_filter(
                            sql_conn,
                            search_item_types,
                            &f,
                            &project_pattern,
                            reset_scroll,
                        ))
                        .unwrap();
                    }))
                    .unwrap()
            }
        }
    }

    view! {
        gtk::Box {
            #[name="search_result_area"]
            gtk::DrawingArea {
                child: {
                    expand: true
                },
                scroll_event(_, event) => (Msg::MouseScroll(event.direction(), event.delta()), Inhibit(false)),
                // motion_notify_event(_, event) => (MoveCursor(event.get_position()), Inhibit(false))
            },
            #[name="search_scroll"]
            gtk::Scrollbar {
                orientation: gtk::Orientation::Vertical,
                value_changed => Msg::ScrollChanged
            }
        },
    }
}
