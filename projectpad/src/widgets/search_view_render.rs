use super::search_view::{Area, OperationMode, ProjectPadItem, SEARCH_RESULT_WIDGET_HEIGHT};
use crate::icons::*;
use gdk::prelude::GdkContextExt;
use gtk::prelude::*;
use projectpadsql::models::{
    EnvironmentType, Project, ProjectNote, ProjectPointOfInterest, Server, ServerAccessType,
    ServerDatabase, ServerExtraUserAccount, ServerLink, ServerNote, ServerPointOfInterest,
    ServerWebsite,
};
const LEFT_RIGHT_MARGIN: i32 = 150;
const ACTION_ICON_SIZE: i32 = 16;
const PROJECT_ICON_SIZE: i32 = 56;
const ACTION_ICON_OFFSET_FROM_RIGHT: f64 = 50.0;
const KEYBOARD_SHORTCUT_HINT_LEFT_MARGIN: i32 = 20;

#[derive(PartialEq, Eq)]
enum ItemType {
    Parent,
    Child,
}

pub struct DrawingContext {
    pub style_context: gtk::StyleContext,
    pub context: cairo::Context,
    pub search_result_area: gtk::DrawingArea,
}

pub struct ItemContext<'a> {
    pub is_selected: bool,
    pub padding: gtk::Border,
    pub y: f64,
    pub item_link_areas: &'a mut Vec<(Area, ProjectPadItem)>,
    pub links: &'a mut Vec<(Area, String)>,
    pub action_areas: &'a mut Vec<(Area, ProjectPadItem)>,
    pub item_with_depressed_action: Option<ProjectPadItem>,
    pub operation_mode: OperationMode,
}

fn draw_button(
    context: &cairo::Context,
    item_type: ItemType,
    flags: gtk::StateFlags,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
    let style_context = &gtk::StyleContext::new();
    let path = gtk::WidgetPath::new();
    if item_type == ItemType::Child {
        // if it's a child, i use the button style when it's
        // in a list, which is more discrete.
        path.append_type(glib::Type::Invalid);
        path.iter_set_object_name(-3, Some("list"));
        path.append_type(glib::Type::Invalid);
        path.iter_set_object_name(-2, Some("row"));
    }
    path.append_type(glib::Type::Invalid);
    path.iter_set_object_name(-1, Some("button"));
    style_context.set_state(flags);
    style_context.set_path(&path);
    style_context.add_class(&gtk::STYLE_CLASS_BUTTON);
    style_context.add_class("image-button");
    style_context.add_class("popup");
    style_context.add_class("toggle");

    gtk::render_background(style_context, context, x, y, w, h);

    gtk::render_frame(style_context, context, x, y, w, h);
}

fn selected_label_style_context() -> gtk::StyleContext {
    let scontext = gtk::StyleContext::new();
    let path = gtk::WidgetPath::new();
    path.append_type(glib::Type::Invalid);
    path.iter_set_object_name(-2, Some("label"));
    path.append_type(glib::Type::Invalid);
    path.iter_set_object_name(-1, Some("selection"));
    scontext.set_state(gtk::StateFlags::SELECTED);
    scontext.set_path(&path);
    scontext.add_class(&gtk::STYLE_CLASS_LABEL);
    scontext
}

