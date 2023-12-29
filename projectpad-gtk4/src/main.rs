use std::{panic, process};

use app::ProjectpadApplication;
use gtk::glib;
use widgets::project_item_list::ProjectItemList;
mod widgets;
use adw::subclass::prelude::*;
use glib::subclass;
use gtk::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

mod app;
mod keyring_helpers;
mod sql_thread;

mod imp {
    use crate::widgets::project_item::ProjectItem;

    use super::*;
    use gtk::{
        subclass::{
            prelude::{ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/emmanueltouzery/projectpad2/src/gtk_builder.ui")]
    pub struct ProjectpadApplicationWindow {
        #[template_child]
        pub project_item_list: TemplateChild<ProjectItemList>,
        #[template_child]
        pub project_item: TemplateChild<ProjectItem>,
        #[template_child]
        pub edit_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub project_popover_menu: TemplateChild<gtk::PopoverMenu>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectpadApplicationWindow {
        const NAME: &'static str = "ProjectpadApplicationWindow";
        type ParentType = adw::ApplicationWindow;
        type Type = super::ProjectpadApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProjectpadApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();

            self.project_item_list.get().set_project_items();
            // let app = ProjectpadApplication::default();
            // let sender = app.imp().sender.clone();
            // let player = app.imp().player.clone();

            // self.obj().setup_widgets(sender.clone(), player);
            // self.obj().setup_gactions(sender);
            self.edit_btn
                .bind_property("active", &self.project_item.get(), "edit_mode")
                .build();
        }
    }

    impl WidgetImpl for ProjectpadApplicationWindow {}

    impl WindowImpl for ProjectpadApplicationWindow {
        // fn close_request(&self) -> glib::Propagation {
        // debug!("Saving window geometry.");
        // let width = self.obj().default_size().0;
        // let height = self.obj().default_size().1;

        // settings_manager::set_integer(Key::WindowWidth, width);
        // settings_manager::set_integer(Key::WindowHeight, height);
        // glib::Propagation::Proceed
        // }
    }

    impl ApplicationWindowImpl for ProjectpadApplicationWindow {}

    impl AdwApplicationWindowImpl for ProjectpadApplicationWindow {}

    impl ProjectpadApplicationWindow {}
}

glib::wrapper! {
    pub struct ProjectpadApplicationWindow(
        ObjectSubclass<imp::ProjectpadApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup;
}

// TODO split the window in a separate win.rs file?
// currently app.rs is using main.rs which is dubious.
impl ProjectpadApplicationWindow {
    pub fn new() -> Self {
        let win = glib::Object::new::<Self>();
        win.imp().project_item_list.connect_activate(
            glib::clone!(@weak win as w => move |project_item_id| {
                w.imp().project_item.set_item_id(project_item_id)
            }),
        );
        win
    }
}

fn main() -> glib::ExitCode {
    let res_bytes = include_bytes!("resources.bin");
    let data = glib::Bytes::from(&res_bytes[..]);
    let resource = gio::Resource::from_data(&data).unwrap();
    gio::resources_register(&resource);

    // https://stackoverflow.com/a/36031130/516188
    // close the app if we panic in the sql thread
    // instead of having that thread silently terminated
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        process::exit(1);
    }));

    let db_path = projectpadsql::database_path();

    // See https://github.com/emmanueltouzery/projectpad2/issues/1
    // if you start the app, and close the login screen without
    // unlocking the DB, we leave a DB file of zero bytes, and at
    // next startup we ask you for the unlock password, we don't
    // anymore ask you for a confirm password, because we assume
    // there's already a DB around => check that the db file is
    // present AND not empty.
    // if reading the file length fails, assume a non-empty file.
    let db_preexisted = db_path.is_file()
        && std::fs::metadata(db_path)
            .map(|m| m.len())
            .unwrap_or_else(|e| {
                eprintln!("Failed reading file metadata? {:?}", e);
                1
            })
            > 0;

    let sql_channel = sql_thread::start_sql_thread();

    ProjectpadApplication::run(sql_channel, !db_preexisted)
}
