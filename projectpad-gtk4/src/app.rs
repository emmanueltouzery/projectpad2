use std::sync::mpsc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use async_channel::Receiver;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use gio::subclass::prelude::ApplicationImpl;
use gtk::CssProvider;
use gtk::{gdk, gio, glib};
use projectpadsql::models::{
    EnvironmentType, Project, Server, ServerDatabase, ServerLink, ServerWebsite,
};

use crate::sql_thread::{self, SqlFunc};
use crate::widgets::move_project_item::MoveProjectItem;
use crate::widgets::project_edit::ProjectEdit;
use crate::widgets::project_item_list::ProjectItemList;
use crate::widgets::project_item_model::ProjectItemType;
use crate::widgets::project_items::common::{self, run_sqlfunc};
use crate::win::ProjectpadApplicationWindow;
use crate::{import_export_ui, keyring_helpers, perform_insert_or_update, sql_util};
use crate::{preferences_dialog, unlock_db_dialog};

const SHORTCUTS_UI: &str = include_str!("shortcuts.ui");

mod imp {
    use std::cell::{OnceCell, RefCell};

    use glib::{
        subclass::{prelude::ObjectImpl, types::ObjectSubclass},
        WeakRef,
    };
    use gtk::subclass::prelude::GtkApplicationImpl;

    use crate::win::ProjectpadApplicationWindow;

    use super::*;

    #[derive(Default)]
    pub struct ProjectpadApplication {
        pub window: OnceCell<WeakRef<ProjectpadApplicationWindow>>,

        pub sql_channel: RefCell<Option<mpsc::Sender<SqlFunc>>>,

        pub is_new_db: RefCell<Option<bool>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProjectpadApplication {
        const NAME: &'static str = "ProjectpadApplication";
        type ParentType = adw::Application;
        type Type = super::ProjectpadApplication;
    }

    impl ObjectImpl for ProjectpadApplication {}

    impl ApplicationImpl for ProjectpadApplication {
        fn activate(&self) {
            let app = self.obj();
            let window = app.create_window();

            let w = window.clone();
            let key_controller = gtk::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_controller, keyval, _keycode, state| {
                if let Some(k) = keyval.to_unicode() {
                    if state == gdk::ModifierType::CONTROL_MASK && k == 'e' {
                        common::main_win().trigger_edit();
                    }
                    if state == gdk::ModifierType::CONTROL_MASK && k == 'y' {
                        common::main_win().trigger_copy_visible_pass();
                    }

                    if state == gdk::ModifierType::CONTROL_MASK
                        && k == 's'
                        && !w.imp().search_toggle_btn.is_active()
                    {
                        w.imp().search_toggle_btn.set_active(true);
                        return glib::Propagation::Stop; // Stop further handling
                    }

                    if Self::is_plaintext_key(state, keyval)
                        && !w.imp().search_toggle_btn.is_active()
                    {
                        w.imp().search_entry.set_text(&k.to_string());
                        w.imp().search_entry.set_position(1);
                        w.imp().search_toggle_btn.set_active(true);
                        return glib::Propagation::Stop; // Stop further handling
                    }
                }
                glib::Propagation::Proceed // Allow other handlers to process the event
            });
            window.add_controller(key_controller);

            let _ = app.imp().window.set(window.downgrade());
            app.unlock_db();
            // let window = app.create_window();
            // let _ = self.window.set(window.downgrade());
        }
    }

    impl GtkApplicationImpl for ProjectpadApplication {}

    impl AdwApplicationImpl for ProjectpadApplication {}

    impl ProjectpadApplication {
        pub fn is_plaintext_key(e: gdk::ModifierType, keyval: gdk::Key) -> bool {
            // return false if control and others were pressed
            // (then the state won't be empty)
            // could be ctrl-c on notes for instance
            // whitelist LOCK (shift or caps lock)
            let mut state = e;
            state.remove(gdk::ModifierType::LOCK_MASK);
            state.is_empty()
                && keyval != gdk::Key::Return
                && keyval != gdk::Key::KP_Enter
                && keyval != gdk::Key::Escape
                && keyval != gdk::Key::Tab
        }
    }
}

glib::wrapper! {
    pub struct ProjectpadApplication(ObjectSubclass<imp::ProjectpadApplication>)
        @extends gio::Application, gtk::Application, adw::Application;
        // @implements gio::ActionMap, gio::ActionGroup;
}