fn draw_box(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    hierarchy_offset: f64,
) {
    let DrawingContext {
        style_context,
        context,
        ..
    } = drawing_context;
    let ItemContext { action_areas, .. } = item_context;
    let margin = style_context.get_margin(gtk::StateFlags::NORMAL);

    let scontext: gtk::StyleContext;
    let bg_context = if item_context.is_selected {
        scontext = selected_label_style_context();
        &scontext
    } else {
        style_context
    };
    let box_x = margin.left as f64 + hierarchy_offset;
    let box_y = item_context.y + margin.top as f64;
    let box_width = drawing_context.search_result_area.get_allocation().width as f64
        - margin.left as f64
        - margin.right as f64
        - hierarchy_offset * 2.0;
    let box_height = SEARCH_RESULT_WIDGET_HEIGHT as f64 - margin.top as f64;
    gtk::render_background(bg_context, context, box_x, box_y, box_width, box_height);

    // https://github.com/GNOME/gtk/blob/ca71340c6bfa10092c756e5fdd5e41230e2981b5/gtk/theme/Adwaita/gtk-contained.css#L1599
    // use the system theme's frame class
    style_context.add_class(&gtk::STYLE_CLASS_FRAME);
    gtk::render_frame(style_context, context, box_x, box_y, box_width, box_height);

    if item_context.operation_mode == OperationMode::SelectItem {
        action_areas.push((
            Area::new(
                box_x as i32,
                box_y as i32,
                box_width as i32,
                box_height as i32,
            ),
            item.clone(),
        ));
    }
    style_context.remove_class(&gtk::STYLE_CLASS_BUTTON);
}

pub fn draw_shortcut(
    index: usize,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    y: i32,
) {
    let pango_context = search_result_area.create_pango_context();
    let layout = pango::Layout::new(&pango_context);
    layout.set_text(&format!("{}", index));
    let extents = layout.get_extents().1;
    let topleft_x = (LEFT_RIGHT_MARGIN
        - extents.width / pango::SCALE
        - KEYBOARD_SHORTCUT_HINT_LEFT_MARGIN) as f64;
    let topleft_y =
        (y + SEARCH_RESULT_WIDGET_HEIGHT / 2 - extents.height / pango::SCALE / 2) as f64;
    let circle_width = (extents.width / pango::SCALE) as f64;
    let circle_height = (extents.height / pango::SCALE) as f64;

    // this rectangle & clip is a workaround.
    // without that, i get a connecting line between this
    // circle and some items (for instance project titles or
    // links for server website). there is a better way for sure...
    context.rectangle(
        topleft_x - circle_height,
        topleft_y - circle_height,
        circle_height * 3.0,
        circle_height * 3.0,
    );
    context.clip();

    context.arc(
        topleft_x + circle_width / 2.0,
        topleft_y + circle_height / 2.0,
        f64::max(circle_width, circle_height) * 0.65,
        0.0,
        2.0 * std::f64::consts::PI,
    );
    context.close_path();
    context.set_source_rgb(0.0, 0.0, 0.0);
    context.stroke_preserve();
    context.set_source_rgb(1.0, 1.0, 1.0);
    context.fill();
    context.reset_clip();
    gtk::render_layout(
        &search_result_area.get_style_context(),
        context,
        topleft_x,
        topleft_y,
        &layout,
    );
}

pub fn draw_child(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    cur_server: Option<&Server>,
) {
    let extra_css_class = match &item {
        ProjectPadItem::Server(_)
        | ProjectPadItem::ProjectNote(_)
        | ProjectPadItem::ServerLink(_)
        | ProjectPadItem::ProjectPoi(_) => "search_view_parent",
        _ => "search_view_child",
    };
    let style_context = &drawing_context.style_context;
    style_context.add_class(extra_css_class);
    let x = item_context.padding.left as f64 + LEFT_RIGHT_MARGIN as f64;
    match &item {
        ProjectPadItem::Project(p) => draw_project(drawing_context, item_context, item, &p),
        ProjectPadItem::Server(s) => draw_server(
            drawing_context,
            item_context,
            item,
            LEFT_RIGHT_MARGIN as f64,
            &s,
        ),
        ProjectPadItem::ServerNote(n) => {
            draw_server_note(drawing_context, item_context, item, x, &n, cur_server)
        }
        ProjectPadItem::ProjectNote(n) => {
            draw_project_note(drawing_context, item_context, item, x, &n)
        }
        ProjectPadItem::ServerWebsite(w) => {
            draw_server_website(drawing_context, item_context, item, x, &w, cur_server)
        }
        ProjectPadItem::ServerExtraUserAccount(u) => {
            draw_server_extra_user(drawing_context, item_context, item, x, &u, cur_server)
        }
        ProjectPadItem::ServerPoi(p) => {
            draw_server_poi(drawing_context, item_context, item, x, &p, cur_server)
        }
        ProjectPadItem::ProjectPoi(p) => {
            draw_project_poi(drawing_context, item_context, item, x, &p)
        }
        ProjectPadItem::ServerDatabase(d) => {
            draw_server_database(drawing_context, item_context, item, x, &d, cur_server)
        }
        ProjectPadItem::ServerLink(s) => {
            draw_linked_server(drawing_context, item_context, item, x, &s)
        }
    }
    style_context.remove_class(extra_css_class);
}

