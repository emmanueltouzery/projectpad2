use std::{cell::RefCell, rc::Rc};

use adw::prelude::*;
use gtk::gdk;

use crate::{app::ProjectpadApplication, keyring_helpers, widgets::project_items::common};

pub fn display_unlock_dialog(is_new_db: bool) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let header_bar = adw::HeaderBar::builder()
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();

    let quit_btn = gtk::Button::builder().label("Quit").build();
    header_bar.pack_start(&quit_btn);
    quit_btn.connect_clicked(|_| {
        gio::Application::default().unwrap().quit();
    });

    let next_btn = gtk::Button::builder()
        .label(if is_new_db { "Start" } else { "Unlock" })
        .css_classes(["suggested-action"])
        .receives_default(true)
        .build();
    header_bar.pack_end(&next_btn);

    vbox.append(&header_bar);

    let contents_hbox = gtk::Box::builder()
        .spacing(10)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .build();

    contents_hbox.append(
        &gtk::Image::builder()
            .icon_name("lock-symbolic")
            .pixel_size(64)
            .build(),
    );

    let contents_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(10)
        .hexpand(true)
        .margin_start(20)
        .margin_end(20)
        .build();

    contents_box.append(&gtk::Label::builder().label(if is_new_db {
        "Projectpad needs a password to encrypt your database, please enter one to continue."
    } else {
        "Please enter the database password"
    }).halign(gtk::Align::Start).build());

    let prefs_group = adw::PreferencesGroup::builder().build();
    let password = adw::PasswordEntryRow::builder().title("Password").build();
    let nb = next_btn.clone();

    // it sounds crazy that i have to do that to get it to activate
    // the default button when the user presses enter but...
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    controller.connect_key_pressed(move |_, keyval, _, _| {
        if keyval == gdk::Key::Return {
            nb.emit_clicked();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });

    password.add_controller(controller);

    prefs_group.add(&password);

    let password_confirm = if is_new_db {
        let password_confirm = adw::PasswordEntryRow::builder()
            .title("Password confirm")
            .build();
        prefs_group.add(&password_confirm);
        Some(password_confirm)
    } else {
        None
    };

    let save_password_check = adw::SwitchRow::builder()
        .title("Save password to OS keyring")
        .build();
    prefs_group.add(&save_password_check);

    contents_box.append(&prefs_group);

    contents_hbox.append(&contents_box);

    vbox.append(&contents_hbox);

    let dialog = adw::Dialog::builder()
        .title("Projectpad")
        .content_width(550)
        .child(&vbox)
        .build();

    let closed_handler_id = Rc::new(RefCell::new(Some(dialog.connect_closed(|_| {
        gio::Application::default().unwrap().quit();
    }))));

    dialog.set_focus(Some(&password));
    dialog.set_default_widget(Some(&next_btn));

    let d = dialog.clone();
    next_btn.connect_clicked(move |_| {
        if password.text().is_empty() {
            // prevent empty passwords in projectpad2, because moving between
            // plaintext and encrypted sqlcipher databases is non-trivial
            // https://www.zetetic.net/sqlcipher/sqlcipher-api/index.html#sqlcipher_export
            common::simple_error_dlg("Unlock error", Some("The password must not be empty"));
            return;
        }
        if is_new_db && password.text() != password_confirm.as_ref().unwrap().text() {
            common::simple_error_dlg("Password issue", Some("Passwords don't match"));
            return;
        }
        let p = password.text();
        let is_save_to_keyring = save_password_check.is_active();
        let receiver = common::run_sqlfunc(Box::new(move |sql_conn| {
            projectpadsql::try_unlock_db(sql_conn, &p)
        }));
        let p = password.text();
        let d = d.clone();
        let i = closed_handler_id.clone();
        glib::spawn_future_local(async move {
            let res = receiver.recv().await.unwrap();
            match res {
                Err(msg) => common::simple_error_dlg("Error checking the password", Some(&msg)),
                Ok(_) if is_save_to_keyring => {
                    if let Err(msg) = keyring_helpers::set_pass_in_keyring(&p) {
                        common::simple_error_dlg(
                            "Error saving the password to the keyring",
                            Some(&msg),
                        );
                    }
                    if let Some(sig) = i.borrow_mut().take() {
                        d.disconnect(sig);
                    }
                    d.close();
                    ProjectpadApplication::load_app_after_unlock();
                }
                Ok(_) => {
                    if let Some(sig) = i.borrow_mut().take() {
                        d.disconnect(sig);
                    }
                    d.close();
                    ProjectpadApplication::load_app_after_unlock();
                }
            }
        });
    });

    dialog.present(Some(&common::main_win()));
}
