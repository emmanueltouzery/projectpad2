use std::cell::OnceCell;
use std::sync::mpsc;

use adw::subclass::prelude::*;
use diesel::prelude::*;
use gio::subclass::prelude::ApplicationImpl;
use glib::{ObjectExt, Properties};
use gtk::subclass::prelude::DerivedObjectProperties;
use gtk::{gdk, gio, glib};
use gtk::{prelude::*, CssProvider};
use projectpadsql::models::Project;

use crate::keyring_helpers;
use crate::sql_thread::SqlFunc;
use crate::win::ProjectpadApplicationWindow;

mod imp {
    use std::cell::{OnceCell, RefCell};

    use glib::{
        subclass::{prelude::ObjectImpl, types::ObjectSubclass},
        WeakRef,
    };
    use gtk::subclass::prelude::GtkApplicationImpl;

    use crate::win::ProjectpadApplicationWindow;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ProjectpadApplication)]
    pub struct ProjectpadApplication {
        #[property(get)]
        pub rb_server: RefCell<Option<String>>, // TODO remove
        //
        pub window: OnceCell<WeakRef<ProjectpadApplicationWindow>>,

        pub sql_channel: RefCell<Option<mpsc::Sender<SqlFunc>>>,
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
            let _ = app.imp().window.set(window.downgrade());
            app.setup_actions(&window);
            app.unlock_db();
            // let window = app.create_window();
            // let _ = self.window.set(window.downgrade());
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
            // .property("sql_channel", sql_channel)
            // .property("application-id", Some(config::APP_ID))
            // .property("flags", gio::ApplicationFlags::empty())
            // .property("resource-base-path", Some(config::PATH_ID))
            .build();
        app.imp().sql_channel.replace(Some(sql_channel));

        app.connect_startup(|_| Self::load_css());

        // app.connect_activate(move |a| Self::unlock_db(a, sql_channel));
        // Self::unlock_db(app);

        // Start running gtk::Application
        app.run()

        // glib::ExitCode::SUCCESS // TODO
    }

    pub fn get_sql_channel(&self) -> mpsc::Sender<SqlFunc> {
        self.imp().sql_channel.borrow().clone().unwrap()
    }

    fn setup_actions(&self, window: &ProjectpadApplicationWindow) {
        // let select_project_action =
        //     gio::SimpleAction::new("select-project", Some(glib::VariantTy::INT64));
        // select_project_action.connect_activate(|action, parameter| {
        //     println!("{} / {:#?}", action, parameter);
        // });
        let select_project_action = gio::SimpleAction::new_stateful(
            "select-project",
            Some(&i32::static_variant_type()),
            // None,
            &(1).to_variant(),
        );
        let w = window.clone();
        select_project_action.connect_change_state(move |action, parameter| {
            println!("{} / {:#?}", action, parameter);
            let project_id = parameter.unwrap().get::<i32>().unwrap();
            w.set_active_project_and_item(project_id, None);
        });
        window.add_action(&select_project_action);
        dbg!(&window.list_actions());
    }

    fn unlock_db(&self) {
        if let Some(pass) = keyring_helpers::get_pass_from_keyring() {
            // https://gtk-rs.org/gtk4-rs/stable/latest/book/main_event_loop.html
            // Create channel that can hold at most 1 message at a time
            let (sender, receiver) = async_channel::bounded(1);
            self.get_sql_channel()
                .send(SqlFunc::new(move |sql_conn| {
                    let unlock_success = projectpadsql::try_unlock_db(sql_conn, &pass).is_ok();
                    sender.send_blocking(unlock_success).unwrap();
                }))
                .unwrap();

            // The main loop executes the asynchronous block
            let channel2 = self.imp().sql_channel.borrow().as_ref().unwrap().clone();
            // let w = self.imp().window.clone();
            // dbg!("running the app");
            // self.run();
            // dbg!("after running the app");
            glib::spawn_future_local(async move {
                let unlock_success = receiver.recv().await.unwrap();
                if unlock_success {
                    // TODO run_prepare_db
                    // TODO request_update_welcome_status

                    Self::fetch_projects(&channel2);
                } else {
                    // self.display_unlock_dialog();
                }
                // self.run();
            });
        } else {
            // self.display_unlock_dialog();
        }
    }

    fn fetch_projects(sql_channel: &mpsc::Sender<SqlFunc>) {
        let (sender, receiver) = async_channel::bounded(1);
        sql_channel
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl::*;
                let prjs = project.order(name.asc()).load::<Project>(sql_conn).unwrap();
                sender.send_blocking(prjs).unwrap();
                // s.send(prjs).unwrap();
            }))
            .unwrap();
        glib::spawn_future_local(async move {
            let prjs = receiver.recv().await.unwrap();
            let app = gio::Application::default()
                .expect("Failed to retrieve application singleton")
                .downcast::<ProjectpadApplication>()
                .unwrap();
            let window = app.imp().window.get().unwrap();
            let binding = window.upgrade();
            let popover = &binding.as_ref().unwrap().imp().project_popover_menu;
            let menu_model = gio::Menu::new();
            for prj in prjs {
                // println!(
                //     "{}",
                //     gio::Action::print_detailed_name("win.select-project", Some(&1.to_variant()))
                // );
                println!("{}", format!("win.select-project({})", prj.id));
                menu_model.append(
                    Some(&prj.name),
                    Some(&format!("win.select-project({})", prj.id)),
                );
            }
            popover.set_menu_model(Some(&menu_model));
        });
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
        let window = ProjectpadApplicationWindow::new(self.get_sql_channel());
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
