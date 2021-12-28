use crate::icons::*;
use gtk::prelude::*;
use relm::Widget;
use relm_derive::{widget, Msg};

#[derive(Msg, Debug)]
pub enum Msg {
    Clicked,
}

pub struct Model {
    icon: Icon,
    title: &'static str,
    subtitle: &'static str,
}

#[widget]
impl Widget for TitleSubtitleBtn {
    fn init_view(&mut self) {}

    fn model(_relm: &relm::Relm<Self>, params: (Icon, &'static str, &'static str)) -> Model {
        let (icon, title, subtitle) = params;
        Model {
            icon,
            title,
            subtitle,
        }
    }

    fn update(&mut self, _event: Msg) {}

    view! {
        gtk::Button {
            gtk::Box {
                gtk::Image {
                    property_icon_name: Some(self.model.icon.name()),
                    // https://github.com/gtk-rs/gtk/issues/837
                    property_icon_size: 5, // gtk::IconSize::Dnd,
                    margin_end: 10,
                },
                gtk::Box {
                    orientation: gtk::Orientation::Vertical,
                    #[style_class="add_btn_title"]
                    gtk::Label {
                        label: self.model.title,
                        xalign: 0.0,
                    },
                    #[style_class="add_btn_subtitle"]
                    gtk::Label {
                        text: self.model.subtitle,
                        xalign: 0.0,
                    },
                },
            },
            clicked => Msg::Clicked,
        },
    }
}
