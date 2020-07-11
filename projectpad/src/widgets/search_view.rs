use crate::icons::*;
use crate::sql_thread::SqlFunc;
use diesel::prelude::*;
use gdk::prelude::GdkContextExt;
use gtk::prelude::*;
use projectpadsql::models::{
    Project, ProjectNote, ProjectPointOfInterest, Server, ServerDatabase, ServerExtraUserAccount,
    ServerLink, ServerNote, ServerPointOfInterest, ServerWebsite,
};
use relm::{DrawHandler, Widget};
use relm_derive::{widget, Msg};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::mpsc;

const SEARCH_RESULT_WIDGET_HEIGHT: i32 = 120;
const SCROLLBAR_WHEEL_DY: f64 = 20.0;
const PROJECT_ICON_SIZE: i32 = 64;
const ACTION_ICON_SIZE: i32 = 24;

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

#[derive(Msg)]
pub enum Msg {
    FilterChanged(Option<String>),
    GotSearchResult(SearchResult),
    MouseScroll(gdk::ScrollDirection, (f64, f64)),
    ScrollChanged,
}

#[derive(Clone, Debug)]
enum ProjectPadItem {
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

pub struct Model {
    db_sender: mpsc::Sender<SqlFunc>,
    filter: Option<String>,
    sender: relm::Sender<SearchResult>,
    // as of 2020-07-08 "the drawing module of relm is not ready" -- have to RefCell
    search_items: Rc<RefCell<Vec<ProjectPadItem>>>,
    draw_handler: DrawHandler<gtk::DrawingArea>, // <--- TODO drop me
}

#[widget]
impl Widget for SearchView {
    fn init_view(&mut self) {
        self.model.draw_handler.init(&self.search_result_area);
        self.search_result_area
            .set_events(gdk::EventMask::ALL_EVENTS_MASK);
        let si = self.model.search_items.clone();
        let search_scroll = self.search_scroll.clone();
        let search_result_area = self.search_result_area.clone();
        self.search_result_area.connect_draw(move |_, context| {
            let search_items = si.borrow();
            // https://gtk-rs.org/docs/gtk/trait.WidgetExt.html#tymethod.connect_draw
            let y_to_display = search_scroll.get_value() as i32;
            println!(
                "draw {} {}",
                y_to_display,
                search_scroll.get_value() / search_scroll.get_adjustment().get_upper()
            );
            println!("items: {}", search_items.len());
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
                Self::draw_child(
                    // not all styles are born equal. if i want gtk::render_frame() to work,
                    // i must give the style of a button (at least the styles of a drawingarea
                    // or a scrollbar don't work)
                    &search_result_area.get_style_context(),
                    &search_items[item_idx],
                    y - y_to_display,
                    context,
                    &search_result_area,
                );
                y += SEARCH_RESULT_WIDGET_HEIGHT;
                item_idx += 1;
            }
            search_result_area
                .get_style_context()
                .remove_class("search_result_frame");
            Inhibit(false)
        });
    }

    fn draw_child(
        style_context: &gtk::StyleContext,
        item: &ProjectPadItem,
        y: i32,
        context: &cairo::Context,
        search_result_area: &gtk::DrawingArea,
    ) {
        let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
        gtk::render_background(
            style_context,
            context,
            padding.left.into(),
            y as f64 + padding.top as f64,
            search_result_area.get_allocation().width as f64
                - padding.left as f64
                - padding.right as f64,
            SEARCH_RESULT_WIDGET_HEIGHT as f64 - padding.top as f64 - padding.bottom as f64,
        );
        // https://github.com/GNOME/gtk/blob/ca71340c6bfa10092c756e5fdd5e41230e2981b5/gtk/theme/Adwaita/gtk-contained.css#L1599
        // use the system theme's frame class
        style_context.add_class(&gtk::STYLE_CLASS_FRAME);
        gtk::render_frame(
            style_context,
            context,
            padding.left.into(),
            y as f64 + padding.top as f64,
            search_result_area.get_allocation().width as f64
                - padding.left as f64
                - padding.right as f64,
            SEARCH_RESULT_WIDGET_HEIGHT as f64 - padding.top as f64 - padding.bottom as f64,
        );
        style_context.remove_class(&gtk::STYLE_CLASS_FRAME);
        match item {
            ProjectPadItem::Project(p) => Self::draw_project(
                style_context,
                context,
                search_result_area,
                padding.left.into(),
                y as f64 + padding.top as f64,
                &p,
            ),
            ProjectPadItem::Server(s) => Self::draw_server(
                style_context,
                context,
                search_result_area,
                padding.left.into(),
                y as f64 + padding.top as f64,
                &s,
            ),
            _ => {}
        }
    }

