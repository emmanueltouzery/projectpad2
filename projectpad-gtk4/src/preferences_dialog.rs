use adw::prelude::*;
use diesel::prelude::*;
use gtk::gdk;

use crate::{keyring_helpers, widgets::project_items::common};

pub fn display_preferences_dialog() {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let header_bar = adw::HeaderBar::builder()
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();

    let close_btn = gtk::Button::builder().label("Close").build();
    header_bar.pack_end(&close_btn);

    vbox.append(&header_bar);

    let contents_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(10)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .build();

    let database_group = adw::PreferencesGroup::builder().title("Database").build();

    let db_pathbuf = projectpadsql::database_path();
    let db_path = db_pathbuf.to_string_lossy().into_owned();
    let db_location_row = adw::ActionRow::builder()
        .title("Database file location")
        .subtitle(&db_path)
        .subtitle_lines(1)
        .build();

    let copy_db_location = gtk::Button::builder()
        .css_classes(["flat"])
        .icon_name("edit-copy-symbolic")
        .build();
    let dbp = db_path.clone();
    copy_db_location.connect_clicked(move |_| copy_db_location_to_clipboard(&dbp));
    db_location_row.add_suffix(&copy_db_location);

    let open_db_location = gtk::Button::builder()
        .css_classes(["flat"])
        .icon_name("document-open-symbolic")
        .build();
    open_db_location.connect_clicked(|_| {
        let db_folder_pathbuf = projectpadsql::config_path();
        let folder_file = gio::File::for_path(&db_folder_pathbuf);
        let launcher = gtk::FileLauncher::new(Some(&folder_file));
        launcher.launch(
            common::app().active_window().as_ref(),
            None::<&gio::Cancellable>,
            |r| {
                if let Err(e) = r {
                    common::app()
                        .get_toast_overlay()
                        .add_toast(adw::Toast::new(&format!("Error opening db folder: {e}")))
                }
            },
        );
    });
    db_location_row.add_suffix(&open_db_location);

    db_location_row.set_activatable(true);
    let dbp = db_path.clone();
    db_location_row.connect_activated(move |_| copy_db_location_to_clipboard(&dbp));
    database_group.add(&db_location_row);

    let change_pass_row = adw::ButtonRow::builder().title("Change password").build();
    // TODO
    database_group.add(&change_pass_row);

    let remove_pass_row = adw::ButtonRow::builder()
        .title("Remove password from keyring")
        .css_classes(["button", "destructive-action"])
        .build();
    remove_pass_row.connect_activated(|_| {
        display_dialog_confirm_remove_from_keyring("");
    });
    database_group.add(&remove_pass_row);

    contents_box.append(&database_group);

    vbox.append(&contents_box);

    let dialog = adw::Dialog::builder()
        .title("Preferences")
        .content_width(450)
        .child(&vbox)
        .build();

    let dlg = dialog.clone();
    close_btn.connect_clicked(move |_| {
        dlg.close();
    });

    dialog.present(Some(&common::main_win()));
}

fn display_dialog_confirm_remove_from_keyring(pass_confirm_text: &str) {
    let pass_confirm = gtk::PasswordEntry::builder()
        .text(pass_confirm_text)
        .show_peek_icon(true)
        .build();

    let dialog = adw::AlertDialog::builder()
        .heading("Remove password from the keyring?")
        .body("Enter the password to confirm that you wish to remove it from the OS keyring")
        .extra_child(&pass_confirm)
        .build();

    // it sounds crazy that i have to do that to get it to activate
    // the default button when the user presses enter but...
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let dlg = dialog.clone();
    let pc = pass_confirm.clone();
    controller.connect_key_pressed(move |_, keyval, _, _| {
        if keyval == gdk::Key::Return {
            dlg.close();
            remove_password_from_keyring_proceed(&pc.text());
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    pass_confirm.add_controller(controller);

    dialog.add_responses(&[("cancel", "_Cancel"), ("remove", "_Remove")]);
    dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));

    dialog.set_focus(Some(&pass_confirm));

    dialog.connect_response(None, move |_dlg, resp| {
        if resp == "remove" {
            remove_password_from_keyring_proceed(&pass_confirm.text());
        }
    });
    dialog.present(Some(&common::main_win()));
}

fn remove_password_from_keyring_proceed(pass_confirm_text: &str) {
    if check_db_password(pass_confirm_text) {
        if let Err(e) = keyring_helpers::clear_pass_from_keyring() {
            common::simple_error_dlg("Removal from keyring failed", Some(&e.to_string()));
        }
    } else {
        let pct = pass_confirm_text.to_owned();
        common::simple_error_dlg_callback(
            "Removal from keyring failed",
            Some("Wrong password"),
            move |_, _| {
                // display the dialog again, with the password the user entered
                display_dialog_confirm_remove_from_keyring(&pct);
            },
        );
    }
}

fn check_db_password(pass: &str) -> bool {
    let mut db_conn =
        SqliteConnection::establish(&projectpadsql::database_path().to_string_lossy()).unwrap();
    projectpadsql::try_unlock_db(&mut db_conn, pass).is_ok()
}

fn copy_db_location_to_clipboard(db_location: &str) {
    common::copy_to_clipboard(db_location);
    common::app()
        .get_toast_overlay()
        .add_toast(adw::Toast::new("Copied the database location to clipboard"));
}
