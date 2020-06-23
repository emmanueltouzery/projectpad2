use crate::icons::Icon;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    pub icon: Icon,
    pub text: String,
    pub secondary_desc: Option<String>,
    pub group_name: Option<String>,
}

#[widget]
impl Widget for ProjectPoiListItem {
    fn model(relm: &relm::Relm<Self>, model: Model) -> Model {
        model
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        gtk::Box {
            // not sure why the -4 is needed. some padding on the parent ListBoxRow or something?
            property_width_request: 260-4,
            spacing: 10,
            border_width: 5,
            orientation: gtk::Orientation::Vertical,
            gtk::Box {
                    child: {
                        pack_type: gtk::PackType::Start,
                        expand: true,
                        fill: true,
                    },
                spacing: 10,
                gtk::Image {
                    property_icon_name: Some(self.model.icon.name()),
                    // https://github.com/gtk-rs/gtk/issues/837
                    property_icon_size: 5, // gtk::IconSize::Dnd
                },
                gtk::Box {
                    orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        text: &self.model.text,
                        ellipsize: pango::EllipsizeMode::End,
                        xalign: 0.0
                    },
                    gtk::Label {
                        text: self.model.secondary_desc.as_deref().unwrap_or(""),
                        ellipsize: pango::EllipsizeMode::End,
                        xalign: 0.0
                    }
                }
            },
        }
    }
}