// here first run is not about the first time we run the app (when there's no
// db yet), but the first time we display the list of projects in a particular
// app run (when we must setup actions and so on)
#[derive(PartialEq, Eq, Copy, Clone)]
pub enum RunMode {
    FirstRun,
    Normal,
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
        app.imp().is_new_db.replace(Some(is_new_db));

        app.connect_startup(|_| Self::load_css());

        // app.connect_activate(move |a| Self::unlock_db(a, sql_channel));
        // Self::unlock_db(app);

        // Start running gtk::Application
        app.run()
    }

    pub fn get_sql_channel(&self) -> mpsc::Sender<SqlFunc> {
        self.imp().sql_channel.borrow().clone().unwrap()
    }

    // set_state vs change_state... this doesn't trigger the change_action_state handler
    // useful to avoid loops of changing state->handler redisplays->changing state->...
    pub fn change_select_project_item_no_signal(&self, select_project_item: glib::Variant) {
        let action = self
            .imp()
            .window
            .get()
            .unwrap()
            .upgrade()
            .unwrap()
            .lookup_action("select-project-item")
            .unwrap()
            .downcast::<gio::SimpleAction>()
            .unwrap();
        action.set_state(&select_project_item);
    }

    fn setup_actions_and_populate_menu(
        &self,
        run_mode: RunMode,
        window: &ProjectpadApplicationWindow,
        cur_project: Option<&Project>,
        prjs: &[Project],
    ) {
        if let Some(prj) = cur_project {
            self.do_setup_actions_and_populate_menu(run_mode, window, prj.id, prjs);
        } else {
            // first startup, no project, create a default one
            let recv = run_sqlfunc(Box::new(|sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                let changeset = (
                    prj::name.eq("Default project"),
                    prj::has_prod.eq(true),
                    // TODO the icon is actually not-null in SQL...
                    prj::icon.eq(Some(vec![])),
                );
                perform_insert_or_update!(sql_conn, None, prj::project, prj::id, changeset, Project,)
            }));
            let s = self.clone();
            let w = window.clone();
            let mut prjs_c = prjs.to_vec().clone();
            glib::spawn_future_local(async move {
                let insert_res = recv.recv().await.unwrap();
                match insert_res {
                    Ok(p) => {
                        let p_id = p.id;
                        prjs_c.push(p);
                        s.do_setup_actions_and_populate_menu(run_mode, &w, p_id, &prjs_c);
                    }
                    Err(e) => {
                        common::simple_error_dlg("Error creating default project", e.1.as_deref())
                    }
                }
            });
        }
    }

    fn do_setup_actions_and_populate_menu(
        &self,
        run_mode: RunMode,
        window: &ProjectpadApplicationWindow,
        cur_project_id: i32,
        prjs: &[Project],
    ) {
        let select_project_variant = glib::VariantDict::new(None);
        select_project_variant.insert("project_id", cur_project_id);
        select_project_variant.insert("item_id", None::<i32>);
        select_project_variant.insert("item_type", None::<u8>);
        select_project_variant.insert("server_id", None::<i32>);

        let select_project_action =
            gio::SimpleAction::new("select-project", Some(glib::VariantTy::INT32));
        window.add_action(&select_project_action);
        let w = window.clone();
        select_project_action.connect_activate(move |_action, parameter| {
            // println!("{} / {:#?}", action, parameter);
            let select_project_variant = glib::VariantDict::new(None);
            select_project_variant.insert("project_id", parameter.unwrap().get::<i32>().unwrap());
            select_project_variant.insert("item_id", None::<i32>);
            select_project_variant.insert("item_type", None::<u8>);
            select_project_variant.insert("search_item_type", None::<u8>);
            w.change_action_state("select-project-item", &select_project_variant.end());
        });
        let select_project_item_action = gio::SimpleAction::new_stateful(
            "select-project-item",
            Some(&glib::VariantDict::static_variant_type()),
            &select_project_variant.to_variant(),
        );
        let w = window.clone();
        let s = self.clone();
        let channel2 = self.imp().sql_channel.borrow().as_ref().unwrap().clone();
        select_project_item_action.connect_change_state(move |action, parameter| {
            let cur_project_id = glib::VariantDict::new(action.state().as_ref())
                .lookup::<i32>("project_id")
                .unwrap();
            let new_project_id = glib::VariantDict::new(parameter)
                .lookup::<i32>("project_id")
                .unwrap();
            let project_changed = cur_project_id != new_project_id;
            action.set_state(parameter.as_ref().unwrap());
            w.display_active_project_item();
            if project_changed {
                s.fetch_projects_and_populate_menu(RunMode::Normal, &channel2);
            }
        });
        window.add_action(&select_project_item_action);

        let new_project_action = gio::SimpleAction::new("add-project", None);
        window.add_action(&new_project_action);

        let s = self.clone();
        new_project_action.connect_activate(move |_action, _parameter| {
            s.open_add_edit_project(None);
        });

        // change type to VariantDict and put id+name in there so i don't have to query for the
        // name
        let delete_project_action = gio::SimpleAction::new(
            "delete-project",
            Some(&glib::VariantDict::static_variant_type()),
        );
        window.add_action(&delete_project_action);

        delete_project_action.connect_activate(move |_action, parameter| {
            let variant_dict = glib::VariantDict::new(parameter);

            let project_id = variant_dict.lookup::<i32>("project_id").unwrap().unwrap();
            let project_name = variant_dict.lookup::<String>("project_name").unwrap().unwrap();
            common::confirm_delete(
                &format!("Delete project {project_name}"),
                &format!("Are you sure you want to delete the project {project_name}? This action cannot be undone, and all project items will also be deleted."),
                Box::new(move || {
                    Self::do_delete_project(project_id);
                }),
            );
        });

        let edit_project_action =
            gio::SimpleAction::new("edit-project", Some(glib::VariantTy::INT32));
        let s = self.clone();
        edit_project_action.connect_activate(move |_action, parameter| {
            let project_id = parameter.as_ref().unwrap().get::<i32>().unwrap();
            let s = s.clone();

            let receiver = common::run_sqlfunc(Box::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                prj::project
                    .filter(prj::id.eq(project_id))
                    .first::<Project>(sql_conn)
                    .unwrap()
            }));
            glib::spawn_future_local(async move {
                let prj = receiver.recv().await.unwrap();
                s.open_add_edit_project(Some(prj));
            });
        });
        window.add_action(&edit_project_action);

        let select_project_action = gio::SimpleAction::new("move-project-item", None);
        select_project_action.connect_activate(move |_action, _parameter| {
            Self::open_move_project_item_dlg();
        });
        window.add_action(&select_project_action);

        let import_export_action = gio::SimpleAction::new("import-export", None);
        import_export_action.connect_activate(move |_action, _parameter| {
            import_export_ui::open_import_export_dlg();
        });
        window.add_action(&import_export_action);

        let prefs_action = gio::SimpleAction::new("preferences", None);
        prefs_action.connect_activate(move |_action, _parameter| {
            preferences_dialog::display_preferences_dialog();
        });
        window.add_action(&prefs_action);

        let open_help_action = gio::SimpleAction::new("open-help", None);
        open_help_action.connect_activate(move |_action, _parameter| {
            if let Err(err) = gio::AppInfo::launch_default_for_uri(
                "https://github.com/emmanueltouzery/projectpad2/wiki/Help",
                None::<&gio::AppLaunchContext>,
            ) {
                eprintln!("Failed to open help: {}", err);
            }
        });
        window.add_action(&open_help_action);

        let open_about_action = gio::SimpleAction::new("open-shortcuts", None);
        open_about_action.connect_activate(move |_action, _parameter| {
            let win = gtk::Builder::from_string(SHORTCUTS_UI)
                .object::<gtk::Window>("shortcuts")
                .unwrap();
            win.set_title(Some("Keyboard Shortcuts"));
            win.set_transient_for(Some(&common::main_win()));
            win.set_visible(true);
        });
        window.add_action(&open_about_action);

        let open_about_action = gio::SimpleAction::new("about", None);
        open_about_action.connect_activate(move |_action, _parameter| {
            adw::AboutDialog::builder()
                .application_name("Projectpad")
                .application_icon("com.github.emmanueltouzery.projectpad")
                .version("3.0.1")
                .website("https://github.com/emmanueltouzery/projectpad2")
                .build()
                .present(Some(&common::main_win()));
        });
        window.add_action(&open_about_action);

        self.populate_menu(run_mode, prjs);
    }

    fn open_move_project_item_dlg() {
        let win = common::main_win();
        let select_project_item_state =
            glib::VariantDict::new(win.action_state("select-project-item").as_ref());

        let project_id = select_project_item_state
            .lookup::<i32>("project_id")
            .unwrap()
            .unwrap();

        let m_project_item_type = select_project_item_state
            .lookup::<Option<u8>>("item_type")
            .unwrap()
            .and_then(std::convert::identity)
            .and_then(ProjectItemType::from_repr);

        let m_project_item_id = select_project_item_state
            .lookup::<Option<i32>>("item_id")
            .unwrap()
            .unwrap();
        if m_project_item_id.is_none() {
            // the current project has no project item at all
            return;
        }
        let project_item_id = m_project_item_id.unwrap();
        let project_item_type = m_project_item_type.unwrap();

        let mpi = MoveProjectItem::new(project_id, project_item_id, project_item_type);
        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let header_bar = adw::HeaderBar::builder()
            .show_end_title_buttons(false)
            .show_start_title_buttons(false)
            .build();

        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        header_bar.pack_start(&cancel_btn);

        let move_btn = gtk::Button::builder()
            .label("Move")
            .css_classes(["suggested-action"])
            .build();
        header_bar.pack_end(&move_btn);

        vbox.append(&header_bar);

        vbox.append(&mpi);

        let dialog = adw::Dialog::builder()
            .title("Move project item")
            .content_width(450)
            .child(&vbox)
            .build();

        dialog.present(Some(&common::main_win()));

        let dlg = dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            dlg.close();
        });

        let dlg = dialog.clone();
        move_btn.connect_clicked(move |_| {
            let to_project_id = mpi.project_id();
            let move_receiver = Self::do_move_project_item(
                project_item_id,
                project_item_type,
                mpi.project_id(),
                EnvironmentType::from_repr(mpi.environment() as u8).unwrap(),
            );
            let dlg = dlg.clone();
            glib::spawn_future_local(async move {
                let move_res = move_receiver.recv().await.unwrap();
                match move_res {
                    Ok(_) => {
                        dlg.close();
                        ProjectItemList::display_project_item(
                            Some(to_project_id),
                            project_item_id,
                            project_item_type,
                        );
                    }
                    Err(e) => {
                        common::simple_error_dlg("Error moving the item", Some(&e.to_string()));
                    }
                }
            });
        });
    }

    fn do_move_project_item(
        project_item_id: i32,
        project_item_type: ProjectItemType,
        to_project_id: i32,
        to_environment: EnvironmentType,
    ) -> Receiver<Result<usize, diesel::result::Error>> {
        use projectpadsql::schema::project_note::dsl as prj_note;
        use projectpadsql::schema::project_point_of_interest::dsl as prj_poi;
        use projectpadsql::schema::server::dsl as srv;
        use projectpadsql::schema::server_link::dsl as srvl;
        common::run_sqlfunc(Box::new(move |sql_conn| match project_item_type {
            ProjectItemType::Server => {
                let changeset = (
                    srv::project_id.eq(to_project_id),
                    srv::environment.eq(to_environment),
                );
                diesel::update(srv::server.filter(srv::id.eq(project_item_id)))
                    .set(changeset)
                    .execute(sql_conn)
            }
            ProjectItemType::ServerLink => {
                let changeset = (
                    srvl::project_id.eq(to_project_id),
                    srvl::environment.eq(to_environment),
                );
                diesel::update(srvl::server_link.filter(srvl::id.eq(project_item_id)))
                    .set(changeset)
                    .execute(sql_conn)
            }
            ProjectItemType::ProjectNote => {
                let update_note_env =
                    diesel::update(prj_note::project_note.filter(prj_note::id.eq(project_item_id)));
                match to_environment {
                    EnvironmentType::EnvDevelopment => update_note_env
                        .set(prj_note::has_dev.eq(true))
                        .execute(sql_conn),
                    EnvironmentType::EnvStage => update_note_env
                        .set(prj_note::has_stage.eq(true))
                        .execute(sql_conn),
                    EnvironmentType::EnvUat => update_note_env
                        .set(prj_note::has_uat.eq(true))
                        .execute(sql_conn),
                    EnvironmentType::EnvProd => update_note_env
                        .set(prj_note::has_prod.eq(true))
                        .execute(sql_conn),
                }?;
                diesel::update(prj_note::project_note.filter(prj_note::id.eq(project_item_id)))
                    .set(prj_note::project_id.eq(to_project_id))
                    .execute(sql_conn)
            }
            ProjectItemType::ProjectPointOfInterest => {
                let changeset = (prj_poi::project_id.eq(to_project_id),);
                diesel::update(
                    prj_poi::project_point_of_interest.filter(prj_poi::id.eq(project_item_id)),
                )
                .set(changeset)
                .execute(sql_conn)
            }
        }))
    }

    fn do_delete_project(prj_id: i32) {
        let (sender, receiver) = async_channel::bounded(1);
        let app = common::app();
        app.get_sql_channel()
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                use projectpadsql::schema::server::dsl as srv;
                use projectpadsql::schema::server_database::dsl as db;
                use projectpadsql::schema::server_link::dsl as srv_link;
                use projectpadsql::schema::server_website::dsl as srvw;

                let prjs_count = prj::project.count().get_result::<i64>(sql_conn).unwrap();

                // we cannot delete a project if a server under it is
                // linked to from another project
                let dependent_serverlinks = srv_link::server_link
                    .inner_join(srv::server)
                    .filter(
                        srv::project_id
                            .eq(prj_id)
                            .and(srv_link::project_id.ne(prj_id)),
                    )
                    .load::<(ServerLink, Server)>(sql_conn)
                    .unwrap();

                let contained_dbs: Vec<_> = db::server_database
                    .inner_join(srv::server)
                    .filter(srv::project_id.eq(prj_id))
                    .load::<(ServerDatabase, Server)>(sql_conn)
                    .unwrap()
                    .into_iter()
                    .map(|x| x.0)
                    .collect();

                let dependent_websites: Vec<_> = srvw::server_website
                    .inner_join(srv::server)
                    .filter(
                        srv::project_id.ne(prj_id).and(
                            srvw::server_database_id
                                .eq_any(contained_dbs.iter().map(|d| d.id).collect::<Vec<_>>()),
                        ),
                    )
                    .load::<(ServerWebsite, Server)>(sql_conn)
                    .unwrap()
                    .into_iter()
                    .map(|x| x.0)
                    .collect();
                if !dependent_serverlinks.is_empty() {
                    sender.send_blocking(Err((
                        "Cannot delete project",
                        Some(format!(
                            "servers {} on that server are linked to by servers {}",
                            itertools::join(
                                dependent_serverlinks.iter().map(|(_, s)| &s.desc),
                                ", "
                            ),
                            itertools::join(
                                dependent_serverlinks.iter().map(|(l, _)| &l.desc),
                                ", "
                            )
                        )),
                    )))
                } else if !dependent_websites.is_empty() {
                    sender.send_blocking(Err((
                        "Cannot delete project",
                        Some(format!(
                            "databases {} on that server are linked to by websites {}",
                            itertools::join(
                                dependent_websites.iter().map(|w| &contained_dbs
                                    .iter()
                                    .find(|d| Some(d.id) == w.server_database_id)
                                    .unwrap()
                                    .desc),
                                ", "
                            ),
                            itertools::join(dependent_websites.iter().map(|w| &w.desc), ", ")
                        )),
                    )))
                } else if prjs_count == 1 {
                    sender.send_blocking(Err((
                        "Cannot delete project",
                        Some("Cannot delete the last project".to_owned()),
                    )))
                } else {
                    sender.send_blocking(
                        sql_util::delete_row(sql_conn, prj::project, prj_id).map(|_| prj_id),
                    )
                }
                .unwrap();
            }))
            .unwrap();

        glib::spawn_future_local(async move {
            let insert_res = receiver.recv().await.unwrap();
            match insert_res {
                Ok(_p_id) => {
                    // FirstRun will make sure another project gets selected
                    common::app().fetch_projects_and_populate_menu(
                        RunMode::FirstRun,
                        &app.get_sql_channel(),
                    );
                }
                Err((msg, e)) => common::simple_error_dlg(msg, e.as_deref()),
            }
        });
    }

    fn open_add_edit_project(&self, project: Option<Project>) {
        let vbox = gtk::Box::builder()
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
        vbox.append(&header_bar);

        let p_id = project.as_ref().map(|p| p.id);

        let is_edit = project.is_some();

        let project_edit = ProjectEdit::new();
        if let Some(prj) = project {
            project_edit.set_title(prj.name);
            project_edit.set_env_dev(prj.has_dev);
            project_edit.set_env_stg(prj.has_stage);
            project_edit.set_env_uat(prj.has_uat);
            project_edit.set_env_prd(prj.has_prod);
        }
        vbox.append(&project_edit);

        let dialog = adw::Dialog::builder()
            .title(if is_edit {
                "Edit project"
            } else {
                "Add project"
            })
            .child(&vbox)
            .build();

        let dlg = dialog.clone();
        cancel_btn.connect_clicked(move |_btn: &gtk::Button| {
            dlg.close();
        });

        let dlg = dialog.clone();
        save_btn.connect_clicked(move |_btn: &gtk::Button| {
            let title = project_edit.title();
            let has_dev = project_edit.env_dev();
            let has_stg = project_edit.env_stg();
            let has_uat = project_edit.env_uat();
            let has_prd = project_edit.env_prd();

            let (sender, receiver) = async_channel::bounded(1);
            common::app()
                .get_sql_channel()
                .send(SqlFunc::new(move |sql_conn| {
                    if !(has_dev || has_stg || has_uat || has_prd) {
                        sender
                            .send_blocking(Err((
                                "Error saving project".to_owned(),
                                Some("Must pick at least one environment".to_owned()),
                            )))
                            .unwrap();
                        return;
                    }

                    use projectpadsql::schema::project::dsl as prj;
                    let changeset = (
                        prj::name.eq(title.as_str()),
                        prj::has_dev.eq(has_dev),
                        prj::has_stage.eq(has_stg),
                        prj::has_uat.eq(has_uat),
                        prj::has_prod.eq(has_prd),
                        // TODO the icon is actually not-null in SQL...
                        prj::icon.eq(Some(vec![])),
                    );
                    let project_after_result = perform_insert_or_update!(
                        sql_conn,
                        p_id,
                        prj::project,
                        prj::id,
                        changeset,
                        Project,
                    );
                    sender.send_blocking(project_after_result).unwrap();
                }))
                .unwrap();

            let dlg = dlg.clone();
            glib::spawn_future_local(async move {
                let insert_res = receiver.recv().await.unwrap();
                match insert_res {
                    Ok(prj) => {
                        dlg.close();
                        let app = common::app();
                        app.fetch_projects_and_populate_menu(
                            RunMode::Normal,
                            &app.get_sql_channel(),
                        );
                        ProjectItemList::display_project(prj.id);
                    }
                    Err((title, msg)) => common::simple_error_dlg(&title, msg.as_deref()),
                }
            });
        });

        dialog.present(Some(&common::main_win()));
    }

    fn unlock_db(&self) {
        let is_new_db = self.imp().is_new_db.borrow().unwrap();
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
            // let w = self.imp().window.clone();
            // dbg!("running the app");
            // self.run();
            // dbg!("after running the app");
            glib::spawn_future_local(async move {
                let unlock_success = receiver.recv().await.unwrap();
                if unlock_success {
                    Self::load_app_after_unlock();
                } else {
                    unlock_db_dialog::display_unlock_dialog(is_new_db);
                }
                // self.run();
            });
        } else {
            unlock_db_dialog::display_unlock_dialog(is_new_db);
        }
    }

    pub fn load_app_after_unlock() {
        let app = common::app();
        let sql_channel = app.get_sql_channel();
        Self::run_prepare_db(sql_channel.clone());
        app.fetch_projects_and_populate_menu(RunMode::FirstRun, &sql_channel);
    }

    fn run_prepare_db(sql_channel: mpsc::Sender<SqlFunc>) {
        sql_channel
            .send(SqlFunc::new(|sql_conn| {
                sql_thread::migrate_db_if_needed(sql_conn).unwrap();
                sql_conn.batch_execute("PRAGMA foreign_keys = ON").unwrap();
            }))
            .unwrap();
    }

    pub fn project_id(&self) -> Option<i32> {
        let project_state = glib::VariantDict::new(
            self.imp()
                .window
                .get()
                .unwrap()
                .upgrade()
                .unwrap()
                .action_state("select-project-item")
                .as_ref(),
        );
        // i32::try_from(project_state.lookup::<i64>("project_id").unwrap().unwrap()).unwrap();
        project_state.lookup::<i32>("project_id").unwrap()
    }

    pub fn fetch_projects_and_populate_menu(
        &self,
        run_mode: RunMode,
        sql_channel: &mpsc::Sender<SqlFunc>,
    ) {
        let (sender, receiver) = async_channel::bounded(1);
        sql_channel
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl::*;
                let prjs = project.order(name.asc()).load::<Project>(sql_conn).unwrap();
                sender.send_blocking(prjs).unwrap();
                // s.send(prjs).unwrap();
            }))
            .unwrap();

        // get the current project now, but then we'll recompute the menu if/when
        // the current project change (or indeed if the project list changes)
        let project_id_maybe = self.project_id();

        glib::spawn_future_local(async move {
            let prjs = receiver.recv().await.unwrap();
            let app = get();
            let window = app.imp().window.get().unwrap();
            let win_binding = window.upgrade();
            let win_binding_ref = win_binding.as_ref().unwrap();
            if run_mode == RunMode::FirstRun && project_id_maybe.is_none() {
                // first run only
                app.setup_actions_and_populate_menu(run_mode, win_binding_ref, prjs.first(), &prjs);
            } else {
                app.populate_menu(run_mode, &prjs);
            }
        });
    }

    fn populate_menu(&self, run_mode: RunMode, prjs: &[Project]) {
        let w = self.imp().window.get().unwrap().upgrade().unwrap();
        let popover = &w.imp().project_popover_menu;
        let project_id_maybe = self.project_id();

        let menu_model = gio::Menu::new();
        let select_project_variant = glib::VariantDict::new(None);

        if run_mode == RunMode::FirstRun && !prjs.is_empty() {
            select_project_variant.insert("project_id", prjs.first().unwrap().id);
            select_project_variant.insert("item_id", None::<i32>);
            select_project_variant.insert("item_type", None::<u8>);
            select_project_variant.insert("search_item_type", None::<u8>);
            w.change_action_state("select-project-item", &select_project_variant.end());
        }

        for prj in prjs.iter() {
            select_project_variant.insert("project_id", prj.id);
            select_project_variant.insert("item_id", None::<i32>);
            select_project_variant.insert("item_type", None::<u8>);
            select_project_variant.insert("search_item_type", None::<u8>);
            // tie this menu to a gsimpleaction without state but with a parameter, which is
            // the project to activate
            menu_model.append(
                Some(&prj.name),
                Some(&gio::Action::print_detailed_name(
                    "win.select-project",
                    Some(&prj.id.to_variant()),
                )),
            );
        }

        let cur_project_maybe = if let Some(project_id) = project_id_maybe {
            let cur_prj = prjs.iter().find(|p| p.id == project_id);
            if cur_prj.is_none() {
                // happens when we delete the current project
                prjs.first()
            } else {
                cur_prj
            }
        } else {
            prjs.first()
        };
        if let Some(cur_project) = cur_project_maybe {
            let project_actions_menu_model = gio::Menu::new();

            project_actions_menu_model.append(Some("Add project"), Some("win.add-project"));
            project_actions_menu_model.append(
                Some(&format!("Edit project: {}", cur_project.name)),
                Some(&gio::Action::print_detailed_name(
                    "win.edit-project",
                    Some(&cur_project.id.to_variant()),
                )),
            );
            let delete_project_variant = glib::VariantDict::new(None);
            delete_project_variant.insert("project_id", cur_project.id);
            delete_project_variant.insert("project_name", cur_project.name.to_owned());
            project_actions_menu_model.append(
                Some(&format!("Delete project: {}", cur_project.name)),
                Some(&gio::Action::print_detailed_name(
                    "win.delete-project",
                    Some(&delete_project_variant.end()),
                )),
            );

            menu_model.append_section(Some("Project actions"), &project_actions_menu_model);
        }
        // also add project, delete project, plus menu separator
        // the separator is possibly a section: https://gtk-rs.org/gtk-rs-core/stable/0.16/docs/gio/struct.Menu.html
        popover.set_menu_model(Some(&menu_model));

        w.display_active_project_item();
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
        provider.load_from_bytes(&glib::Bytes::from(include_str!("style.css").as_bytes()));

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

pub fn get() -> ProjectpadApplication {
    gio::Application::default()
        .expect("Failed to retrieve application singleton")
        .downcast::<ProjectpadApplication>()
        .unwrap()
}

impl Default for ProjectpadApplication {
    fn default() -> Self {
        gio::Application::default()
            .expect("Could not get default GApplication")
            .downcast()
            .unwrap()
    }
}