fn draw_project(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    project: &Project,
) {
    let x = item_context.padding.left as f64 + LEFT_RIGHT_MARGIN as f64;
    // since the servers have 10px padding on top of them,
    // let's draw the projects at the bottom of their area
    // so, y+height-icon_size
    let title_extents = draw_title(
        drawing_context,
        item_context,
        item,
        &project.name,
        Some("search_result_project_title".to_string()),
        (
            x,
            item_context.y + SEARCH_RESULT_WIDGET_HEIGHT as f64 - PROJECT_ICON_SIZE as f64,
        ),
        Some(PROJECT_ICON_SIZE),
        // TODO used to have have false for is_selected for projects...
    );

    if let Some(icon) = &project.icon {
        if !icon.is_empty() {
            let translate_x =
                x + (title_extents.width / pango::SCALE) as f64 + item_context.padding.left as f64;
            let translate_y = item_context.y
                + item_context.padding.top as f64
                + SEARCH_RESULT_WIDGET_HEIGHT as f64
                - PROJECT_ICON_SIZE as f64;
            drawing_context.context.translate(translate_x, translate_y);
            super::project_badge::ProjectBadge::draw_icon(
                &drawing_context.context,
                PROJECT_ICON_SIZE,
                &icon,
            );
            drawing_context
                .context
                .translate(-translate_x, -translate_y);
        }
    }
}

fn draw_server_item_common(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    x: f64,
    title: &str,
    icon: &Icon,
    server: Option<&Server>,
) -> (gtk::Border, gtk::Border, pango::Rectangle) {
    let y = item_context.y;
    let style_context = &drawing_context.style_context;
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let margin = style_context.get_margin(gtk::StateFlags::NORMAL);
    let is_retired = server.map(|s| s.is_retired).unwrap_or(false);
    if is_retired {
        drawing_context
            .style_context
            .add_class("server_item_header_titlebox_retired");
    }
    draw_box(
        drawing_context,
        item_context,
        item,
        LEFT_RIGHT_MARGIN as f64,
    );
    if is_retired {
        drawing_context
            .style_context
            .remove_class("server_item_header_titlebox_retired");
    }
    draw_icon(
        &drawing_context.style_context,
        &drawing_context.context,
        icon,
        x + padding.left as f64,
        y + margin.top as f64 + padding.top as f64,
    );
    let title_rect = draw_title(
        drawing_context,
        item_context,
        item,
        title,
        None,
        (
            x + ACTION_ICON_SIZE as f64 + (padding.left / 2) as f64,
            y + margin.top as f64,
        ),
        Some(ACTION_ICON_SIZE),
    );
    if item_context.operation_mode == OperationMode::ItemActions {
        draw_action(
            drawing_context,
            item_context,
            item,
            &Icon::COG,
            y + padding.top as f64 + margin.top as f64,
        );
    }
    (padding, margin, title_rect)
}