    fn draw_project(
        style_context: &gtk::StyleContext,
        context: &cairo::Context,
        search_result_area: &gtk::DrawingArea,
        x: f64,
        y: f64,
        project: &Project,
    ) {
        let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
        let title_extents = Self::draw_title(
            style_context,
            context,
            &padding,
            search_result_area,
            &project.name,
            x,
            y,
        );

        if let Some(icon) = &project.icon {
            if icon.len() > 0 {
                let translate_x = padding.left as f64;
                let translate_y = y + (title_extents.height / 1024) as f64 + 5.0;
                context.translate(translate_x, translate_y);
                super::project_badge::ProjectBadge::draw_icon(context, PROJECT_ICON_SIZE, &icon);
                context.translate(-translate_x, -translate_y);
            }
        }
        Self::draw_actions_icon(
            style_context,
            context,
            search_result_area.get_allocation().width as f64,
            y + padding.top as f64,
        );
    }

    fn draw_server(
        style_context: &gtk::StyleContext,
        context: &cairo::Context,
        search_result_area: &gtk::DrawingArea,
        x: f64,
        y: f64,
        server: &Server,
    ) {
        let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
        Self::draw_title(
            style_context,
            context,
            &padding,
            search_result_area,
            &server.desc,
            x,
            y,
        );
        Self::draw_actions_icon(
            style_context,
            context,
            search_result_area.get_allocation().width as f64,
            y + padding.top as f64,
        );
    }

    fn draw_title(
        style_context: &gtk::StyleContext,
        context: &cairo::Context,
        padding: &gtk::Border,
        search_result_area: &gtk::DrawingArea,
        text: &str,
        x: f64,
        y: f64,
    ) -> pango::Rectangle {
        style_context.add_class("search_result_item_title");
        let pango_context = search_result_area
            .get_pango_context()
            .expect("failed getting pango context");
        let layout = pango::Layout::new(&pango_context);
        layout.set_text(text);
        layout.set_ellipsize(pango::EllipsizeMode::End);
        layout.set_width(350 * 1024);
        gtk::render_layout(
            style_context,
            context,
            x + padding.left as f64,
            y + padding.top as f64,
            &layout,
        );
        style_context.remove_class("search_result_item_title");

        layout.get_extents().0
    }

    fn draw_actions_icon(
        style_context: &gtk::StyleContext,
        context: &cairo::Context,
        parent_width: f64,
        y: f64,
    ) {
        // we know we use symbolic (single color) icons.
        // i want to paint them in the theme's foreground color
        // (important for dark themes).
        // the way that I found is to paint a mask.

        // 1. load the icon as a pixbuf...
        let pixbuf = gtk::IconTheme::get_default()
            .expect("get icon theme")
            .load_icon(
                Icon::COG.name(),
                ACTION_ICON_SIZE,
                gtk::IconLookupFlags::FORCE_SYMBOLIC,
            )
            .expect("load icon1")
            .expect("load icon2");

        // 2. create a cairo surface, paint the pixbuf on it...
        let surf =
            cairo::ImageSurface::create(cairo::Format::ARgb32, ACTION_ICON_SIZE, ACTION_ICON_SIZE)
                .expect("ImageSurface");
        let surf_context = cairo::Context::new(&surf);
        surf_context.set_source_pixbuf(&pixbuf, 0.0, 0.0);
        surf_context.paint();

        // 3. set the foreground color of our context to the theme's fg color
        let fore_color = style_context.get_color(gtk::StateFlags::NORMAL);
        context.set_source_rgba(
            fore_color.red,
            fore_color.green,
            fore_color.blue,
            fore_color.alpha,
        );

        // 4. use the surface we created with the icon as a mask
        // (the alpha channel of the surface is mixed with the context
        // color to paint)
        context.mask_surface(&surf, parent_width - 50.0, y);
    }

    fn model(relm: &relm::Relm<Self>, db_sender: mpsc::Sender<SqlFunc>) -> Model {
        let stream = relm.stream().clone();
        let (channel, sender) = relm::Channel::new(move |search_r: SearchResult| {
            stream.emit(Msg::GotSearchResult(search_r));
        });
        Model {
            draw_handler: DrawHandler::new().expect("draw handler"),
            filter: None,
            db_sender,
            sender,
            search_items: Rc::new(RefCell::new(vec![])),
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
        }
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
