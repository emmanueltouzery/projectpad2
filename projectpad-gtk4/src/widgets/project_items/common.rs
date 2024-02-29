use std::{collections::HashSet, rc::Rc};

use adw::prelude::*;
use gtk::gdk;
use projectpadsql::models::EnvironmentType;

use crate::{
    app::ProjectpadApplication,
    widgets::{
        environment_list_picker::EnvironmentListPicker, environment_picker::EnvironmentPicker,
        project_item::WidgetMode,
    },
};

pub enum EnvOrEnvs {
    Env(EnvironmentType),
    Envs(HashSet<EnvironmentType>),
    None,
}

pub fn get_contents_box_with_header(
    title: &str,
    env: EnvOrEnvs,
    widget_mode: WidgetMode,
) -> gtk::Box {
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
            .halign(gtk::Align::Fill)
            .hexpand(true)
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
        let environment_picker = match env {
            EnvOrEnvs::Env(e) => Some(EnvironmentPicker::new(e).upcast::<gtk::Widget>()),
            EnvOrEnvs::Envs(es) => Some(EnvironmentListPicker::new(es).upcast::<gtk::Widget>()),
            EnvOrEnvs::None => None,
        };
        if let Some(ep) = environment_picker {
            ep.set_halign(gtk::Align::End);
            ep.set_hexpand(true);
            header_box.append(&ep);
        }
        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .css_classes(["destructive-action"])
            .valign(gtk::Align::Center)
            .halign(gtk::Align::End)
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
    pub password_mode: PasswordMode,
    pub main_action: Option<SuffixAction>,
    pub suffix_actions: &'a [SuffixAction],
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

    pub fn link(url: &str) -> SuffixAction {
        let u = url.to_owned();
        SuffixAction {
            icon: "external-link-alt-symbolic",
            action: Rc::new(Box::new(move || {
                gtk::UriLauncher::new(&u).launch(
                    None::<&gtk::Window>,
                    None::<&gio::Cancellable>,
                    |_| {},
                );
            })),
        }
    }
}

impl DetailsRow<'_> {
    pub fn new<'a>(
        title: &'a str,
        subtitle: &'a str,
        main_action: Option<SuffixAction>,
        suffix_actions: &'a [SuffixAction],
    ) -> DetailsRow<'a> {
        DetailsRow {
            title,
            subtitle,
            password_mode: PasswordMode::PlainText,
            main_action,
            suffix_actions,
        }
    }

    pub fn new_password<'a>(
        title: &'a str,
        subtitle: &'a str,
        main_action: Option<SuffixAction>,
    ) -> DetailsRow<'a> {
        DetailsRow {
            title,
            subtitle,
            password_mode: PasswordMode::Password,
            main_action,
            suffix_actions: &[],
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
            for suffix in self.suffix_actions.iter() {
                let widget = gtk::Button::builder()
                    .css_classes(["flat"])
                    .icon_name(suffix.icon)
                    .build();
                let a = suffix.action.clone();
                widget.connect_closure(
                    "clicked",
                    false,
                    glib::closure_local!(|_b: gtk::Button| {
                        a();
                    }),
                );
                e.add_suffix(&widget);
            }
            if self.password_mode == PasswordMode::Password {
                let widget = gtk::Button::builder()
                    .css_classes(["flat"])
                    .icon_name("view-reveal-symbolic")
                    .build();
                let ar = e.clone();
                let st = self.subtitle.to_owned();
                widget.connect_closure(
                    "clicked",
                    false,
                    glib::closure_local!(|b: gtk::Button| {
                        if b.icon_name() == Some("view-reveal-symbolic".into()) {
                            ar.set_subtitle(&st);
                            b.set_icon_name("view-conceal-symbolic");
                        } else {
                            ar.set_subtitle("●●●●");
                            b.set_icon_name("view-reveal-symbolic");
                        }
                    }),
                );
                e.add_suffix(&widget);
            }
            if let Some(SuffixAction { icon, action }) = self.main_action.as_ref() {
                e.add_suffix(&gtk::Image::builder().icon_name(*icon).build());
                e.set_activatable(true);
                let c_a = action.clone();
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
