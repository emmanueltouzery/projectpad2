use std::panic;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use crate::search_engine;
use crate::sql_thread::SqlFunc;
use crate::widgets::project_item::ProjectItem;
use crate::widgets::project_items::common;
use crate::widgets::search::search_item_list::SearchItemList;
use crate::widgets::search::search_item_model::SearchItemType;

use super::widgets::project_item_list::ProjectItemList;
use adw::subclass::prelude::*;
use diesel::prelude::*;
use glib::subclass;
use gtk::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use gtk::{gdk, glib};
use projectpadsql::models::Project;

mod imp {
    use std::cell::RefCell;

    use crate::widgets::{project_item::ProjectItem, search::search_item_list::SearchItemList};

    use super::*;
    use glib::subclass::prelude::ObjectImpl;
    use gtk::{
        subclass::{
            prelude::ObjectSubclass,
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/emmanueltouzery/projectpad2/src/win.ui")]
    pub struct ProjectpadApplicationWindow {
        #[template_child]
        pub project_item_list: TemplateChild<ProjectItemList>,
        #[template_child]
        pub project_item: TemplateChild<ProjectItem>,
        #[template_child]
        pub project_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub project_popover_menu: TemplateChild<gtk::PopoverMenu>,
        #[template_child]
        pub app_popover_menu: TemplateChild<gtk::PopoverMenu>,
        #[template_child]
        pub search_toggle_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_item_list: TemplateChild<SearchItemList>,
        #[template_child]
        pub main_or_search: TemplateChild<gtk::Stack>,
        #[template_child]
        pub split_view: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub project_scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub project_toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub project_item_header_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,

        pub sql_channel: RefCell<Option<mpsc::Sender<SqlFunc>>>,

        pub timer_id: Arc<RefCell<Option<glib::JoinHandle<()>>>>,
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

            // self.obj().connect_action_state_changed(
            //     Some("select-project"),
            //     |_win, _action_name, st| {
            //         println!("action state changed #{st}");
            //     },
            // );

            // self.project_item_list
            //     .get()
            //     .set_project_items(&Vec::new(), HashMap::new(), None);
            // let app = ProjectpadApplication::default();
            // let sender = app.imp().sender.clone();
            // let player = app.imp().player.clone();

            // self.obj().setup_widgets(sender.clone(), player);
            // self.obj().setup_gactions(sender);
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

impl ProjectpadApplicationWindow {
    pub fn new(db_sender: mpsc::Sender<SqlFunc>) -> Self {
        let win = glib::Object::new::<Self>();
        win.imp().sql_channel.replace(Some(db_sender.clone()));
        win.imp().project_item_list.connect_closure(
            "activate-item",
            false,
            glib::closure_local!(
                #[strong(rename_to = w)]
                win,
                move |_project_item_list: ProjectItemList,
                      item_id: i32,
                      project_item_type: u8,
                      sub_item_id: i32,
                      title: String| {
                    let _freeze_guard = w.imp().project_item.freeze_notify(); // https://github.com/gtk-rs/gtk-rs-core/issues/1339
                    w.imp().project_item.set_properties(&[
                        ("item-id", &item_id),
                        ("sub-item-id", &sub_item_id),
                        ("project-item-type", &project_item_type),
                    ]);
                    w.imp().project_item_header_label.set_label(&title);

                    // update the select project item info
                    let win = common::main_win();
                    let select_project_item_state =
                        glib::VariantDict::new(win.action_state("select-project-item").as_ref());

                    let project_id = select_project_item_state
                        .lookup::<i32>("project_id")
                        .unwrap()
                        .unwrap();

                    let select_project_param = glib::VariantDict::new(None);
                    select_project_param.insert("project_id", project_id);
                    select_project_param.insert("item_id", Some(item_id));
                    select_project_param.insert("item_type", Some(project_item_type));

                    // w.change_action_state("select-project-item", &select_project_param.end());

                    common::app().change_select_project_item_no_signal(select_project_param.end());
                    // end update the select project item info

                    let popover = &w.imp().app_popover_menu;
                    let menu_model = gio::Menu::new();
                    // if: possible the project is empty, no project items at all
                    if item_id > 0 {
                        menu_model.append(
                            Some(&format!("Move '{title}'...")),
                            Some("win.move-project-item"),
                        );
                    }
                    menu_model.append(Some("Import/Export"), Some("win.import-export"));
                    menu_model.append(Some("Help"), Some("win.open-help"));
                    popover.set_menu_model(Some(&menu_model));
                }
            ),
        );
        win.imp().project_item.connect_closure(
            "request-scroll",
            false,
            glib::closure_local!(
                #[strong(rename_to = w)]
                win,
                move |_project_item: ProjectItem, offset: f32| {
                    let vadj = w.imp().project_scrolled_window.vadjustment();
                    vadj.set_value(offset.into());
                }
            ),
        );

        win.imp()
            .project_scrolled_window
            .vadjustment()
            .connect_closure(
                "value-changed",
                true,
                glib::closure_local!(
                    #[strong(rename_to = w)]
                    win,
                    move |adj: gtk::Adjustment| {
                        let should_reveal = adj.value() > 0.0;
                        let tb = w.imp().project_toolbar_view.clone();

                        // debounce the showing/hiding of the top bar, otherwise it can glitch
                        // to show/hide very fast when you're on the edge
                        if let Some(id) = w.imp().timer_id.borrow().as_ref() {
                            id.abort();
                        }
                        w.imp()
                            .timer_id
                            .replace(Some(glib::spawn_future_local(async move {
                                glib::timeout_future(Duration::from_millis(300)).await;
                                tb.set_reveal_top_bars(should_reveal);
                            })));
                    }
                ),
            );

        win.imp().search_entry.connect_show(|entry| {
            entry.grab_focus();
        });

        win.imp().search_toggle_btn.connect_toggled(glib::clone!(
            #[strong(rename_to = w)]
            win,
            move |_| {
                Self::search_toggled(&w);
            }
        ));

        win.imp().search_entry.connect_search_changed(glib::clone!(
            #[strong(rename_to = w)]
            win,
            move |_entry| {
                w.trigger_search();
            }
        ));

        let w0 = win.clone();
        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_controller, keyval, _keycode, _state| {
            if keyval == gdk::Key::Escape && w0.imp().search_toggle_btn.is_active() {
                w0.imp().search_toggle_btn.set_active(false);
                return glib::Propagation::Stop; // Stop further handling
            }
            glib::Propagation::Proceed // Allow other handlers to process the event
        });
        win.imp().search_entry.add_controller(key_controller);
        let w = win.clone();
        win.imp().search_entry.connect_activate(move |_| {
            let search_matches = w.imp().search_item_list.displayed_items();
            let mut i = 0;
            let mut level1_item = None::<glib::Object>;
            let mut level2_item = None::<glib::Object>;
            let mut level3_item = None::<glib::Object>;
            while i < search_matches.n_items() {
                let item_model = search_matches.item(i).unwrap();
                let search_item_type =
                    SearchItemType::from_repr(item_model.property("search-item-type")).unwrap();
                match search_item_type {
                    SearchItemType::Project => {
                        if level1_item.is_some() {
                            return;
                        }
                        level1_item = Some(item_model);
                    }
                    SearchItemType::Server
                    | SearchItemType::ServerLink
                    | SearchItemType::ProjectNote
                    | SearchItemType::ProjectPointOfInterest => {
                        if level2_item.is_some() {
                            return;
                        }
                        level2_item = Some(item_model);
                    }
                    SearchItemType::ServerWebsite
                    | SearchItemType::ServerNote
                    | SearchItemType::ServerDatabase
                    | SearchItemType::ServerExtraUserAccount
                    | SearchItemType::ServerPoi => {
                        if level3_item.is_some() {
                            return;
                        }
                        level3_item = Some(item_model);
                    }
                }
                i += 1;
            }
            if let Some(level3) = level3_item {
                Self::display_item_from_search(
                    w.clone(),
                    level3.property("project-id"),
                    level3.property("id"),
                    level3.property("search-item-type"),
                    level3.property("server-id"),
                );
            } else if let Some(level2) = level2_item {
                Self::display_item_from_search(
                    w.clone(),
                    level2.property("project-id"),
                    level2.property("id"),
                    level2.property("search-item-type"),
                    level2.property("server-id"),
                );
            } else if let Some(level1) = level1_item {
                Self::display_item_from_search(
                    w.clone(),
                    level1.property("project-id"),
                    level1.property("id"),
                    level1.property("search-item-type"),
                    level1.property("server-id"),
                );
            }
        });

        win.imp().search_item_list.connect_closure(
            "activate-item",
            false,
            glib::closure_local!(
                #[strong(rename_to = w)]
                win,
                move |_search_item_list: SearchItemList,
                      project_id: i32,
                      item_id: i32,
                      search_item_type: u8,
                      server_id: i32| {
                    Self::display_item_from_search(
                        w.clone(),
                        project_id,
                        item_id,
                        search_item_type,
                        server_id,
                    );
                }
            ),
        );

        // hide top bar when the mouse is over it, so the user can
        // trigger actions underneath it (esp since we make it not take
        // the full width and transparent on the edges)
        let motion_controller = gtk::EventControllerMotion::new();
        let top_bar_view = win.imp().project_toolbar_view.clone();
        let w = win.clone();
        motion_controller.connect_motion(move |_, _x, y| {
            if y <= top_bar_view.top_bar_height().into() {
                w.imp().project_toolbar_view.set_reveal_top_bars(false);
            }
        });
        win.imp()
            .project_toolbar_view
            .add_controller(motion_controller);

        win
    }

    fn search_toggled(w: &ProjectpadApplicationWindow) {
        // new_is_main reflects the state that we want after the toggle
        let new_is_main = w.imp().main_or_search.visible_child_name() == Some("search".into());
        w.imp()
            .main_or_search
            .set_visible_child_name(if new_is_main { "main" } else { "search" });
        w.imp().split_view.set_show_sidebar(new_is_main);

        if !new_is_main {
            w.trigger_search();
        }
    }

    pub fn display_item_from_search(
        w: ProjectpadApplicationWindow,
        project_id: i32,
        item_id: i32,
        search_item_type: u8,
        server_id: i32,
    ) {
        let select_project_param = glib::VariantDict::new(None);
        select_project_param.insert("project_id", project_id);
        //
        select_project_param.insert("item_id", Some(item_id));
        select_project_param.insert("item_type", Some(search_item_type));
        select_project_param.insert("server_id", Some(server_id));
        // select_project_param.insert("item_id", None::<i32>);
        // select_project_param.insert("item_type", None::<u8>);
        // select_project_param.insert("search_item_type", None::<u8>);
        w.change_action_state("select-project-item", &select_project_param.end());

        w.imp().search_toggle_btn.set_active(false);
    }

    fn trigger_search(&self) {
        let (sender, receiver) = async_channel::bounded(1);
        let search_text = self.imp().search_entry.text().as_str().to_owned();

        let search_spec = search_engine::search_parse(&search_text);
        let f = search_spec.search_pattern;
        self.imp()
            .sql_channel
            .borrow()
            .clone()
            .unwrap()
            .send(SqlFunc::new(move |sql_conn| {
                let res = search_engine::run_search_filter(
                    sql_conn,
                    search_engine::SearchItemsType::All,
                    &f,
                    &None,
                    false,
                );
                sender.send_blocking(res).unwrap();
            }))
            .unwrap();
        // let mut s = self.clone();
        let mut sil = self.imp().search_item_list.clone();
        glib::spawn_future_local(async move {
            let search_res = receiver.recv().await.unwrap();
            // probably a switcher for the main window for the search mode and a new search
            // widget
            sil.set_search_items(search_res, None);
        });
    }

    pub fn get_sql_channel(&self) -> mpsc::Sender<SqlFunc> {
        self.imp().sql_channel.borrow().clone().unwrap()
    }

    pub fn display_active_project_item(&self) {
        let project_state =
            glib::VariantDict::new(self.action_state("select-project-item").as_ref());
        let project_id =
            // i32::try_from(project_state.lookup::<i64>("project_id").unwrap().unwrap()).unwrap();
            project_state.lookup::<i32>("project_id").unwrap().unwrap();
        // let server_id = project_state.lookup::<i32>("server_id").unwrap().unwrap();
        let item_id = project_state
            .lookup::<Option<i32>>("item_id")
            .unwrap()
            .unwrap();
        let search_item_type = project_state
            .lookup::<Option<u8>>("item_type")
            .unwrap()
            .and_then(std::convert::identity)
            .and_then(SearchItemType::from_repr);

        let db_sender = self.get_sql_channel();
        let (sender, receiver) = async_channel::bounded::<(Project, Option<i32>, Option<i32>)>(1);
        db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                let project = prj::project
                    .filter(prj::id.eq(project_id))
                    .first::<Project>(sql_conn)
                    .unwrap();

                let server_id =
                    Self::query_search_item_get_server_id(sql_conn, search_item_type, item_id);

                match server_id {
                    None => sender.send_blocking((project, item_id, None)).unwrap(),
                    Some(_) => sender.send_blocking((project, server_id, item_id)).unwrap(),
                }
            }))
            .unwrap();
        let w = self.clone();
        glib::spawn_future_local(async move {
            let (project, project_item_id, server_item_id) = receiver.recv().await.unwrap();
            w.imp().project_menu_button.set_label(&project.name);
            w.imp().project_item_list.get().fetch_project_items(
                &db_sender,
                search_item_type.and_then(|sit| sit.to_project_item_type()),
                project,
                project_item_id,
                server_item_id,
            );
        });
    }

