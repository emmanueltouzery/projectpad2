// for a summary as to how I came to that approach of using a
// DrawingArea to render the search results, you can view this
// discussion:
// https://discourse.gnome.org/t/lazy-scrollable-list/3774

use super::project_items_list::ProjectItem;
use super::project_poi_header;
use super::server_item_list_item;
use super::server_poi_contents::ServerItem;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gdk::prelude::*;
use gtk::prelude::*;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase, ServerExtraUserAccount,
    ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};
use relm::Widget;
use relm_derive::{widget, Msg};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::mpsc;

pub const SEARCH_RESULT_WIDGET_HEIGHT: i32 = 75;
const SCROLLBAR_WHEEL_DY: f64 = 20.0;

pub struct SearchResult {
    pub projects: Vec<Project>,
    pub project_notes: Vec<ProjectNote>,
    pub project_pois: Vec<ProjectPointOfInterest>,
    pub server_links: Vec<ServerLink>,
    pub servers: Vec<Server>,
    pub server_databases: Vec<ServerDatabase>,
    pub server_extra_users: Vec<ServerExtraUserAccount>,
    pub server_notes: Vec<ServerNote>,
    pub server_pois: Vec<ServerPointOfInterest>,
    pub server_websites: Vec<ServerWebsite>,
}

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
        gtk::Rectangle {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }
}

#[derive(Msg)]
pub enum Msg {
    FilterChanged(Option<String>),
    GotSearchResult(SearchResult),
    MouseScroll(gdk::ScrollDirection, (f64, f64)),
    ScrollChanged,
    CopyClicked(String),
    OpenItem(ProjectPadItem),
    OpenItemFull((Project, Option<ProjectItem>, Option<ServerItem>)),
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
    sender: relm::Sender<SearchResult>,
    // as of 2020-07-08 "the drawing module of relm is not ready" -- have to RefCell
    search_items: Rc<RefCell<Vec<ProjectPadItem>>>,
    links: Rc<RefCell<Vec<(Area, String)>>>,
    action_buttons: Rc<RefCell<Vec<(Area, ProjectPadItem)>>>,
    item_with_depressed_action: Rc<RefCell<Option<ProjectPadItem>>>,
    action_popover: Option<gtk::Popover>,
}

#[widget]
impl Widget for SearchView {
    fn init_view(&mut self) {
        self.model.action_popover = Some(
            gtk::PopoverBuilder::new()
                .relative_to(&self.search_result_area)
                .position(gtk::PositionType::Bottom)
                .build(),
        );
        let search_result_area_popdown = self.search_result_area.clone();
        let item_with_depressed_popdown = self.model.item_with_depressed_action.clone();
        self.model
            .action_popover
            .as_ref()
            .unwrap()
            .connect_closed(move |_| {
                item_with_depressed_popdown.borrow_mut().take();
                search_result_area_popdown.queue_draw();
            });
        self.search_result_area
            .set_events(gdk::EventMask::ALL_EVENTS_MASK);
        let si = self.model.search_items.clone();
        let search_scroll = self.search_scroll.clone();
        let links = self.model.links.clone();
        let action_buttons = self.model.action_buttons.clone();
        let search_result_area = self.search_result_area.clone();
        let item_with_depressed = self.model.item_with_depressed_action.clone();
        self.search_result_area.connect_draw(move |_, context| {
            Self::draw_search_view(
                context,
                &links,
                &action_buttons,
                &si,
                &search_result_area,
                &search_scroll,
                &item_with_depressed.borrow(),
            );
            Inhibit(false)
        });
        let links_mmove = self.model.links.clone();
        let search_result_area_mmove = self.search_result_area.clone();
        let hand_cursor = gdk::Cursor::new_for_display(
            &self.search_result_area.get_display().unwrap(),
            gdk::CursorType::Hand2,
        );
        self.search_result_area
            .connect_motion_notify_event(move |_, event_motion| {
                let x = event_motion.get_position().0 as i32;
                let y = event_motion.get_position().1 as i32;
                let links = links_mmove.borrow();
                search_result_area_mmove
                    .get_parent_window()
                    .unwrap()
                    .set_cursor(
                        links
                            .iter()
                            .find(|l| l.0.contains(x, y))
                            .and_then(|_| Some(&hand_cursor)),
                    );
                Inhibit(false)
            });
        let links_btnclick = self.model.links.clone();
        let action_buttons_btnclick = self.model.action_buttons.clone();
        let search_result_area_btnclick = self.search_result_area.clone();
        let popover = self.model.action_popover.as_ref().unwrap().clone();
        let item_with_depressed_btnclick = self.model.item_with_depressed_action.clone();
        let relm = self.model.relm.clone();
        self.search_result_area
            .connect_button_release_event(move |_, event_click| {
                let x = event_click.get_position().0 as i32;
                let y = event_click.get_position().1 as i32;
                let window = search_result_area_btnclick
                    .get_toplevel()
                    .and_then(|w| w.downcast::<gtk::Window>().ok());
                let links = links_btnclick.borrow();
                let action_buttons = action_buttons_btnclick.borrow();
                if let Some(link) = links.iter().find(|l| l.0.contains(x, y)) {
                    if let Result::Err(err) =
                        gtk::show_uri_on_window(window.as_ref(), &link.1, event_click.get_time())
                    {
                        eprintln!("Error opening the link: {}", err);
                    }
                } else if let Some(btn) = action_buttons.iter().find(|b| b.0.contains(x, y)) {
                    println!("button click!!");
                    item_with_depressed_btnclick
                        .borrow_mut()
                        .replace(btn.1.clone());

                    Self::fill_popover(&relm, &popover, &btn.1);
                    popover.set_pointing_to(&btn.0.to_rect());
                    popover.popup();
                }
                Inhibit(false)
            });
    }

