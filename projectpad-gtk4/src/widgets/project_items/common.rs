use std::{collections::HashSet, rc::Rc};

use adw::prelude::*;
use diesel::prelude::*;
use gtk::gdk;
use projectpadsql::{models::EnvironmentType, schema};

use crate::{app::ProjectpadApplication, widgets::project_item::WidgetMode};

use super::password_action_row::PasswordActionRow;

#[derive(Clone)]
pub enum EnvOrEnvs {
    Env(EnvironmentType),
    Envs(HashSet<EnvironmentType>),
    None,
}

impl Default for EnvOrEnvs {
    fn default() -> Self {
        EnvOrEnvs::None
    }
}

#[derive(PartialEq, Eq)]
pub enum DialogClamp {
    Yes,
    No,
}

pub fn display_item_edit_dialog(
    v: &gtk::Box,
    title: &str,
    item_box: gtk::Box,
    width: i32,
    height: i32,
    clamp: DialogClamp,
) -> (adw::Dialog, gtk::Button) {
    let cbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let header_bar = adw::HeaderBar::builder()
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();

    let cancel_btn = gtk::Button::builder().label("Cancel").build();
    header_bar.pack_start(&cancel_btn);
    let save_btn = gtk::Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    header_bar.pack_end(&save_btn);
    cbox.append(&header_bar);
    let contents = if clamp == DialogClamp::Yes {
        adw::Clamp::builder()
            .margin_top(10)
            .child(&item_box)
            .build()
            .upcast::<gtk::Widget>()
    } else {
        item_box.upcast::<gtk::Widget>()
    };
    cbox.append(&contents);
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(width)
        .content_height(height)
        .child(&cbox)
        .build();
    let dlg = dialog.clone();
    cancel_btn.connect_clicked(move |_btn: &gtk::Button| {
        dlg.close();
    });
    dialog.present(v);

    (dialog, save_btn)
}

pub fn get_project_group_names(
    sql_conn: &mut diesel::SqliteConnection,
    project_id: i32,
) -> Vec<String> {
    use schema::project_note::dsl as pnote;
    use schema::project_point_of_interest::dsl as ppoi;
    use schema::server::dsl as srv;
    let server_group_names = srv::server
        .filter(
            srv::project_id
                .eq(project_id)
                .and(srv::group_name.is_not_null()),
        )
        .order(srv::group_name.asc())
        .select(srv::group_name)
        .load(sql_conn)
        .unwrap();
    let mut prj_poi_group_names = ppoi::project_point_of_interest
        .filter(
            ppoi::project_id
                .eq(project_id)
                .and(ppoi::group_name.is_not_null()),
        )
        .order(ppoi::group_name.asc())
        .select(ppoi::group_name)
        .load(sql_conn)
        .unwrap();
    let mut prj_note_group_names = pnote::project_note
        .filter(
            pnote::project_id
                .eq(project_id)
                .and(pnote::group_name.is_not_null()),
        )
        .order(pnote::group_name.asc())
        .select(pnote::group_name)
        .load(sql_conn)
        .unwrap();

    let mut project_group_names = server_group_names;
    project_group_names.append(&mut prj_poi_group_names);
    project_group_names.append(&mut prj_note_group_names);
    let mut project_group_names_no_options: Vec<_> = project_group_names
        .into_iter()
        .map(|n: Option<String>| n.unwrap())
        .collect();
    project_group_names_no_options.sort();
    project_group_names_no_options.dedup();
    project_group_names_no_options
}

