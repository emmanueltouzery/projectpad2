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

#[derive(PartialEq, Eq)]
enum ItemType {
    Parent,
    Child,
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
    hierarchy_offset: f64,
    style_context: &gtk::StyleContext,
    y: f64,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    is_selected: bool,
    item: &ProjectPadItem,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    operation_mode: OperationMode,
) {
    let margin = style_context.get_margin(gtk::StateFlags::NORMAL);

    let scontext: gtk::StyleContext;
    let bg_context = if is_selected {
        scontext = selected_label_style_context();
        &scontext
    } else {
        style_context
    };
    let box_x = margin.left as f64 + hierarchy_offset;
    let box_y = y + margin.top as f64;
    let box_width = search_result_area.get_allocation().width as f64
        - margin.left as f64
        - margin.right as f64
        - hierarchy_offset * 2.0;
    let box_height = SEARCH_RESULT_WIDGET_HEIGHT as f64 - margin.top as f64;
    gtk::render_background(bg_context, context, box_x, box_y, box_width, box_height);

    // https://github.com/GNOME/gtk/blob/ca71340c6bfa10092c756e5fdd5e41230e2981b5/gtk/theme/Adwaita/gtk-contained.css#L1599
    // use the system theme's frame class
    style_context.add_class(&gtk::STYLE_CLASS_FRAME);
    gtk::render_frame(style_context, context, box_x, box_y, box_width, box_height);

    if operation_mode == OperationMode::SelectItem {
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

pub fn draw_child(
    style_context: &gtk::StyleContext,
    item: &ProjectPadItem,
    y: i32,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    links: &mut Vec<(Area, String)>,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    item_with_depressed_icon: &Option<ProjectPadItem>,
    is_selected: bool,
    operation_mode: OperationMode,
) {
    let extra_css_class = match item {
        ProjectPadItem::Server(_)
        | ProjectPadItem::ProjectNote(_)
        | ProjectPadItem::ProjectPoi(_) => "search_view_parent",
        _ => "search_view_child",
    };
    style_context.add_class(extra_css_class);
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    match item {
        ProjectPadItem::Project(p) => draw_project(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            &p,
        ),
        ProjectPadItem::Server(s) => draw_server(
            style_context,
            context,
            &padding,
            LEFT_RIGHT_MARGIN as f64,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            &s,
            item,
            item_with_depressed_icon,
            links,
            action_areas,
            is_selected,
            operation_mode,
        ),
        ProjectPadItem::ServerNote(n) => draw_server_note(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            item,
            item_with_depressed_icon,
            &n,
            action_areas,
            is_selected,
            operation_mode,
        ),
        ProjectPadItem::ProjectNote(n) => draw_project_note(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            item,
            item_with_depressed_icon,
            &n,
            action_areas,
            is_selected,
            operation_mode,
        ),
        ProjectPadItem::ServerWebsite(w) => draw_server_website(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            item,
            item_with_depressed_icon,
            &w,
            action_areas,
            links,
            is_selected,
            operation_mode,
        ),
        ProjectPadItem::ServerExtraUserAccount(u) => draw_server_extra_user(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            item,
            item_with_depressed_icon,
            &u,
            action_areas,
            is_selected,
            operation_mode,
        ),
        ProjectPadItem::ServerPoi(p) => draw_server_poi(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            item,
            item_with_depressed_icon,
            &p,
            action_areas,
            is_selected,
            operation_mode,
        ),
        ProjectPadItem::ProjectPoi(p) => draw_project_poi(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            item,
            item_with_depressed_icon,
            &p,
            action_areas,
            is_selected,
            operation_mode,
        ),
        ProjectPadItem::ServerDatabase(d) => draw_server_database(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            item,
            item_with_depressed_icon,
            &d,
            action_areas,
            is_selected,
            operation_mode,
        ),
        ProjectPadItem::ServerLink(s) => draw_linked_server(
            style_context,
            context,
            search_result_area,
            padding.left as f64 + LEFT_RIGHT_MARGIN as f64,
            y as f64,
            item,
            item_with_depressed_icon,
            &s,
            action_areas,
            is_selected,
            operation_mode,
        ),
    }
    style_context.remove_class(extra_css_class);
}

fn draw_project(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    project: &Project,
) {
    // since the servers have 10px padding on top of them,
    // let's draw the projects at the bottom of their area
    // so, y+height-icon_size
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let title_extents = draw_title(
        style_context,
        context,
        &padding,
        search_result_area,
        &project.name,
        Some("search_result_project_title".to_string()),
        x,
        y + SEARCH_RESULT_WIDGET_HEIGHT as f64 - PROJECT_ICON_SIZE as f64,
        Some(PROJECT_ICON_SIZE),
        false,
    );

    if let Some(icon) = &project.icon {
        if icon.len() > 0 {
            let translate_x = x + (title_extents.width / pango::SCALE) as f64 + padding.left as f64;
            let translate_y = y + padding.top as f64 + SEARCH_RESULT_WIDGET_HEIGHT as f64
                - PROJECT_ICON_SIZE as f64;
            context.translate(translate_x, translate_y);
            super::project_badge::ProjectBadge::draw_icon(context, PROJECT_ICON_SIZE, &icon);
            context.translate(-translate_x, -translate_y);
        }
    }
}

fn draw_server_item_common(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    title: &str,
    icon: &Icon,
    item: &ProjectPadItem,
    item_with_depressed_action: &Option<ProjectPadItem>,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    is_selected: bool,
    operation_mode: OperationMode,
) -> (gtk::Border, gtk::Border, pango::Rectangle) {
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let margin = style_context.get_margin(gtk::StateFlags::NORMAL);
    draw_box(
        LEFT_RIGHT_MARGIN as f64,
        style_context,
        y,
        context,
        search_result_area,
        is_selected,
        item,
        action_areas,
        operation_mode,
    );
    draw_icon(
        style_context,
        context,
        icon,
        x + padding.left as f64,
        y + margin.top as f64 + padding.top as f64,
    );
    let title_rect = draw_title(
        style_context,
        context,
        &padding,
        search_result_area,
        title,
        None,
        x + ACTION_ICON_SIZE as f64 + (padding.left / 2) as f64,
        y + margin.top as f64,
        Some(ACTION_ICON_SIZE),
        is_selected,
    );
    if operation_mode == OperationMode::ItemActions {
        draw_action(
            style_context,
            context,
            action_areas,
            item,
            item_with_depressed_action,
            &Icon::COG,
            search_result_area.get_allocation().width as f64
                - ACTION_ICON_OFFSET_FROM_RIGHT
                - LEFT_RIGHT_MARGIN as f64,
            y + padding.top as f64 + margin.top as f64,
        );
    }
    (padding, margin, title_rect)
}

fn draw_server_website(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    item: &ProjectPadItem,
    item_with_depressed_action: &Option<ProjectPadItem>,
    website: &ServerWebsite,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    links: &mut Vec<(Area, String)>,
    is_selected: bool,
    operation_mode: OperationMode,
) {
    let (padding, margin, title_rect) = draw_server_item_common(
        style_context,
        context,
        search_result_area,
        x,
        y,
        &website.desc,
        &Icon::HTTP,
        item,
        item_with_depressed_action,
        action_areas,
        is_selected,
        operation_mode,
    );
    draw_link(
        style_context,
        context,
        search_result_area,
        &website.url,
        x + padding.left as f64,
        y + margin.top as f64 + (title_rect.height / pango::SCALE) as f64 + padding.top as f64,
        links,
    );
}

fn draw_server_extra_user(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    item: &ProjectPadItem,
    item_with_depressed_action: &Option<ProjectPadItem>,
    user: &ServerExtraUserAccount,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    is_selected: bool,
    operation_mode: OperationMode,
) {
    let (padding, margin, title_rect) = draw_server_item_common(
        style_context,
        context,
        search_result_area,
        x,
        y,
        &user.username,
        &Icon::USER,
        item,
        item_with_depressed_action,
        action_areas,
        is_selected,
        operation_mode,
    );

    draw_subtext(
        style_context,
        context,
        search_result_area,
        &user.desc,
        x + padding.left as f64,
        y + margin.top as f64 + (title_rect.height / pango::SCALE) as f64 + padding.top as f64,
    );
}

fn draw_server_poi(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    item: &ProjectPadItem,
    item_with_depressed_action: &Option<ProjectPadItem>,
    poi: &ServerPointOfInterest,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    is_selected: bool,
    operation_mode: OperationMode,
) {
    let (padding, margin, title_rect) = draw_server_item_common(
        style_context,
        context,
        search_result_area,
        x,
        y,
        &poi.desc,
        &Icon::POINT_OF_INTEREST,
        item,
        item_with_depressed_action,
        action_areas,
        is_selected,
        operation_mode,
    );

    draw_subtext(
        style_context,
        context,
        search_result_area,
        &poi.text,
        x + padding.left as f64,
        y + margin.top as f64 + (title_rect.height / pango::SCALE) as f64 + padding.top as f64,
    );
}

fn draw_project_poi(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    item: &ProjectPadItem,
    item_with_depressed_action: &Option<ProjectPadItem>,
    poi: &ProjectPointOfInterest,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    is_selected: bool,
    operation_mode: OperationMode,
) {
    let (padding, margin, title_rect) = draw_server_item_common(
        style_context,
        context,
        search_result_area,
        x,
        y,
        &poi.desc,
        &Icon::POINT_OF_INTEREST,
        item,
        item_with_depressed_action,
        action_areas,
        is_selected,
        operation_mode,
    );

    draw_subtext(
        style_context,
        context,
        search_result_area,
        &poi.text,
        x + padding.left as f64,
        y + margin.top as f64 + (title_rect.height / pango::SCALE) as f64 + padding.top as f64,
    );
}

fn draw_server_database(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    item: &ProjectPadItem,
    item_with_depressed_action: &Option<ProjectPadItem>,
    db: &ServerDatabase,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    is_selected: bool,
    operation_mode: OperationMode,
) {
    let (padding, margin, title_rect) = draw_server_item_common(
        style_context,
        context,
        search_result_area,
        x,
        y,
        &db.desc,
        &Icon::DATABASE,
        item,
        item_with_depressed_action,
        action_areas,
        is_selected,
        operation_mode,
    );

    draw_subtext(
        style_context,
        context,
        search_result_area,
        &format!("{} {}", db.text, db.username),
        x + padding.left as f64,
        y + margin.top as f64 + (title_rect.height / pango::SCALE) as f64 + padding.top as f64,
    );
}

fn draw_linked_server(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    item: &ProjectPadItem,
    item_with_depressed_action: &Option<ProjectPadItem>,
    srv: &ServerLink,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    is_selected: bool,
    operation_mode: OperationMode,
) {
    let (_padding, _margin, _title_rect) = draw_server_item_common(
        style_context,
        context,
        search_result_area,
        x,
        y,
        &srv.desc,
        &Icon::SERVER_LINK,
        item,
        item_with_depressed_action,
        action_areas,
        is_selected,
        operation_mode,
    );
}

fn draw_project_note(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    item: &ProjectPadItem,
    item_with_depressed_action: &Option<ProjectPadItem>,
    note: &ProjectNote,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    is_selected: bool,
    operation_mode: OperationMode,
) {
    let (_padding, _margin, _title_rect) = draw_server_item_common(
        style_context,
        context,
        search_result_area,
        x,
        y,
        &note.title,
        &Icon::NOTE,
        item,
        item_with_depressed_action,
        action_areas,
        is_selected,
        operation_mode,
    );
}

fn draw_server_note(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    item: &ProjectPadItem,
    item_with_depressed_action: &Option<ProjectPadItem>,
    note: &ServerNote,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    is_selected: bool,
    operation_mode: OperationMode,
) {
    let (_padding, _margin, _title_rect) = draw_server_item_common(
        style_context,
        context,
        search_result_area,
        x,
        y,
        &note.title,
        &Icon::NOTE,
        item,
        item_with_depressed_action,
        action_areas,
        is_selected,
        operation_mode,
    );
}

fn draw_server(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    padding: &gtk::Border,
    hierarchy_offset: f64,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    server: &Server,
    item: &ProjectPadItem,
    item_with_depressed_action: &Option<ProjectPadItem>,
    links: &mut Vec<(Area, String)>,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    is_selected: bool,
    operation_mode: OperationMode,
) {
    let margin = style_context.get_margin(gtk::StateFlags::NORMAL);
    draw_box(
        hierarchy_offset,
        style_context,
        y,
        context,
        search_result_area,
        is_selected,
        item,
        action_areas,
        operation_mode,
    );
    style_context.add_class("title");
    let title_rect = draw_title(
        style_context,
        context,
        &padding,
        search_result_area,
        &server.desc,
        None,
        x,
        y + margin.top as f64,
        None,
        is_selected,
    );
    style_context.remove_class("title");
    let env_rect = draw_environment(
        style_context,
        context,
        search_result_area,
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
            style_context,
            context,
            search_result_area,
            &server.ip,
            (env_rect.x + env_rect.width) as f64,
            y + (title_rect.height / pango::SCALE) as f64 + padding.top as f64,
            links,
        );
    }
    if operation_mode == OperationMode::ItemActions {
        draw_action(
            style_context,
            context,
            action_areas,
            item,
            item_with_depressed_action,
            &Icon::COG,
            search_result_area.get_allocation().width as f64
                - ACTION_ICON_OFFSET_FROM_RIGHT
                - LEFT_RIGHT_MARGIN as f64,
            y + padding.top as f64 + margin.top as f64,
        );
    }
}

fn draw_environment(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    x: f64,
    y: f64,
    env_name: &str,
) -> gtk::Rectangle {
    let label_classname = format!("environment_label_{}", env_name);
    style_context.add_class(&label_classname);
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let pango_context = search_result_area.create_pango_context();
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
    style_context_orig: &gtk::StyleContext,
    context: &cairo::Context,
    padding: &gtk::Border,
    search_result_area: &gtk::DrawingArea,
    text: &str,
    custom_class: Option<String>,
    x: f64,
    y: f64,
    height: Option<i32>,
    is_selected: bool,
) -> pango::Rectangle {
    let mut scontext: gtk::StyleContext;
    let style_context = if is_selected {
        scontext = selected_label_style_context();
        &scontext
    } else {
        style_context_orig
    };
    let clazz = custom_class
        .as_deref()
        .unwrap_or("search_result_item_title");
    style_context.add_class(clazz);
    let pango_context = search_result_area.create_pango_context();
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
    gtk::render_layout(
        style_context,
        context,
        x + padding.left as f64,
        y + padding.top as f64 + extra_y,
        &layout,
    );
    style_context.remove_class(clazz);

    layout.get_extents().1
}

fn draw_basic_layout(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    text: &str,
    x: f64,
    y: f64,
) -> (pango::Rectangle, f64, f64) {
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let pango_context = search_result_area.create_pango_context();
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
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    text: &str,
    x: f64,
    y: f64,
    links: &mut Vec<(Area, String)>,
) -> pango::Rectangle {
    style_context.add_class("search_result_item_link");
    let (extents, left, top) =
        draw_basic_layout(style_context, context, search_result_area, text, x, y);

    links.push((
        Area::new(
            left as i32,
            top as i32,
            extents.width / pango::SCALE,
            extents.height / pango::SCALE,
        ),
        text.to_string(),
    ));

    style_context.remove_class("search_result_item_link");
    extents
}

fn draw_subtext(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    search_result_area: &gtk::DrawingArea,
    text: &str,
    x: f64,
    y: f64,
) -> pango::Rectangle {
    style_context.add_class("search_result_item_subtext");
    let (extents, _left, _top) =
        draw_basic_layout(style_context, context, search_result_area, text, x, y);
    style_context.remove_class("search_result_item_subtext");
    extents
}

fn draw_action(
    style_context: &gtk::StyleContext,
    context: &cairo::Context,
    action_areas: &mut Vec<(Area, ProjectPadItem)>,
    item: &ProjectPadItem,
    item_with_depressed_icon: &Option<ProjectPadItem>,
    icon: &Icon,
    x: f64,
    y: f64,
) {
    style_context.add_class("search_result_action_btn");
    let padding = style_context.get_padding(gtk::StateFlags::NORMAL);
    let w = ACTION_ICON_SIZE as f64 + (padding.left + padding.right) as f64;
    let h = ACTION_ICON_SIZE as f64 + (padding.top + padding.bottom) as f64;
    let flags = if Some(item) == item_with_depressed_icon.as_ref() {
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