fn draw_server_website(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    x: f64,
    website: &ServerWebsite,
    server: Option<&Server>,
) {
    let (padding, margin, title_rect) = draw_server_item_common(
        drawing_context,
        item_context,
        item,
        x,
        &website.desc,
        &Icon::HTTP,
        server,
    );
    draw_link(
        drawing_context,
        &website.url,
        x + padding.left as f64,
        item_context.y
            + margin.top as f64
            + (title_rect.height / pango::SCALE) as f64
            + padding.top as f64,
        item_context.links,
    );
}

fn draw_server_extra_user(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    x: f64,
    user: &ServerExtraUserAccount,
    server: Option<&Server>,
) {
    let (padding, margin, title_rect) = draw_server_item_common(
        drawing_context,
        item_context,
        item,
        x,
        &user.username,
        &Icon::USER,
        server,
    );

    draw_subtext(
        drawing_context,
        &user.desc,
        x + padding.left as f64,
        item_context.y
            + margin.top as f64
            + (title_rect.height / pango::SCALE) as f64
            + padding.top as f64,
    );
}

fn draw_server_poi(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    x: f64,
    poi: &ServerPointOfInterest,
    server: Option<&Server>,
) {
    let (padding, margin, title_rect) = draw_server_item_common(
        drawing_context,
        item_context,
        item,
        x,
        &poi.desc,
        &Icon::POINT_OF_INTEREST,
        server,
    );

    draw_subtext(
        drawing_context,
        &poi.text,
        x + padding.left as f64,
        item_context.y
            + margin.top as f64
            + (title_rect.height / pango::SCALE) as f64
            + padding.top as f64,
    );
}

fn draw_project_poi(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    x: f64,
    poi: &ProjectPointOfInterest,
) {
    let (padding, margin, title_rect) = draw_server_item_common(
        drawing_context,
        item_context,
        item,
        x,
        &poi.desc,
        &Icon::POINT_OF_INTEREST,
        None,
    );

    draw_subtext(
        drawing_context,
        &poi.text,
        x + padding.left as f64,
        item_context.y
            + margin.top as f64
            + (title_rect.height / pango::SCALE) as f64
            + padding.top as f64,
    );
}

fn draw_server_database(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    x: f64,
    db: &ServerDatabase,
    server: Option<&Server>,
) {
    let (padding, margin, title_rect) = draw_server_item_common(
        drawing_context,
        item_context,
        item,
        x,
        &db.desc,
        &Icon::DATABASE,
        server,
    );

    draw_subtext(
        drawing_context,
        &format!("{} {}", db.text, db.username),
        x + padding.left as f64,
        item_context.y
            + margin.top as f64
            + (title_rect.height / pango::SCALE) as f64
            + padding.top as f64,
    );
}

fn draw_linked_server(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    x: f64,
    srv: &ServerLink,
) {
    let (_padding, _margin, _title_rect) = draw_server_item_common(
        drawing_context,
        item_context,
        item,
        x,
        &srv.desc,
        &Icon::SERVER_LINK,
        None,
    );
}

fn draw_project_note(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    x: f64,
    note: &ProjectNote,
) {
    let (_padding, _margin, _title_rect) = draw_server_item_common(
        drawing_context,
        item_context,
        item,
        x,
        &note.title,
        &Icon::NOTE,
        None,
    );
}

fn draw_server_note(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    x: f64,
    note: &ServerNote,
    server: Option<&Server>,
) {
    let (_padding, _margin, _title_rect) = draw_server_item_common(
        drawing_context,
        item_context,
        item,
        x,
        &note.title,
        &Icon::NOTE,
        server,
    );
}

