use std::{collections::HashSet, rc::Rc};

use adw::prelude::*;
use async_channel::Receiver;
use diesel::prelude::*;
use gtk::gdk;
use projectpadsql::models::EnvironmentType;

use crate::{
    app::{self, ProjectpadApplication},
    sql_thread::SqlFunc,
    widgets::project_item::WidgetMode,
};

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

        let toast_overlay = app::get().get_toast_overlay();
        toast_overlay.add_toast(adw::Toast::new("Copied to the clipboard"));
    }
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

pub fn combo_row<
    FF: Fn(glib::Value) -> u32 + Send + 'static + Copy + Sync,
    FT: Fn(u32) -> glib::Value + Send + 'static + Copy + Sync,
>(
    bind_object: &glib::Object,
    bind_key: &str,
    widget_mode: WidgetMode,
    title: &str,
    combo_vals: &[&str],
    bind_from: FF,
    bind_to: FT,
) -> adw::PreferencesRow {
    if widget_mode == WidgetMode::Edit {
        // server type
        let combo_row = adw::ComboRow::new();
        combo_row.set_title(title);
        let vals_model = gtk::StringList::new(combo_vals);
        combo_row.set_model(Some(&vals_model));
        // the binding is from the bind object so that its value is initially
        // used for the sync
        bind_object
            .bind_property(bind_key, &combo_row, "selected")
            .transform_to(move |_, val| Some(bind_from(val)))
            .transform_from(move |_, number: u32| Some(bind_to(number)))
            .bidirectional()
            .sync_create()
            .build();
        combo_row.upcast::<adw::PreferencesRow>()
    } else {
        let val_index = bind_from(bind_object.property(bind_key));
        adw::ActionRow::builder()
            // https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/boxed-lists.html#property-rows
            // When used together with the .property style class, AdwActionRow and
            // AdwExpanderRow deemphasize their title and emphasize their subtitle instead
            .css_classes(["property"])
            .title(title)
            .subtitle(combo_vals[TryInto::<usize>::try_into(val_index).unwrap()])
            .build()
            .upcast::<adw::PreferencesRow>()
    }
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

pub fn simple_error_dlg(title: &str, details: Option<&str>) {
    let dialog = adw::AlertDialog::new(Some(title), details);
    dialog.add_responses(&[("close", "_Close")]);
    dialog.set_default_response(Some("close"));
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    dialog.present(&app.active_window().unwrap());
}

pub fn confirm_delete(title: &str, msg: &str, delete_fn: Box<dyn Fn() + Send + 'static>) {
    let dialog = adw::AlertDialog::new(Some(title), Some(msg));
    dialog.add_responses(&[("cancel", "_Cancel"), ("delete", "_Delete")]);
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));

    dialog.connect_response(None, move |_dlg, resp| {
        if resp == "delete" {
            delete_fn();
        }
    });
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    dialog.present(&app.active_window().unwrap());
}

pub fn run_sqlfunc<T: Sync + Send + 'static>(
    sql_fn: Box<dyn Fn(&mut SqliteConnection) -> T + Send + 'static>,
) -> Receiver<T> {
    let (sender, receiver) = async_channel::bounded(1);
    let app = gio::Application::default()
        .and_downcast::<ProjectpadApplication>()
        .unwrap();
    let db_sender = app.get_sql_channel();
    db_sender
        .send(SqlFunc::new(move |sql_conn| {
            sender.send_blocking(sql_fn(sql_conn)).unwrap();
        }))
        .unwrap();
    receiver
}

pub fn run_sqlfunc_and_then<T: Sync + Send + 'static>(
    sql_fn: Box<dyn Fn(&mut SqliteConnection) -> T + Send + 'static>,
    after: Box<dyn Fn(T) + Send + 'static>,
) {
    let receiver = run_sqlfunc(sql_fn);
    glib::spawn_future_local(async move {
        let val = receiver.recv().await.unwrap();
        after(val);
    });
}
