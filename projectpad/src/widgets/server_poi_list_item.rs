use crate::icons::*;
use gtk::prelude::*;
use projectpadsql::models::{InterestType, ServerPointOfInterest};
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    server_poi: ServerPointOfInterest,
    icon: Icon,
}

#[widget]
impl Widget for ServerPoiListItem {
    fn init_view(&mut self) {
        self.items_frame
            .get_style_context()
            .add_class("items_frame");
        self.title
            .get_style_context()
            .add_class("items_frame_title");
        for l in &[&mut self.label1, &mut self.label2] {
            l.get_style_context().add_class("item_label");
        }
    }

    fn model(relm: &relm::Relm<Self>, server_poi: ServerPointOfInterest) -> Model {
        Model {
            icon: Self::get_icon(&server_poi),
            server_poi,
        }
    }

    fn update(&mut self, _event: Msg) {}

    fn get_icon(server_poi: &ServerPointOfInterest) -> Icon {
        match server_poi.interest_type {
            InterestType::PoiLogFile => Icon::LOG_FILE,
            InterestType::PoiConfigFile => Icon::CONFIG_FILE,
            InterestType::PoiApplication => Icon::FOLDER_PLUS,
            InterestType::PoiCommandToRun => Icon::COG,
            InterestType::PoiBackupArchive => Icon::ARCHIVE,
            InterestType::PoiCommandTerminal => Icon::TERMINAL,
        }
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
                        property_icon_name: Some(self.model.icon.name()),
                        // https://github.com/gtk-rs/gtk/issues/837
                        property_icon_size: 1, // gtk::IconSize::Menu,
                    },
                    gtk::Label {
                        margin_start: 5,
                        text: &self.model.server_poi.desc
                    },
                },
                #[name="label1"]
                gtk::Label {
                    cell: {
                        left_attach: 0,
                        top_attach: 1,
                    },
                    text: "Path"
                },
                gtk::Label {
                    cell: {
                        left_attach: 1,
                        top_attach: 1,
                    },
                    hexpand: true,
                    xalign: 0.0,
                    text: &self.model.server_poi.path
                },
                #[name="label2"]
                gtk::Label {
                    cell: {
                        left_attach: 0,
                        top_attach: 2,
                    },
                    text: "Text"
                },
                gtk::Label {
                    cell: {
                        left_attach: 1,
                        top_attach: 2,
                    },
                    hexpand: true,
                    xalign: 0.0,
                    text: &self.model.server_poi.text
                }
            }
        }
    }
}
