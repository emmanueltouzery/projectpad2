use super::project_poi_header::{populate_grid, GridItem};
use crate::icons::*;
use gtk::prelude::*;
use projectpadsql::models::ServerWebsite;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {
    CopyClicked(String),
}

pub struct Model {
    relm: relm::Relm<ServerWebsiteListItem>,
    server_website: ServerWebsite,
    header_popover: gtk::Popover,
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

        self.header_actions_btn
            .set_popover(Some(&self.model.header_popover));
        let items = populate_grid(
            self.items_grid.clone(),
            self.model.header_popover.clone(),
            &[
                GridItem::new(
                    "Address",
                    Some(Icon::HTTP),
                    format!(
                        "<a href=\"{}\">{}</a>",
                        self.model.server_website.url, self.model.server_website.url
                    ),
                    self.model.server_website.url.clone(),
                ),
                GridItem::new(
                    "Username",
                    None,
                    self.model.server_website.username.clone(),
                    self.model.server_website.username.clone(),
                ),
                GridItem::new(
                    "Password",
                    None,
                    if self.model.server_website.username.is_empty() {
                        "".to_string()
                    } else {
                        "●●●●●".to_string()
                    },
                    self.model.server_website.password.clone(),
                ),
            ],
            &|btn: &gtk::ModelButton, str_val: String| {
                relm::connect!(
                    self.model.relm,
                    &btn,
                    connect_clicked(_),
                    Msg::CopyClicked(str_val.clone())
                );
            },
        );
    }

    fn model(relm: &relm::Relm<Self>, server_website: ServerWebsite) -> Model {
        Model {
            relm: relm.clone(),
            server_website,
            header_popover: gtk::Popover::new(None::<&gtk::Button>),
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::CopyClicked(val) => {
                if let Some(clip) = self
                    .items_grid
                    .get_display()
                    .as_ref()
                    .and_then(gtk::Clipboard::get_default)
                {
                    clip.set_text(&val);
                }
            }
        }
    }

    fn format_link(str: &str) -> String {
        format!("<a href='{}'>{}</a>", str, str)
    }

    view! {
        #[name="items_frame"]
        gtk::Frame {
            margin_start: 20,
            margin_end: 20,
            margin_top: 20,
            gtk::Box {
                orientation: gtk::Orientation::Vertical,
                #[name="title"]
                gtk::Box {
                    orientation: gtk::Orientation::Horizontal,
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
                    },
                },
                #[name="items_grid"]
                gtk::Grid {
                    margin_start: 10,
                    margin_end: 10,
                    margin_top: 10,
                    margin_bottom: 5,
                    row_spacing: 5,
                    column_spacing: 10,
                }
            }
        }
    }
}
