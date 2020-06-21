use crate::icons::*;
use gtk::prelude::*;
use projectpadsql::models::ServerNote;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    server_note: ServerNote,
    truncated_contents: String,
}

#[widget]
impl Widget for ServerNoteListItem {
    fn init_view(&mut self) {
        self.items_frame
            .get_style_context()
            .add_class("items_frame");
        self.title
            .get_style_context()
            .add_class("items_frame_title");
    }

    fn model(relm: &relm::Relm<Self>, server_note: ServerNote) -> Model {
        Model {
            truncated_contents: Self::truncate(&server_note.contents),
            server_note,
        }
    }

    fn update(&mut self, _event: Msg) {}

    fn truncate(contents: &str) -> String {
        contents.lines().take(3).collect::<Vec<_>>().join("\n")
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
                        property_icon_name: Some(Icon::NOTE.name()),
                        // https://github.com/gtk-rs/gtk/issues/837
                        property_icon_size: 1, // gtk::IconSize::Menu,
                    },
                    gtk::Label {
                        margin_start: 5,
                        text: &self.model.server_note.title,
                        ellipsize: pango::EllipsizeMode::End,
                    },
                },
                gtk::Label {
                    cell: {
                        left_attach: 0,
                        top_attach: 1,
                        width: 2
                    },
                    hexpand: true,
                    xalign: 0.0,
                    single_line_mode: false,
                    // TODO markdown formatting
                    // TODO if the text was truncated, do some fading out
                    markup: &self.model.truncated_contents,
                    ellipsize: pango::EllipsizeMode::End,
                },
            }
        }
    }
}