fn draw_server(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    hierarchy_offset: f64,
    server: &Server,
) {
    let y = item_context.y;
    let x = item_context.padding.left as f64 + LEFT_RIGHT_MARGIN as f64;
    let margin = drawing_context
        .style_context
        .get_margin(gtk::StateFlags::NORMAL);
    if server.is_retired {
        drawing_context
            .style_context
            .add_class("project_poi_header_titlebox_retired");
    }
    draw_box(drawing_context, item_context, item, hierarchy_offset);
    if server.is_retired {
        drawing_context
            .style_context
            .remove_class("project_poi_header_titlebox_retired");
    }
    drawing_context.style_context.add_class("title");
    let title_rect = draw_title(
        drawing_context,
        item_context,
        item,
        &server.desc,
        None,
        (x, item_context.y + margin.top as f64),
        None,
    );
    {
        let padding = &item_context.padding;
        drawing_context.style_context.remove_class("title");
        let env_rect = draw_environment(
            drawing_context,
            x + padding.left as f64,
            y + (title_rect.height / pango::SCALE) as f64 + padding.top as f64 + margin.top as f64,
            &match server.environment {
                EnvironmentType::EnvUat => "uat",
                EnvironmentType::EnvProd => "prod",
                EnvironmentType::EnvStage => "stg",
                EnvironmentType::EnvDevelopment => "dev",
            },
        );
        if server.access_type == ServerAccessType::SrvAccessWww && !server.ip.is_empty() {
            draw_link(
                drawing_context,
                &server.ip,
                (env_rect.x + env_rect.width) as f64,
                y + (title_rect.height / pango::SCALE) as f64 + padding.top as f64,
                item_context.links,
            );
        }
    }
    if item_context.operation_mode == OperationMode::ItemActions {
        draw_action(
            drawing_context,
            item_context,
            item,
            &Icon::COG,
            y + item_context.padding.top as f64 + margin.top as f64,
        );
    }
}

fn draw_environment(
    drawing_context: &DrawingContext,
    x: f64,
    y: f64,
    env_name: &str,
) -> gtk::Rectangle {
    let context = &drawing_context.context;
    let style_context = &drawing_context.style_context;
    let label_classname = format!("environment_label_{}", env_name);
    style_context.add_class(&label_classname);
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let pango_context = drawing_context.search_result_area.create_pango_context();
    let layout = pango::Layout::new(&pango_context);
    layout.set_text(&env_name.to_uppercase());
    let rect = layout.get_extents().1;
    let text_w = (rect.width / pango::SCALE) as f64;
    let text_h = (rect.height / pango::SCALE) as f64;

    let total_width = text_w + padding.left as f64 + padding.right as f64;
    let total_height = text_h + padding.top as f64 + padding.bottom as f64;

    gtk::render_background(style_context, context, x, y, total_width, total_height);

    gtk::render_frame(style_context, context, x, y, total_width, total_height);

    gtk::render_layout(
        style_context,
        context,
        x + padding.left as f64,
        y + padding.top as f64,
        &layout,
    );
    style_context.remove_class(&label_classname);
    gtk::Rectangle {
        x: x as i32,
        y: y as i32,
        width: total_width as i32,
        height: total_height as i32,
    }
}

fn draw_title(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    text: &str,
    custom_class: Option<String>,
    (x, y): (f64, f64), // only 7 parameters, clippy is happier
    height: Option<i32>,
) -> pango::Rectangle {
    let ItemContext {
        item_link_areas,
        padding,
        ..
    } = item_context;
    let scontext: gtk::StyleContext;
    let style_context = if item_context.is_selected {
        scontext = selected_label_style_context();
        &scontext
    } else {
        &drawing_context.style_context
    };
    let clazz = custom_class
        .as_deref()
        .unwrap_or("search_result_item_title");
    style_context.add_class(clazz);
    let pango_context = drawing_context.search_result_area.create_pango_context();
    let layout = pango::Layout::new(&pango_context);
    layout.set_text(text);
    layout.set_ellipsize(pango::EllipsizeMode::End);
    layout.set_width(350 * pango::SCALE);
    let extra_y = if let Some(h) = height {
        let layout_height = layout.get_extents().1.height as f64 / pango::SCALE as f64;
        (h as f64 - layout_height) / 2.0
    } else {
        0.0
    };
    let left = x + padding.left as f64;
    let top = y + padding.top as f64 + extra_y;
    gtk::render_layout(style_context, &drawing_context.context, left, top, &layout);
    style_context.remove_class(clazz);

    let extents = layout.get_extents().1;

    item_link_areas.push((
        Area::new(
            left as i32,
            top as i32,
            extents.width as i32 / pango::SCALE,
            extents.height as i32 / pango::SCALE,
        ),
        item.clone(),
    ));

    extents
}

