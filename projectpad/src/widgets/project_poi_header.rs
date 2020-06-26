use super::project_items_list::ProjectItem;
use crate::icons::Icon;
use gtk::prelude::*;
use projectpadsql::models::{ProjectPointOfInterest, Server, ServerAccessType};
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    ProjectItemSelected(Option<ProjectItem>),
    CopyClicked(String),
}

pub struct Model {
    relm: relm::Relm<ProjectPoiHeader>,
    project_item: Option<ProjectItem>,
    header_popover: gtk::Popover,
    title: gtk::Label,
}

pub struct GridItem {
    pub label_name: &'static str,
    pub icon: Option<Icon>,
    pub markup: String,
    pub raw_value: String,
}

impl GridItem {
    pub fn new(
        label_name: &'static str,
        icon: Option<Icon>,
        markup: String,
        raw_value: String,
    ) -> GridItem {
        GridItem {
            label_name,
            icon,
            markup,
            raw_value,
        }
    }
}

pub fn populate_grid(
    header_grid: gtk::Grid,
    actions_popover: gtk::Popover,
    fields: &[GridItem],
    register_btn: &dyn Fn(gtk::ModelButton, String),
) {
    for child in header_grid.get_children() {
        header_grid.remove(&child);
    }
    for child in actions_popover.get_children() {
        actions_popover.remove(&child);
    }
    let popover_vbox = gtk::BoxBuilder::new()
        .margin(10)
        .orientation(gtk::Orientation::Vertical)
        .build();
    popover_vbox.add(&gtk::ModelButtonBuilder::new().label("Edit").build());

    let mut i = 0;
    for item in fields {
        let label = gtk::LabelBuilder::new().label(&item.label_name).build();
        header_grid.attach(&label, 0, i, 1, 1);
        label.get_style_context().add_class("item_label");

        let label = gtk::LabelBuilder::new()
            .use_markup(true)
            .label(&item.markup)
            .xalign(0.0)
            .build();

        if let Some(icon) = &item.icon {
            let gbox = gtk::BoxBuilder::new().build();
            // https://github.com/gtk-rs/gtk/issues/837
            // property_icon_size: 1, // gtk::IconSize::Menu,
            gbox.add(
                &gtk::ImageBuilder::new()
                    .icon_name(&icon.name())
                    .icon_size(1)
                    .margin_end(5)
                    .build(),
            );
            gbox.add(&label);
            header_grid.attach(&gbox, 1, i, 1, 1);
        } else {
            header_grid.attach(&label, 1, i, 1, 1);
        }

        let popover_btn = gtk::ModelButtonBuilder::new()
            .label(&format!("Copy {}", item.label_name))
            .build();
        register_btn(popover_btn.clone(), item.raw_value.clone());
        popover_vbox.add(&popover_btn);

        i += 1;
    }
    header_grid.show_all();
    popover_vbox.show_all();
    actions_popover.add(&popover_vbox);
}

#[widget]
impl Widget for ProjectPoiHeader {
    fn init_view(&mut self) {
        self.items_frame
            .get_style_context()
            .add_class("items_frame");

        self.model
            .title
            .get_style_context()
            .add_class("header_frame_title");
        self.model.title.show_all();

        self.header_actions_btn
            .set_popover(Some(&self.model.header_popover));
    }

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {
            relm: relm.clone(),
            project_item: None,
            header_popover: gtk::Popover::new(None::<&gtk::Button>),
            title: gtk::LabelBuilder::new()
                .margin_top(8)
                .margin_bottom(8)
                .hexpand(true)
                .build(),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ProjectItemSelected(pi) => {
                self.model.project_item = pi;
                self.populate_header();
                self.model.title.set_markup(
                    self.model
                        .project_item
                        .as_ref()
                        .map(Self::project_item_desc)
                        .as_deref()
                        .unwrap_or(""),
                );
            }
            Msg::CopyClicked(val) => {
                if let Some(clip) = self
                    .header_grid
                    .get_display()
                    .as_ref()
                    .and_then(gtk::Clipboard::get_default)
                {
                    clip.set_text(&val);
                }
            }
        }
    }

    fn server_access_icon(srv: &Server) -> Icon {
        match srv.access_type {
            ServerAccessType::SrvAccessSsh => Icon::SSH,
            ServerAccessType::SrvAccessSshTunnel => Icon::SSH,
            ServerAccessType::SrvAccessRdp => Icon::WINDOWS,
            ServerAccessType::SrvAccessWww => Icon::HTTP,
        }
    }

    fn project_item_desc(pi: &ProjectItem) -> &str {
        match pi {
            ProjectItem::Server(srv) => &srv.desc,
            ProjectItem::ServerLink(srv) => &srv.desc,
            ProjectItem::ProjectNote(note) => &note.title,
            ProjectItem::ProjectPointOfInterest(poi) => &poi.desc,
        }
    }

    fn server_ip_display(srv: &Server) -> String {
        if srv.access_type == ServerAccessType::SrvAccessWww {
            format!("<a href=\"{}\">{}</a>", srv.ip, srv.ip)
        } else {
            srv.ip.clone()
        }
    }

    fn populate_header(&self) {
        let fields = match &self.model.project_item {
            Some(ProjectItem::Server(srv)) => vec![
                GridItem::new(
                    "Address",
                    Some(Self::server_access_icon(&srv)),
                    Self::server_ip_display(&srv),
                    srv.ip.clone(),
                ),
                GridItem::new("Username", None, srv.username.clone(), srv.username.clone()),
                GridItem::new(
                    "Password",
                    None,
                    if srv.password.is_empty() {
                        "".to_string()
                    } else {
                        "●●●●●".to_string()
                    },
                    srv.password.clone(),
                ),
            ],
            Some(ProjectItem::ProjectPointOfInterest(poi)) => vec![
                GridItem::new("Path", None, poi.path.clone(), poi.path.clone()),
                GridItem::new("Text", None, poi.text.clone(), poi.path.clone()),
            ],
            _ => vec![],
        };
        populate_grid(
            self.header_grid.clone(),
            self.model.header_popover.clone(),
            &fields,
            &|btn: gtk::ModelButton, str_val: String| {
                relm::connect!(
                    self.model.relm,
                    btn,
                    connect_clicked(_),
                    Msg::CopyClicked(str_val.clone())
                );
            },
        );
    }

    view! {
        #[name="items_frame"]
        gtk::Frame {
            hexpand: true,
            margin_start: 10,
            margin_end: 10,
            margin_top: 10,
            gtk::Box {
                hexpand: true,
                orientation: gtk::Orientation::Vertical,
                #[name="titlebox"]
                gtk::Box {
                    hexpand: true,
                    orientation: gtk::Orientation::Horizontal,
                    center_widget: Some(&self.model.title),
                    #[name="header_actions_btn"]
                    gtk::MenuButton {
                        child: {
                            pack_type: gtk::PackType::End,
                        },
                        always_show_image: true,
                        image: Some(&gtk::Image::new_from_icon_name(
                            Some(Icon::COG.name()), gtk::IconSize::Menu)),
                        halign: gtk::Align::End,
                        valign: gtk::Align::Center,
                        margin_end: 10,
                    },
                },
                #[name="header_grid"]
                gtk::Grid {
                    margin_start: 30,
                    margin_end: 30,
                    margin_top: 10,
                    margin_bottom: 5,
                    row_spacing: 5,
                    column_spacing: 10,
                },
            }
        }
    }
}
