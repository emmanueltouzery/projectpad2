use adw::prelude::*;
use gtk::gdk;

use crate::{app::ProjectpadApplication, widgets::project_item::WidgetMode};

pub fn get_contents_box_with_header(title: &str, widget_mode: WidgetMode) -> gtk::Box {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(10)
        .margin_top(10)
        .build();

    let header_box = gtk::Box::builder().spacing(10).build();

    let server_icon = gtk::Image::builder()
        .icon_name("server")
        .pixel_size(48)
        .build();
    header_box.append(&server_icon);

    let header_second_col = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .valign(gtk::Align::Center)
        .build();

    if widget_mode == WidgetMode::Edit {
        let server = gtk::Entry::builder()
            .text(title)
            .halign(gtk::Align::Start)
            .css_classes(["title-1"])
            // .description("desc")
            .build();
        header_second_col.append(&server);
    } else {
        let server = gtk::Label::builder()
            .label(title)
            .halign(gtk::Align::Start)
            .css_classes(["title-1"])
            // .description("desc")
            .build();
        header_second_col.append(&server);
    }

    header_box.append(&header_second_col);

    if widget_mode == WidgetMode::Edit {
        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .halign(gtk::Align::End)
            .hexpand(true)
            .build();
        header_box.append(&delete_btn);
    }

    vbox.append(&header_box);
    vbox
}

pub fn copy_to_clipboard(text: &str) {
    if let Some(display) = gdk::Display::default() {
        display.clipboard().set_text(text);

        let toast_overlay = gio::Application::default()
            .expect("Failed to retrieve application singleton")
            .downcast::<ProjectpadApplication>()
            .unwrap()
            .get_toast_overlay();
        toast_overlay.add_toast(adw::Toast::new("Copied to the clipboard"));
    }
}