fn draw_basic_layout(
    drawing_context: &DrawingContext,
    text: &str,
    x: f64,
    y: f64,
) -> (pango::Rectangle, f64, f64) {
    let context = &drawing_context.context;
    let style_context = &drawing_context.style_context;
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let pango_context = drawing_context.search_result_area.create_pango_context();
    let layout = pango::Layout::new(&pango_context);
    layout.set_text(text);
    layout.set_ellipsize(pango::EllipsizeMode::End);
    layout.set_width(350 * pango::SCALE);
    let left = x + padding.left as f64;
    let top = y + padding.top as f64;
    gtk::render_layout(style_context, context, left, top, &layout);

    (layout.get_extents().1, left, top)
}

fn draw_link(
    drawing_context: &DrawingContext,
    text: &str,
    x: f64,
    y: f64,
    links: &mut Vec<(Area, String)>,
) -> pango::Rectangle {
    drawing_context
        .style_context
        .add_class("search_result_item_link");
    let (extents, left, top) = draw_basic_layout(drawing_context, text, x, y);

    links.push((
        Area::new(
            left as i32,
            top as i32,
            extents.width / pango::SCALE,
            extents.height / pango::SCALE,
        ),
        text.to_string(),
    ));

    drawing_context
        .style_context
        .remove_class("search_result_item_link");
    extents
}

fn draw_subtext(drawing_context: &DrawingContext, text: &str, x: f64, y: f64) -> pango::Rectangle {
    let style_context = &drawing_context.style_context;
    style_context.add_class("search_result_item_subtext");
    let (extents, _left, _top) = draw_basic_layout(drawing_context, text, x, y);
    style_context.remove_class("search_result_item_subtext");
    extents
}

fn draw_action(
    drawing_context: &DrawingContext,
    item_context: &mut ItemContext,
    item: &ProjectPadItem,
    icon: &Icon,
    y: f64,
) {
    let DrawingContext {
        context,
        style_context,
        ..
    } = drawing_context;
    let ItemContext {
        item_with_depressed_action,
        action_areas,
        ..
    } = item_context;
    let x = drawing_context.search_result_area.get_allocation().width as f64
        - ACTION_ICON_OFFSET_FROM_RIGHT
        - LEFT_RIGHT_MARGIN as f64;
    drawing_context
        .style_context
        .add_class("search_result_action_btn");
    let padding = drawing_context
        .style_context
        .get_padding(gtk::StateFlags::NORMAL);
    let w = ACTION_ICON_SIZE as f64 + (padding.left + padding.right) as f64;
    let h = ACTION_ICON_SIZE as f64 + (padding.top + padding.bottom) as f64;
    let flags = if Some(&*item) == item_with_depressed_action.as_ref() {
        gtk::StateFlags::CHECKED
    } else {
        gtk::StateFlags::NORMAL
    };
    let item_type = match item {
        ProjectPadItem::Server(_)
        | ProjectPadItem::ProjectPoi(_)
        | ProjectPadItem::ProjectNote(_) => ItemType::Parent,
        _ => ItemType::Child,
    };
    draw_button(context, item_type, flags, x, y, w, h);
    style_context.remove_class("search_result_action_btn");
    draw_icon(
        style_context,
        context,
        icon,
        x + padding.left as f64,
        y + padding.top as f64,
    );
    action_areas.push((
        Area::new(x as i32, y as i32, w as i32, h as i32),
        item.clone(),
    ));
}

fn draw_icon(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    icon: &Icon,
    x: f64,
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
            icon.name(),
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
    context.mask_surface(&surf, x, y);
}
