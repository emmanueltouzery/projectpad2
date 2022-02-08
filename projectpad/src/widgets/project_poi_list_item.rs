use crate::icons::Icon;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg)]
pub enum Msg {}

pub struct Model {
    pub icon: Icon,
    pub markup: String,
    pub group_name: Option<String>,
}

#[widget]
impl Widget for ProjectPoiListItem {
    fn model(_relm: &relm::Relm<Self>, model: Model) -> Model {
        model
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        gtk::Box {
            // not sure why the -5 is needed. some padding on the parent ListBoxRow or something?
            width_request: 260-5,
            spacing: 10,
            border_width: 10,
            orientation: gtk::Orientation::Vertical,
            gtk::Box {
                    child: {
                        pack_type: gtk::PackType::Start,
                        expand: true,
                        fill: true,
                    },
                spacing: 10,
                gtk::Image {
                    icon_name: Some(self.model.icon.name()),
                    // https://github.com/gtk-rs/gtk/issues/837
                    // property_icon_size: 4, // gtk::IconSize::Dnd
                    pixel_size: 24,
                },
                gtk::Box {
                    orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        markup: &self.model.markup,
                        ellipsize: pango::EllipsizeMode::End,
                        xalign: 0.0,
                        vexpand: true,
                    },
                }
            },
        }
    }
}
