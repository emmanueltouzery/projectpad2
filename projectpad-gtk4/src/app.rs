use std::sync::mpsc;

use adw::subclass::prelude::*;
use diesel::prelude::*;
use gio::subclass::prelude::ApplicationImpl;
use glib::Properties;
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

    fn setup_actions(&self, window: &ProjectpadApplicationWindow, cur_project: Option<&Project>) {
        let select_project_variant = glib::VariantDict::new(None);
        select_project_variant.insert("project_id", cur_project.unwrap().id); // TODO first startup
                                                                              // if no projects
        select_project_variant.insert("item_id", None::<i32>);
        select_project_variant.insert("item_type", None::<u8>);
        select_project_variant.insert("server_id", None::<i32>);

        let select_project_action =
            gio::SimpleAction::new("select-project", Some(glib::VariantTy::INT32));
        window.add_action(&select_project_action);
        let w = window.clone();
        select_project_action.connect_activate(move |_action, parameter| {
            dbg!(&parameter);
            dbg!(&parameter.as_ref().unwrap().get::<i64>());
            // println!("{} / {:#?}", action, parameter);
            let select_project_variant = glib::VariantDict::new(None);
            select_project_variant.insert("project_id", parameter.unwrap().get::<i32>().unwrap());
            select_project_variant.insert("item_id", None::<i32>);
            select_project_variant.insert("item_type", None::<u8>);
            select_project_variant.insert("search_item_type", None::<u8>);
            w.change_action_state("select-project-item", &dbg!(select_project_variant.end()));
        });
        let select_project_item_action = gio::SimpleAction::new_stateful(
            // probably rename this to select-project-item, then uncomment the select-project just
            // above, and have it trigger this, but with the default item id
            "select-project-item",
            Some(&glib::VariantDict::static_variant_type()),
            &select_project_variant.to_variant(),
        );
        let w = window.clone();
        select_project_item_action.connect_change_state(move |action, parameter| {
            dbg!(&parameter);
            action.set_state(parameter.as_ref().unwrap());
            w.set_active_project_item();
        });
        window.add_action(&select_project_item_action);
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
            let app_clone = self.clone();
            glib::spawn_future_local(async move {
                let unlock_success = receiver.recv().await.unwrap();
                if unlock_success {
                    // TODO run_prepare_db
                    // TODO request_update_welcome_status

                    app_clone.fetch_projects(&channel2);
                } else {
                    // self.display_unlock_dialog();
                }
                // self.run();
            });
        } else {
            // self.display_unlock_dialog();
        }
    }

    fn fetch_projects(&self, sql_channel: &mpsc::Sender<SqlFunc>) {
        let (sender, receiver) = async_channel::bounded(1);
        sql_channel
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl::*;
                let prjs = project.order(name.asc()).load::<Project>(sql_conn).unwrap();
                sender.send_blocking(prjs).unwrap();
                // s.send(prjs).unwrap();
            }))
            .unwrap();
        let app_clone = self.clone();
        glib::spawn_future_local(async move {
            let prjs = receiver.recv().await.unwrap();
            let app = gio::Application::default()
                .expect("Failed to retrieve application singleton")
                .downcast::<ProjectpadApplication>()
                .unwrap();
            let window = app.imp().window.get().unwrap();
            let win_binding = window.upgrade();
            let win_binding_ref = win_binding.as_ref().unwrap();
            let popover = &win_binding_ref.imp().project_popover_menu;
            let menu_model = gio::Menu::new();
            let select_project_variant = glib::VariantDict::new(None);
            app_clone.setup_actions(&win_binding_ref, prjs.first());

            let w = app_clone.imp().window.get().unwrap().upgrade().unwrap();

            if !prjs.is_empty() {
                select_project_variant.insert("project_id", prjs.first().unwrap().id);
                select_project_variant.insert("item_id", None::<i32>);
                select_project_variant.insert("item_type", None::<u8>);
                select_project_variant.insert("search_item_type", None::<u8>);
                w.change_action_state("select-project-item", &dbg!(select_project_variant.end()));
            }

            for prj in prjs {
                select_project_variant.insert("project_id", prj.id);
                select_project_variant.insert("item_id", None::<i32>);
                select_project_variant.insert("item_type", None::<u8>);
                select_project_variant.insert("search_item_type", None::<u8>);
                // tie this menu to a gsimpleaction without state but with a parameter, which is
                // the project to activate
                dbg!(Some(&gio::Action::print_detailed_name(
                    "win.select-project",
                    Some(&prj.id.to_variant()),
                )));
                menu_model.append(
                    Some(&prj.name),
                    Some(&gio::Action::print_detailed_name(
                        "win.select-project",
                        Some(&prj.id.to_variant()),
                    )),
                );
            }
            popover.set_menu_model(Some(&menu_model));

            win_binding_ref.set_active_project_item();
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

    pub fn get_toast_overlay(&self) -> adw::ToastOverlay {
        self.imp()
            .window
            .get()
            .unwrap()
            .upgrade()
            .unwrap()
            .get_toast_overlay()
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
