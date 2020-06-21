use crate::icons::*;
use gtk::prelude::*;
use projectpadsql::models::ServerWebsite;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    server_website: ServerWebsite,
}

#[widget]
impl Widget for ServerWebsiteListItem {
    fn init_view(&mut self) {
        self.items_frame
            .get_style_context()
            .add_class("items_frame");
        self.title
            .get_style_context()
            .add_class("items_frame_title");
        for l in &[&mut self.label1, &mut self.label2, &mut self.label3] {
            l.get_style_context().add_class("item_label");
        }
    }

    fn model(relm: &relm::Relm<Self>, server_website: ServerWebsite) -> Model {
        Model { server_website }
    }

    fn update(&mut self, _event: Msg) {}

    fn format_link(str: &str) -> String {
        format!("<a href='{}'>{}</a>", str, str)
    }

    view! {
        #[name="items_frame"]
        gtk::Frame {
            margin_start: 20,
            margin_end: 20,
            margin_top: 20,
            gtk::Grid {
                margin_start: 10,
                margin_end: 10,
                margin_top: 10,
                margin_bottom: 5,
                row_spacing: 5,
                column_spacing: 10,
                #[name="title"]
                gtk::Box {
                    cell: {
                        left_attach: 0,
                        top_attach: 0,
                    },
                    orientation: gtk::Orientation::Horizontal,
                    cell: {
                        width: 2
                    },
                    gtk::Image {
                        property_icon_name: Some(Icon::HTTP.name()),
                        // https://github.com/gtk-rs/gtk/issues/837
                        property_icon_size: 1, // gtk::IconSize::Menu,
                    },
                    gtk::Label {
                        margin_start: 5,
                        text: &self.model.server_website.desc,
                        ellipsize: pango::EllipsizeMode::End,
                    },
                },
                #[name="label1"]
                gtk::Label {
                    cell: {
                        left_attach: 0,
                        top_attach: 1,
                    },
                    text: "Address"
                },
                gtk::Label {
                    cell: {
                        left_attach: 1,
                        top_attach: 1,
                    },
                    hexpand: true,
                    xalign: 0.0,
                    markup: &Self::format_link(&self.model.server_website.url),
                    ellipsize: pango::EllipsizeMode::End,
                },
                #[name="label2"]
                gtk::Label {
                    cell: {
                        left_attach: 0,
                        top_attach: 2,
                    },
                    text: "Username"
                },
                gtk::Label {
                    cell: {
                        left_attach: 1,
                        top_attach: 2,
                    },
                    hexpand: true,
                    xalign: 0.0,
                    text: &self.model.server_website.username,
                    ellipsize: pango::EllipsizeMode::End,
                },
                #[name="label3"]
                gtk::Label {
                    cell: {
                        left_attach: 0,
                        top_attach: 3,
                    },
                    text: "Password"
                },
                gtk::Label {
                    cell: {
                        left_attach: 1,
                        top_attach: 3,
                    },
                    hexpand: true,
                    xalign: 0.0,
                    text: if self.model.server_website.password.is_empty() { "" } else { "●●●●●"},
                    ellipsize: pango::EllipsizeMode::End,
                }
            }
        }
    }
}
