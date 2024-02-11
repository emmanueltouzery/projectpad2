use std::rc::Rc;

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PasswordMode {
    PlainText,
    Password,
}

pub struct DetailsRow<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub suffix_icon: Option<&'static str>,
    pub password_mode: PasswordMode,
    pub action: Option<Rc<Box<dyn Fn() -> ()>>>,
}

pub struct SuffixAction {
    pub icon: &'static str,
    pub action: Rc<Box<dyn Fn() -> ()>>,
}

impl SuffixAction {
    pub fn copy(txt: &str) -> Option<SuffixAction> {
        let t = txt.to_owned();
        Some(SuffixAction {
            icon: "edit-copy-symbolic",
            action: Rc::new(Box::new(move || copy_to_clipboard(&t))),
        })
    }
}

impl DetailsRow<'_> {
    pub fn new<'a>(
        title: &'a str,
        subtitle: &'a str,
        suffix_action: Option<SuffixAction>,
    ) -> DetailsRow<'a> {
        DetailsRow {
            title,
            subtitle,
            password_mode: PasswordMode::PlainText,
            suffix_icon: suffix_action.as_ref().map(|a| a.icon),
            action: suffix_action.as_ref().map(|a| a.action.clone()),
        }
    }

    pub fn new_password<'a>(
        title: &'a str,
        subtitle: &'a str,
        suffix_action: Option<SuffixAction>,
    ) -> DetailsRow<'a> {
        DetailsRow {
            title,
            subtitle,
            password_mode: PasswordMode::Password,
            suffix_icon: suffix_action.as_ref().map(|a| a.icon),
            action: suffix_action.as_ref().map(|a| a.action.clone()),
        }
    }

    pub fn add(&self, widget_mode: WidgetMode, group: &adw::PreferencesGroup) {
        match widget_mode {
            WidgetMode::Show => self.add_show(group),
            WidgetMode::Edit => self.add_edit(group),
        }
    }

    fn add_show(&self, group: &adw::PreferencesGroup) {
        if !self.subtitle.is_empty() {
            let subtitle = if self.password_mode == PasswordMode::PlainText {
                self.subtitle
            } else {
                "●●●●"
            };
            let e = adw::ActionRow::builder()
                .title(glib::markup_escape_text(self.title))
                .subtitle(glib::markup_escape_text(subtitle))
                // https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/boxed-lists.html#property-rows
                // When used together with the .property style class, AdwActionRow and
                // AdwExpanderRow deemphasize their title and emphasize their subtitle instead
                .css_classes(["property"])
                .build();
            if let Some(i) = self.suffix_icon {
                e.add_suffix(&gtk::Image::builder().icon_name(i).build());
                // e.set_activatable_widget(Some(&e));
            }
            if let Some(a) = self.action.as_ref() {
                e.set_activatable(true);
                let c_a = a.clone();
                e.connect_activated(move |_ar| {
                    c_a();
                });
            }
            group.add(&e);
        }
    }

    fn add_edit(&self, group: &adw::PreferencesGroup) {
        match self.password_mode {
            PasswordMode::PlainText => {
                let e = adw::EntryRow::builder()
                    .title(self.title)
                    .text(self.subtitle)
                    .build();
                // if let Some(i) = self.suffix_icon {
                //     e.add_suffix(&gtk::Image::builder().icon_name(i).build());
                // }
                group.add(&e);
            }
            PasswordMode::Password => {
                let e = adw::PasswordEntryRow::builder()
                    .title(self.title)
                    .text(self.subtitle)
                    .build();
                // if let Some(i) = self.suffix_icon {
                //     e.add_suffix(&gtk::Image::builder().icon_name(i).build());
                // }
                group.add(&e);
            }
        }
    }
}
