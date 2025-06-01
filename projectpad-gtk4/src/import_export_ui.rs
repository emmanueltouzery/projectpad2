use itertools::Itertools;
use std::{ffi::OsStr, path::PathBuf};

use adw::prelude::*;
use diesel::prelude::*;
use projectpadsql::models::Project;

use crate::{
    app::RunMode,
    export, import,
    widgets::{
        project_item::WidgetMode,
        project_items::{
            common,
            file_picker_action_row::{FilePickerActionRow, UpdateFilenameProp},
        },
    },
};

pub fn open_import_export_dlg() {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let header_bar = adw::HeaderBar::builder()
        .show_end_title_buttons(false)
        .show_start_title_buttons(false)
        .build();

    let cancel_btn = gtk::Button::builder().label("Cancel").build();
    header_bar.pack_start(&cancel_btn);

    let next_btn = gtk::Button::builder()
        .label("Next")
        .css_classes(["suggested-action"])
        .build();
    header_bar.pack_end(&next_btn);

    vbox.append(&header_bar);

    let stack = gtk::Stack::new();

    let imp_exp_tab = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .spacing(10)
        .build();

    imp_exp_tab.append(
        &gtk::Label::builder()
            .label(
                "You can export any project to a single data file. The file can then be \
                shared. The exported file is an encrypted 7zip file which can be either \
                imported back in another projectpad instance, or used directly by the \
                recipient as a textual description of the exported project. The \
                7zip contains a human-readable YAML file.",
            )
            .wrap(true)
            .build(),
    );

    let import_radio = gtk::CheckButton::builder()
        .label("Import")
        .active(true)
        .build();
    imp_exp_tab.append(&import_radio);

    let export_radio = gtk::CheckButton::builder()
        .group(&import_radio)
        .label("Export")
        .build();
    imp_exp_tab.append(&export_radio);

    stack.add_child(&imp_exp_tab);

    vbox.append(&stack);

    let dialog = adw::Dialog::builder()
        .title("Import/Export")
        .content_width(550)
        .child(&vbox)
        .build();

    dialog.present(&common::main_win());

    let dlg = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dlg.close();
    });

    next_btn.connect_clicked(move |btn| {
        if export_radio.is_active() {
            btn.set_visible(false);
            let export_btn = gtk::Button::builder()
                .label("Export")
                .css_classes(["suggested-action"])
                .build();
            header_bar.pack_end(&export_btn);
            switch_export_tab(&dialog, &stack, export_btn);
        } else if import_radio.is_active() {
            btn.set_visible(false);
            let import_btn = gtk::Button::builder()
                .label("Import")
                .css_classes(["suggested-action"])
                .build();
            header_bar.pack_end(&import_btn);
            switch_import_tab(&dialog, &stack, import_btn);
        }
    });
}

fn switch_export_tab(dialog: &adw::Dialog, stack: &gtk::Stack, next: gtk::Button) {
    let import_tab_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .spacing(10)
        .build();

    let select_all_btn = gtk::Button::builder().label("Select all").build();
    let projects_group = adw::PreferencesGroup::builder()
        .title("Projects to export")
        .header_suffix(&select_all_btn)
        .build();
    import_tab_box.append(&projects_group);

    // two preferences groups, because i don't want the title to scroll,
    // i only want the actual list to scroll. So one group for the title
    // and one for the contents, within a scrolled window.
    let projects_group2 = adw::PreferencesGroup::builder().build();

    let projects_scroll = gtk::ScrolledWindow::builder()
        .child(&projects_group2)
        .height_request(250)
        .build();

    import_tab_box.append(&projects_scroll);

    let projects_recv = common::run_sqlfunc(Box::new(|sql_conn| {
        use projectpadsql::schema::project::dsl as prj;
        prj::project
            .order(prj::name.asc())
            .load::<Project>(sql_conn)
    }));

    let pass1 = adw::PasswordEntryRow::builder().title("Password").build();
    let pass2 = adw::PasswordEntryRow::builder()
        .title("Password confirm")
        .build();

    let password_group = adw::PreferencesGroup::builder().title("Password").build();
    password_group.add(&pass1);
    password_group.add(&pass2);
    import_tab_box.append(&password_group);

    stack.add_child(&import_tab_box);
    stack.set_visible_child(&import_tab_box);

    let target_file_group = adw::PreferencesGroup::builder()
        .title("Export to file...")
        .build();
    let file_picker_row =
        FilePickerActionRow::new_ext(WidgetMode::Show, UpdateFilenameProp::Always);
    target_file_group.add(&file_picker_row);
    import_tab_box.append(&target_file_group);

    let d = dialog.clone();
    glib::spawn_future_local(async move {
        let projects = projects_recv.recv().await.unwrap().unwrap();
        let mut project_rows = vec![];
        for project in projects.iter() {
            let project_row = adw::SwitchRow::builder().title(&project.name).build();
            projects_group2.add(&project_row);
            project_rows.push(project_row);
        }
        let prs = project_rows.clone();
        select_all_btn.connect_clicked(move |_| {
            for pr in prs.iter() {
                pr.set_active(true);
            }
        });
        let dlg = d.clone();
        next.connect_clicked(move |_| {
            if pass1.text() != pass2.text() {
                common::simple_error_dlg(
                    "Export failed",
                    Some("New and confirm passwords don't match"),
                );
                return;
            }
            let mut selected_projects = vec![];
            for (idx, project_row) in project_rows.iter().enumerate() {
                if project_row.is_active() {
                    selected_projects.push(projects.get(idx).unwrap().clone());
                }
            }
            if selected_projects.is_empty() {
                common::simple_error_dlg(
                    "Export failed",
                    Some("No projects were selected for export"),
                );
            }
            match PathBuf::from(&file_picker_row.filename()) {
                pb if pb.as_os_str().is_empty() => {
                    common::simple_error_dlg(
                        "Export failed",
                        Some("Must pick a file to export to"),
                    );
                }

                pb if pb.extension() == Some(OsStr::new("7z")) => {
                    do_export(&dlg, pb, selected_projects, pass1.text().to_string());
                }

                // need to make sure the user picks a filename ending in .7z, or we get
                // a subtle issue in the flatpak: when you enter filename /a/b/c in the
                // file picker, flatpak gives us access to /a/b/c and NOTHING ELSE.
                // attempting to write to /a/b/c.7z will fail, and we do want to have
                // the extension...
                _ => common::simple_error_dlg(
                    "Export file",
                    Some("Please pick a file to save to ending with .7z"),
                ),
            }
        });
    });
}