    fn fill_popover(
        relm: &relm::Relm<SearchView>,
        popover: &gtk::Popover,
        projectpad_item: &ProjectPadItem,
    ) {
        let grid_items = if let Some(server_item) = projectpad_item.to_server_item() {
            server_item_list_item::get_server_item_grid_items(&server_item)
        } else if let Some(project_item) = projectpad_item.to_project_item() {
            project_poi_header::get_project_item_fields(&project_item)
        } else {
            vec![]
        };
        let open_btn = gtk::ModelButtonBuilder::new().label("Open").build();
        let ppitem = projectpad_item.clone();
        relm::connect!(
            relm,
            open_btn,
            connect_clicked(_),
            Msg::OpenItem(ppitem.clone())
        );
        let edit_btn = gtk::ModelButtonBuilder::new().label("Edit").build();
        project_poi_header::populate_popover(
            popover,
            &vec![open_btn, edit_btn],
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

    fn draw_search_view(
        context: &cairo::Context,
        links: &Rc<RefCell<Vec<(Area, String)>>>,
        action_buttons: &Rc<RefCell<Vec<(Area, ProjectPadItem)>>>,
        si: &Rc<RefCell<Vec<ProjectPadItem>>>,
        search_result_area: &gtk::DrawingArea,
        search_scroll: &gtk::Scrollbar,
        item_with_depressed_action: &Option<ProjectPadItem>,
    ) {
        let mut links = links.borrow_mut();
        links.clear();
        let mut action_buttons = action_buttons.borrow_mut();
        action_buttons.clear();
        let search_items = si.borrow();
        // https://gtk-rs.org/docs/gtk/trait.WidgetExt.html#tymethod.connect_draw
        let y_to_display = search_scroll.get_value() as i32;
        gtk::render_background(
            &search_result_area.get_style_context(),
            context,
            0.0,
            0.0,
            search_result_area.get_allocation().width.into(),
            search_result_area.get_allocation().height.into(),
        );
        let mut y = 0;
        let mut item_idx = 0;
        while y + SEARCH_RESULT_WIDGET_HEIGHT < y_to_display {
            y += SEARCH_RESULT_WIDGET_HEIGHT;
            item_idx += 1;
        }
        search_result_area
            .get_style_context()
            .add_class("search_result_frame");
        while item_idx < search_items.len()
            && y < y_to_display + search_result_area.get_allocation().height
        {
            super::search_view_render::draw_child(
                &search_result_area.get_style_context(),
                &search_items[item_idx],
                y - y_to_display,
                context,
                &search_result_area,
                &mut links,
                &mut action_buttons,
                item_with_depressed_action,
            );
            y += SEARCH_RESULT_WIDGET_HEIGHT;
            item_idx += 1;
        }
        search_result_area
            .get_style_context()
            .remove_class("search_result_frame");
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |search_r: SearchResult| {
            stream.emit(Msg::GotSearchResult(search_r));
        });
        Model {
            relm: relm.clone(),
            filter: None,
            db_sender,
            sender,
            search_items: Rc::new(RefCell::new(vec![])),
            links: Rc::new(RefCell::new(vec![])),
            action_buttons: Rc::new(RefCell::new(vec![])),
            action_popover: None,
            item_with_depressed_action: Rc::new(RefCell::new(None)),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::FilterChanged(filter) => {
                self.model.filter = filter;
                self.fetch_search_results();
            }
            Msg::GotSearchResult(search_result) => {
                self.refresh_display(Some(&search_result));
            }
            Msg::MouseScroll(direction, (dx, dy)) => {
                let old_val = self.search_scroll.get_value();
                let new_val = old_val
                    + if direction == gdk::ScrollDirection::Up || dy < 0.0 {
                        -SCROLLBAR_WHEEL_DY
                    } else {
                        SCROLLBAR_WHEEL_DY
                    };
                println!(
                    "scroll!! {} {} {} {} {}",
                    direction, old_val, new_val, dx, dy
                );
                self.search_scroll.set_value(new_val);
            }
            Msg::ScrollChanged => self.search_result_area.queue_draw(),
            Msg::CopyClicked(val) => {
                if let Some(clip) = self
                    .search_result_area
                    .get_display()
                    .as_ref()
                    .and_then(gtk::Clipboard::get_default)
                {
                    clip.set_text(&val);
                }
            }
            Msg::OpenItem(item) => {
                self.emit_open_item_full(item);
            }
            Msg::OpenItemFull(item) => {
                // meant for my parent
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
                Some(ProjectItem::Server(s.clone())),
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
        self.model.relm.stream().emit(Msg::OpenItemFull(data));
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
        println!("adjustment upper is {}", upper);
        self.search_scroll.set_adjustment(&gtk::Adjustment::new(
            0.0,
            0.0,
            upper as f64,
            10.0,
            60.0,
            self.search_result_area.get_allocation().height as f64,
        ));
        self.search_result_area.queue_draw();
    }

    fn fetch_search_results(&self) {
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
                })
                .unwrap(),
            Some(filter) => {
                let s = self.model.sender.clone();
                let f = format!("%{}%", filter.replace('%', "\\%"));
                self.model
                    .db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        // find all the leaves...
                        let servers = Self::filter_servers(sql_conn, &f);
                        let prjs = Self::filter_projects(sql_conn, &f);
                        let project_pois = Self::filter_project_pois(sql_conn, &f);
                        let project_notes = Self::filter_project_notes(sql_conn, &f);
                        let server_notes = Self::filter_server_notes(sql_conn, &f);
                        let server_links = Self::filter_server_links(sql_conn, &f);
                        let server_pois = Self::filter_server_pois(sql_conn, &f);
                        let server_databases = Self::filter_server_databases(sql_conn, &f);
                        let server_extra_users = Self::filter_server_extra_users(sql_conn, &f);
                        let server_websites = Self::filter_server_websites(sql_conn, &f)
                            .into_iter()
                            .map(|p| p.0)
                            .collect::<Vec<_>>();

                        // bubble up to the toplevel...
                        let mut all_server_ids =
                            servers.iter().map(|s| s.id).collect::<HashSet<_>>();
                        all_server_ids.extend(server_websites.iter().map(|sw| sw.server_id));
                        all_server_ids.extend(server_notes.iter().map(|sn| sn.server_id));
                        all_server_ids.extend(server_links.iter().map(|sl| sl.linked_server_id));
                        all_server_ids.extend(server_extra_users.iter().map(|sl| sl.server_id));
                        all_server_ids.extend(server_pois.iter().map(|sl| sl.server_id));
                        all_server_ids.extend(server_databases.iter().map(|sl| sl.server_id));
                        let all_servers = Self::load_servers_by_id(sql_conn, &all_server_ids);

                        let mut all_project_ids = all_servers
                            .iter()
                            .map(|s| s.project_id)
                            .collect::<HashSet<_>>();
                        all_project_ids.extend(prjs.iter().map(|p| p.id));
                        all_project_ids.extend(project_pois.iter().map(|ppoi| ppoi.project_id));
                        all_project_ids.extend(project_notes.iter().map(|pn| pn.project_id));
                        let all_projects = Self::load_projects_by_id(sql_conn, &all_project_ids);
                        s.send(SearchResult {
                            projects: all_projects,
                            project_notes,
                            project_pois,
                            servers: all_servers,
                            server_notes,
                            server_links,
                            server_pois,
                            server_databases,
                            server_extra_users,
                            server_websites,
                        })
                        .unwrap();
                    }))
                    .unwrap();
            }
        }
    }

    fn load_projects_by_id(db_conn: &SqliteConnection, ids: &HashSet<i32>) -> Vec<Project> {
        use projectpadsql::schema::project::dsl::*;
        project
            .filter(id.eq_any(ids))
            .order(name.asc())
            .load::<Project>(db_conn)
            .unwrap()
    }

    fn load_servers_by_id(db_conn: &SqliteConnection, ids: &HashSet<i32>) -> Vec<Server> {
        use projectpadsql::schema::server::dsl::*;
        server
            .filter(id.eq_any(ids))
            .load::<Server>(db_conn)
            .unwrap()
    }

    fn filter_projects(db_conn: &SqliteConnection, filter: &str) -> Vec<Project> {
        use projectpadsql::schema::project::dsl::*;
        project
            .filter(name.like(filter).escape('\\'))
            .load::<Project>(db_conn)
            .unwrap()
    }

    fn filter_project_pois(
        db_conn: &SqliteConnection,
        filter: &str,
    ) -> Vec<ProjectPointOfInterest> {
        use projectpadsql::schema::project_point_of_interest::dsl::*;
        project_point_of_interest
            .filter(
                desc.like(filter)
                    .escape('\\')
                    .or(text.like(filter).escape('\\'))
                    .or(path.like(filter).escape('\\')),
            )
            .load::<ProjectPointOfInterest>(db_conn)
            .unwrap()
    }

    fn filter_project_notes(db_conn: &SqliteConnection, filter: &str) -> Vec<ProjectNote> {
        use projectpadsql::schema::project_note::dsl::*;
        project_note
            .filter(
                title
                    .like(filter)
                    .escape('\\')
                    .or(contents.like(filter).escape('\\')),
            )
            .load::<ProjectNote>(db_conn)
            .unwrap()
    }

    fn filter_server_notes(db_conn: &SqliteConnection, filter: &str) -> Vec<ServerNote> {
        use projectpadsql::schema::server_note::dsl::*;
        server_note
            .filter(
                title
                    .like(filter)
                    .escape('\\')
                    .or(contents.like(filter).escape('\\')),
            )
            .load::<ServerNote>(db_conn)
            .unwrap()
    }

    fn filter_server_links(db_conn: &SqliteConnection, filter: &str) -> Vec<ServerLink> {
        use projectpadsql::schema::server_link::dsl::*;
        server_link
            .filter(desc.like(filter).escape('\\'))
            .load::<ServerLink>(db_conn)
            .unwrap()
    }

    fn filter_server_extra_users(
        db_conn: &SqliteConnection,
        filter: &str,
    ) -> Vec<ServerExtraUserAccount> {
        use projectpadsql::schema::server_extra_user_account::dsl::*;
        server_extra_user_account
            .filter(desc.like(filter).escape('\\'))
            .load::<ServerExtraUserAccount>(db_conn)
            .unwrap()
    }

    fn filter_server_pois(db_conn: &SqliteConnection, filter: &str) -> Vec<ServerPointOfInterest> {
        use projectpadsql::schema::server_point_of_interest::dsl::*;
        server_point_of_interest
            .filter(
                desc.like(filter)
                    .escape('\\')
                    .or(path.like(filter).escape('\\'))
                    .or(text.like(filter).escape('\\')),
            )
            .load::<ServerPointOfInterest>(db_conn)
            .unwrap()
    }

    fn filter_server_databases(db_conn: &SqliteConnection, filter: &str) -> Vec<ServerDatabase> {
        use projectpadsql::schema::server_database::dsl::*;
        server_database
            .filter(
                desc.like(filter)
                    .escape('\\')
                    .or(name.like(filter).escape('\\'))
                    .or(text.like(filter).escape('\\')),
            )
            .load::<ServerDatabase>(db_conn)
            .unwrap()
    }

    fn filter_servers(db_conn: &SqliteConnection, filter: &str) -> Vec<Server> {
        use projectpadsql::schema::server::dsl::*;
        server
            .filter(
                desc.like(filter)
                    .escape('\\')
                    .or(ip.like(filter).escape('\\'))
                    .or(text.like(filter).escape('\\')),
            )
            .load::<Server>(db_conn)
            .unwrap()
    }

    fn filter_server_websites(
        db_conn: &SqliteConnection,
        filter: &str,
    ) -> Vec<(ServerWebsite, Option<ServerDatabase>)> {
        use projectpadsql::schema::server_database::dsl as db;
        use projectpadsql::schema::server_website::dsl::*;
        server_website
            .left_outer_join(db::server_database)
            .filter(
                desc.like(filter)
                    .escape('\\')
                    .or(url.like(filter).escape('\\'))
                    .or(text.like(filter).escape('\\'))
                    .or(db::desc.like(filter).escape('\\'))
                    .or(db::name.like(filter).escape('\\')),
            )
            .load::<(ServerWebsite, Option<ServerDatabase>)>(db_conn)
            .unwrap()
    }

    view! {
        gtk::Box {
            #[name="search_result_area"]
            gtk::DrawingArea {
                child: {
                    expand: true
                },
                scroll_event(_, event) => (Msg::MouseScroll(event.get_direction(), event.get_delta()), Inhibit(false)),
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