    pub fn query_search_item_get_server_id(
        sql_conn: &mut SqliteConnection,
        search_item_type: Option<SearchItemType>,
        item_id: Option<i32>,
    ) -> Option<i32> {
        match search_item_type {
            // TODO special handling needed for serverlink here?
            Some(SearchItemType::ServerWebsite) => {
                use projectpadsql::schema::server_website::dsl as srv_www;
                srv_www::server_website
                    .filter(srv_www::id.eq(item_id.unwrap()))
                    .select(srv_www::server_id)
                    .first::<i32>(sql_conn)
                    .ok()
            }
            Some(SearchItemType::ServerPoi) => {
                use projectpadsql::schema::server_point_of_interest::dsl as srv_poi;
                srv_poi::server_point_of_interest
                    .filter(srv_poi::id.eq(item_id.unwrap()))
                    .select(srv_poi::server_id)
                    .first::<i32>(sql_conn)
                    .ok()
            }
            Some(SearchItemType::ServerNote) => {
                use projectpadsql::schema::server_note::dsl as srv_note;
                srv_note::server_note
                    .filter(srv_note::id.eq(item_id.unwrap()))
                    .select(srv_note::server_id)
                    .first::<i32>(sql_conn)
                    .ok()
            }
            Some(SearchItemType::ServerDatabase) => {
                use projectpadsql::schema::server_database::dsl as srv_db;
                srv_db::server_database
                    .filter(srv_db::id.eq(item_id.unwrap()))
                    .select(srv_db::server_id)
                    .first::<i32>(sql_conn)
                    .ok()
            }
            Some(SearchItemType::ServerExtraUserAccount) => {
                use projectpadsql::schema::server_extra_user_account::dsl as srv_db;
                srv_db::server_extra_user_account
                    .filter(srv_db::id.eq(item_id.unwrap()))
                    .select(srv_db::server_id)
                    .first::<i32>(sql_conn)
                    .ok()
            }
            // TODO correct to have that catch-all?
            _ => None,
        }
    }

    pub fn get_toast_overlay(&self) -> adw::ToastOverlay {
        self.imp().toast_overlay.get()
    }
}