fn do_export(
    dialog: &adw::Dialog,
    target_fname: PathBuf,
    selected_projects: Vec<Project>,
    password: String,
) {
    let recv = common::run_sqlfunc(Box::new(move |sql_conn| {
        export::export_projects(sql_conn, &selected_projects, &target_fname, &password)
            .map_err(|e| e.to_string())
    }));
    let dlg = dialog.clone();
    glib::spawn_future_local(async move {
        let res_missing_dep_project_names = recv.recv().await.unwrap();
        match res_missing_dep_project_names {
            Err(e) => {
                common::simple_error_dlg("Export error", Some(&e));
            }
            Ok(missing_dep_project_names) if !missing_dep_project_names.is_empty() => {
                common::simple_error_dlg(
                    "Export warning",
                    Some(&format!(
                        "Some dependent projects were not exported: {}",
                        missing_dep_project_names.iter().join(", ")
                    )),
                );
            }
            Ok(_) => {
                // TODO could open a dialog confirming the export was done, with
                // a link to the export folder (same as we have in the preferences for
                // the config file)
                common::app()
                    .get_toast_overlay()
                    .add_toast(adw::Toast::new("Export successfully performed"));
                dlg.close();
            }
        }
    });
}

fn switch_import_tab(dialog: &adw::Dialog, stack: &gtk::Stack, next: gtk::Button) {
    let import_tab_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .spacing(10)
        .build();

    let prefs_group = adw::PreferencesGroup::builder()
        .title("Import file and password")
        .build();

    // TODO filename filter for picker... *.7z only
    let file_picker_row = FilePickerActionRow::new(WidgetMode::Edit);

    prefs_group.add(&file_picker_row);

    let pass = adw::PasswordEntryRow::builder().title("Password").build();
    prefs_group.add(&pass);

    import_tab_box.append(&prefs_group);

    stack.add_child(&import_tab_box);
    stack.set_visible_child(&import_tab_box);

    let dlg = dialog.clone();
    next.connect_clicked(move |_| match PathBuf::from(&file_picker_row.filename()) {
        pb if pb.as_os_str().is_empty() => {
            common::simple_error_dlg("Import failed", Some("Must pick a file to import from"));
        }

        pb => {
            let pass_txt = pass.text();
            let import_recv = common::run_sqlfunc(Box::new(move |sql_conn| {
                sql_conn
                    .transaction(|sql_conn| {
                        import::do_import(sql_conn, &pb.to_string_lossy(), &pass_txt)
                    })
                    .map_err(|e| e.to_string())
            }));
            let dlg = dlg.clone();
            glib::spawn_future_local(async move {
                let import_res = import_recv.recv().await.unwrap();
                match import_res {
                    Ok(_) => {
                        let app = common::app();
                        app.fetch_projects_and_populate_menu(
                            RunMode::Normal,
                            &app.get_sql_channel(),
                        );
                        app.get_toast_overlay()
                            .add_toast(adw::Toast::new("Import successfully performed"));
                        dlg.close();
                    }
                    Err(e) => {
                        common::simple_error_dlg("Import failed", Some(&e));
                    }
                }
            });
        }
    });
}
