use super::project_items_list::ProjectItem;
use crate::icons::Icon;
use gtk::prelude::*;
use projectpadsql::models::{ProjectPointOfInterest, Server, ServerAccessType};
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    ProjectItemSelected(Option<ProjectItem>),
    ServerHeaderActionsClicked,
}

pub struct Model {
    project_item: Option<ProjectItem>,
    header_popover: gtk::Popover,
    title: gtk::Label,
}

#[widget]
impl Widget for ProjectPoiHeader {
    fn init_view(&mut self) {
        self.items_frame
            .get_style_context()
            .add_class("items_frame");
        for l in &[
            &mut self.server_label1,
            &mut self.server_label2,
            &mut self.server_label3,
            &mut self.poi_label1,
            &mut self.poi_label2,
        ] {
            l.get_style_context().add_class("item_label");
        }

        self.model
            .title
            .get_style_context()
            .add_class("header_frame_title");
        self.model.title.show_all();

        let vbox = gtk::BoxBuilder::new()
            .margin(10)
            .orientation(gtk::Orientation::Vertical)
            .build();
        vbox.add(&gtk::ModelButtonBuilder::new().label("Edit").build());
        vbox.show_all();
        self.model.header_popover.add(&vbox);
        self.header_actions_btn
            .set_popover(Some(&self.model.header_popover));
    }

    fn model(relm: &relm::Relm<Self>, _: ()) -> Model {
        Model {
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
                self.header_contents.set_visible_child_name(match &pi {
                    Some(ProjectItem::Server(_)) => "server",
                    Some(ProjectItem::ProjectPointOfInterest(_)) => "poi",
                    _ => "none",
                });
                self.model.project_item = pi;
                self.model.title.set_markup(
                    self.model
                        .project_item
                        .as_ref()
                        .map(Self::project_item_desc)
                        .as_deref()
                        .unwrap_or(""),
                );
            }
            Msg::ServerHeaderActionsClicked => {
                println!("server header actions clicked");
            }
        }
    }

    fn as_server(pi: &ProjectItem) -> Option<&Server> {
        match pi {
            ProjectItem::Server(srv) => Some(srv),
            _ => None,
        }
    }

    fn as_poi(pi: &ProjectItem) -> Option<&ProjectPointOfInterest> {
        match pi {
            ProjectItem::ProjectPointOfInterest(poi) => Some(poi),
            _ => None,
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
                        button_release_event(c, _) =>
                            (Msg::ServerHeaderActionsClicked, Inhibit(false))
                    },
                },
                #[name="header_contents"]
                gtk::Stack {
                    homogeneous: false,
                    gtk::Box {
                        child: {
                            name: Some("none"),
                        },
                    },
                    // POI HEADER
                    gtk::Grid {
                        margin_start: 30,
                        margin_end: 30,
                        margin_top: 10,
                        margin_bottom: 5,
                        row_spacing: 5,
                        column_spacing: 10,
                        child: {
                            name: Some("poi"),
                        },
                        #[name="poi_label1"]
                        gtk::Label {
                            cell: {
                                left_attach: 0,
                                top_attach: 0,
                            },
                            text: "Path",
                        },
                        gtk::Label {
                            margin_start: 5,
                            xalign: 0.0,
                            markup: &self.model.project_item
                                         .as_ref()
                                         .and_then(Self::as_poi)
                                         .map(|p| p.path.to_string())
                                         .unwrap_or_else(|| "".to_string())
                        },
                        #[name="poi_label2"]
                        gtk::Label {
                            cell: {
                                left_attach: 0,
                                top_attach: 1,
                            },
                            text: "Text",
                        },
                        gtk::Label {
                            cell: {
                                left_attach: 1,
                                top_attach: 1,
                            },
                            xalign: 0.0,
                            text: &self.model.project_item
                                         .as_ref()
                                         .and_then(Self::as_poi)
                                         .map(|s| s.text.to_string())
                                         .unwrap_or_else(|| "".to_string())
                        },
                    },
                    // SERVER HEADER
                    gtk::Grid {
                        margin_start: 30,
                        margin_end: 30,
                        margin_top: 10,
                        margin_bottom: 5,
                        row_spacing: 5,
                        column_spacing: 10,
                        child: {
                            name: Some("server"),
                        },
                        #[name="server_label1"]
                        gtk::Label {
                            cell: {
                                left_attach: 0,
                                top_attach: 0,
                            },
                            text: "Address",
                        },
                        gtk::Box {
                            cell: {
                                left_attach: 1,
                                top_attach: 0,
                            },
                            gtk::Image {
                                property_icon_name: self.model.project_item
                                                              .as_ref()
                                                              .and_then(Self::as_server)
                                                              .map(Self::server_access_icon)
                                                              .map(|i| i.name()),
                                // https://github.com/gtk-rs/gtk/issues/837
                                property_icon_size: 1, // gtk::IconSize::Menu,
                            },
                            gtk::Label {
                                margin_start: 5,
                                xalign: 0.0,
                                markup: &self.model.project_item
                                             .as_ref()
                                             .and_then(Self::as_server)
                                             .map(Self::server_ip_display)
                                             .unwrap_or_else(|| "".to_string())
                            }
                        },
                        #[name="server_label2"]
                        gtk::Label {
                            cell: {
                                left_attach: 0,
                                top_attach: 1,
                            },
                            text: "Username",
                        },
                        gtk::Label {
                            cell: {
                                left_attach: 1,
                                top_attach: 1,
                            },
                            xalign: 0.0,
                            text: &self.model.project_item
                                         .as_ref()
                                         .and_then(Self::as_server)
                                         .map(|s| s.username.to_string())
                                         .unwrap_or_else(|| "".to_string())
                        },
                        #[name="server_label3"]
                        gtk::Label {
                            cell: {
                                left_attach: 0,
                                top_attach: 2,
                            },
                            text: "Password",
                        },
                        gtk::Label {
                            cell: {
                                left_attach: 1,
                                top_attach: 2,
                            },
                            xalign: 0.0,
                            text: self.model.project_item
                                         .as_ref()
                                         .and_then(Self::as_server)
                                         .map(|s| if s.password.is_empty() { "" } else { "●●●●●" })
                                         .unwrap_or_else(|| "")
                        },
                    },
                }
            }
        }
    }
}
