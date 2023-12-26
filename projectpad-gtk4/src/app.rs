use std::sync::mpsc;

use adw::subclass::prelude::*;
use gio::subclass::prelude::ApplicationImpl;
use glib::{clone, ObjectExt, Properties, Receiver, Sender};
use gtk::subclass::prelude::DerivedObjectProperties;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib};
use gtk::{prelude::*, CssProvider};

use crate::sql_thread::SqlFunc;
use crate::{keyring_helpers, ProjectpadApplicationWindow};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use glib::{
        subclass::{prelude::ObjectImpl, types::ObjectSubclass},
        WeakRef,
    };
    use gtk::subclass::prelude::GtkApplicationImpl;

    use crate::ProjectpadApplicationWindow;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ProjectpadApplication)]
    pub struct ProjectpadApplication {
        #[property(get)]
        pub rb_server: RefCell<Option<String>>, // TODO remove
        //
        pub window: OnceCell<WeakRef<ProjectpadApplicationWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectpadApplication {
        const NAME: &'static str = "ProjectpadApplication";
        type ParentType = adw::Application;
        type Type = super::ProjectpadApplication;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProjectpadApplication {}

    impl ApplicationImpl for ProjectpadApplication {
        fn activate(&self) {
            let app = self.obj();
            let window = app.create_window();
            let _ = self.window.set(window.downgrade());
        }
    }

    impl GtkApplicationImpl for ProjectpadApplication {}

    impl AdwApplicationImpl for ProjectpadApplication {}
}

glib::wrapper! {
    pub struct ProjectpadApplication(ObjectSubclass<imp::ProjectpadApplication>)
        @extends gio::Application, gtk::Application, adw::Application;
        // @implements gio::ActionMap, gio::ActionGroup;
}

impl ProjectpadApplication {
    pub fn run(sql_channel: mpsc::Sender<SqlFunc>, is_new_db: bool) -> glib::ExitCode {
        // Create new GObject and downcast it into SwApplication
        let app = glib::Object::builder::<ProjectpadApplication>()
            // .property("application-id", Some(config::APP_ID))
            // .property("flags", gio::ApplicationFlags::empty())
            // .property("resource-base-path", Some(config::PATH_ID))
            .build();

        app.connect_startup(|_| Self::load_css());

        Self::unlock_db(&sql_channel);

        // Start running gtk::Application
        app.run()
    }

    fn unlock_db(sql_channel: &mpsc::Sender<SqlFunc>) {
        if let Some(pass) = keyring_helpers::get_pass_from_keyring() {
            // https://gtk-rs.org/gtk4-rs/stable/latest/book/main_event_loop.html
            // Create channel that can hold at most 1 message at a time
            let (sender, receiver) = async_channel::bounded(1);
            sql_channel
                .send(SqlFunc::new(move |sql_conn| {
                    let unlock_success = projectpadsql::try_unlock_db(sql_conn, &pass).is_ok();
                    sender.send_blocking(unlock_success).unwrap();
                }))
                .unwrap();

            // The main loop executes the asynchronous block
            glib::spawn_future_local(async move {
                let unlock_success = receiver.recv().await.unwrap();
                if unlock_success {
                    // TODO run_prepare_db
                    // TODO request_update_welcome_status
                } else {
                    // self.display_unlock_dialog();
                }
            });
        } else {
            // self.display_unlock_dialog();
        }
    }

    fn load_css() {
        // https://developer.gnome.org/documentation/tutorials/themed-icons.html
        // https://docs.elementary.io/develop/apis/gresource
        gtk::IconTheme::for_display(
            &gdk::Display::default().expect("Could not connect to a display."),
        )
        .add_resource_path("/icons");
        // Load the CSS file and add it to the provider
        let provider = CssProvider::new();
        provider.load_from_data(include_str!("style.css"));

        // Add the provider to the default screen
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    fn create_window(&self) -> ProjectpadApplicationWindow {
        let window = ProjectpadApplicationWindow::new();
        self.add_window(&window);

        window.present();
        window
    }
}

impl Default for ProjectpadApplication {
    fn default() -> Self {
        gio::Application::default()
            .expect("Could not get default GApplication")
            .downcast()
            .unwrap()
    }
}
