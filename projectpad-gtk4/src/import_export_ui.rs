use adw::prelude::*;
use diesel::{prelude::*, select};
use glib::SignalHandlerId;
use projectpadsql::models::Project;

use crate::widgets::project_items::common;

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
            switch_export_tab(&stack, export_btn);
        }
    });
}

fn switch_export_tab(stack: &gtk::Stack, next: gtk::Button) {
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
        .height_request(300)
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
        next.connect_clicked(move |_| {
            if pass1.text() != pass2.text() {
                common::simple_error_dlg(
                    "Export failed",
                    Some("New and confirm passwords don't match"),
                );
                return;
            }
            let mut selected_projects = vec![];
            let mut idx = 0;
            for project_row in project_rows.iter() {
                if project_row.is_active() {
                    selected_projects.push(projects.get(idx).unwrap().id);
                }
                idx += 1;
            }
            if selected_projects.is_empty() {
                common::simple_error_dlg(
                    "Export failed",
                    Some("No projects were selected for export"),
                );
            }
        });
    });
}