pub fn ask_user(title: &str, msg: &str, parent: &gtk::Widget, handle_save: Box<dyn Fn(String)>) {
    let contents_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let header_bar = gtk::HeaderBar::builder()
        .title_widget(&gtk::Label::new(Some(title)))
        .show_title_buttons(false)
        .build();
    let cancel_btn = gtk::Button::builder().label("Cancel").build();
    header_bar.pack_start(&cancel_btn);
    let save_btn = gtk::Button::builder()
        .label("Create")
        .css_classes(["suggested-action"])
        .build();
    header_bar.pack_end(&save_btn);
    contents_box.append(&header_bar);
    let item_box = gtk::Box::builder()
        .spacing(10)
        .orientation(gtk::Orientation::Vertical)
        .build();
    item_box.append(
        &gtk::Label::builder()
            .label(msg)
            .halign(gtk::Align::Start)
            .build(),
    );
    let entry = gtk::Entry::builder().hexpand(true).build();
    item_box.append(&entry);
    let contents = adw::Clamp::builder()
        .margin_top(10)
        .margin_start(10)
        .margin_end(10)
        .child(&item_box)
        .build();
    contents_box.append(&contents);
    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(450)
        .content_height(150)
        .child(&contents_box)
        .build();
    dialog.present(parent);
    entry.grab_focus();

    let dlg = dialog.clone();
    cancel_btn.connect_clicked(move |_btn: &gtk::Button| {
        dlg.close();
    });
    let dlg = dialog.clone();
    let e = entry.clone();
    save_btn.connect_clicked(move |_btn: &gtk::Button| {
        handle_save(e.text().to_string());
        dlg.close();
    });
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

pub fn text_row(
    bind_object: &glib::Object,
    bind_key: &str,
    widget_mode: WidgetMode,
    title: &str,
    main_action: Option<SuffixAction>,
    suffix_actions: &[SuffixAction],
) -> adw::PreferencesRow {
    let (par, prop_name) = if widget_mode == WidgetMode::Show {
        let action_row = adw::ActionRow::builder()
            // https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/boxed-lists.html#property-rows
            // When used together with the .property style class, AdwActionRow and
            // AdwExpanderRow deemphasize their title and emphasize their subtitle instead
            .css_classes(["property"])
            .build();

        add_actions(&action_row, main_action, suffix_actions);

        bind_object
            .bind_property(bind_key, &action_row, "visible")
            .transform_to(|_, str: &str| {
                if str.is_empty() {
                    Some(false.to_value())
                } else {
                    Some(true.to_value())
                }
            })
            .sync_create()
            .build();

        (action_row.upcast::<adw::PreferencesRow>(), "subtitle")
    } else {
        (
            adw::EntryRow::builder()
                .build()
                .upcast::<adw::PreferencesRow>(),
            "text",
        )
    };
    par.set_title(title);

    bind_object
        .bind_property(bind_key, &par, prop_name)
        .bidirectional()
        .sync_create()
        .build();

    par
}

pub fn password_row(
    bind_object: &glib::Object,
    bind_key: &str,
    widget_mode: WidgetMode,
    title: &str,
    main_action: Option<SuffixAction>,
    suffix_actions: &[SuffixAction],
) -> adw::PreferencesRow {
    let (par, prop_name) = if widget_mode == WidgetMode::Show {
        let action_row = PasswordActionRow::new();
        add_actions(
            action_row.upcast_ref::<adw::ActionRow>(),
            main_action,
            suffix_actions,
        );

        bind_object
            .bind_property(bind_key, &action_row, "visible")
            .transform_to(|_, str: &str| {
                if str.is_empty() {
                    Some(false.to_value())
                } else {
                    Some(true.to_value())
                }
            })
            .sync_create()
            .build();

        (action_row.upcast::<adw::PreferencesRow>(), "text")
    } else {
        (
            adw::PasswordEntryRow::builder()
                .build()
                .upcast::<adw::PreferencesRow>(),
            "text",
        )
    };
    par.set_title(title);

    bind_object
        .bind_property(bind_key, &par, prop_name)
        .bidirectional()
        .sync_create()
        .build();

    par
}

fn add_actions(
    action_row: &adw::ActionRow,
    main_action: Option<SuffixAction>,
    suffix_actions: &[SuffixAction],
) {
    for suffix in suffix_actions.iter() {
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
        action_row.add_suffix(&widget);
    }

    if let Some(SuffixAction { icon, action }) = main_action.as_ref() {
        action_row.add_suffix(&gtk::Image::builder().icon_name(*icon).build());
        action_row.set_activatable(true);
        let c_a = action.clone();
        action_row.connect_activated(move |_ar| {
            c_a();
        });
    }
}
