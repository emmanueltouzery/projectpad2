use adw::prelude::*;

use crate::widgets::project_items::common::{self, copy_to_clipboard};

pub fn display_copy_server_password_dialog(
    srv_pass: &str,
    www_passes: &[(String, String)],
    db_passes: &[(String, String)],
    usr_passes: &[(String, String)],
) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let header_bar = adw::HeaderBar::builder().build();

    vbox.append(&header_bar);

    let contents_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(10)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .build();

    let prefs_group = adw::PreferencesGroup::builder()
        .title("Password to copy")
        .build();
    contents_box.append(&prefs_group);

    let dialog = adw::Dialog::builder()
        .title("Copy server password")
        .content_width(450)
        .child(&vbox)
        .build();

    if !srv_pass.is_empty() {
        add_password_copy_btn(&dialog, &prefs_group, "server", "Server password", srv_pass);
    }
    for (desc, pass) in www_passes {
        add_password_copy_btn(
            &dialog,
            &prefs_group,
            "globe",
            &format!("Website {desc}"),
            pass,
        );
    }
    for (desc, pass) in db_passes {
        add_password_copy_btn(
            &dialog,
            &prefs_group,
            "database",
            &format!("Database {desc}"),
            pass,
        );
    }
    for (desc, pass) in usr_passes {
        add_password_copy_btn(
            &dialog,
            &prefs_group,
            "user",
            &format!("Extra user {desc}"),
            pass,
        );
    }

    vbox.append(&contents_box);

    dialog.present(Some(&common::main_win()));
}

fn add_password_copy_btn(
    dlg: &adw::Dialog,
    prefs_group: &adw::PreferencesGroup,
    icon: &str,
    pass_desc: &str,
    pass: &str,
) {
    let password_row = adw::ActionRow::builder()
        .title(pass_desc)
        .activatable(true)
        .build();
    password_row.add_prefix(&gtk::Image::builder().icon_name(icon).build());

    let copy_pass = gtk::Button::builder()
        .css_classes(["flat"])
        .icon_name("edit-copy-symbolic")
        .build();
    let p = pass.to_owned();
    let dlg_ = dlg.clone();
    copy_pass.connect_clicked(move |_| {
        copy_to_clipboard(&p);
        dlg_.close();
    });
    password_row.add_suffix(&copy_pass);

    let p = pass.to_owned();
    let dlg_ = dlg.clone();
    password_row.connect_activated(move |_| {
        copy_to_clipboard(&p);
        dlg_.close();
    });
    prefs_group.add(&password_row);
}
