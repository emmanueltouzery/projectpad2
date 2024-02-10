use std::panic;
use std::sync::mpsc;

use crate::search_engine;
use crate::sql_thread::SqlFunc;
use crate::widgets::search::search_item_list::SearchItemList;

use super::widgets::project_item_list::ProjectItemList;
use adw::subclass::prelude::*;
use diesel::prelude::*;
use glib::subclass;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use projectpadsql::models::Project;

mod imp {
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
    };

    use crate::widgets::{project_item::ProjectItem, search::search_item_list::SearchItemList};

    use super::*;
    use glib::{subclass::prelude::ObjectImpl, ObjectExt, Properties};
    use gtk::{
        subclass::{
            prelude::ObjectSubclass,
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Properties, Debug, Default, CompositeTemplate)]
    #[properties(wrapper_type = super::ProjectpadApplicationWindow)]
    #[template(resource = "/com/github/emmanueltouzery/projectpad2/src/win.ui")]
    pub struct ProjectpadApplicationWindow {
        #[template_child]
        pub project_item_list: TemplateChild<ProjectItemList>,
        #[template_child]
        pub project_item: TemplateChild<ProjectItem>,
        #[template_child]
        pub edit_btn: TemplateChild<gtk::Button>,
        #[template_child]
        pub edit_btn_contents: TemplateChild<adw::ButtonContent>,
        #[template_child]
        pub project_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub project_popover_menu: TemplateChild<gtk::PopoverMenu>,
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

        #[property(get, set)]
        edit_mode: Cell<bool>,

        pub sql_channel: RefCell<Option<mpsc::Sender<SqlFunc>>>,
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

    #[glib::derived_properties]
    impl ObjectImpl for ProjectpadApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();

            self.project_item_list
                .get()
                .set_project_items(&Vec::new(), HashMap::new(), None);
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
        win.imp().project_item_list.connect_activate(
            glib::clone!(@weak win as w => move |project_item_id, project_item_type| {
                w.imp().project_item.set_project_item_type(project_item_type as u8);
                w.imp().project_item.set_item_id(project_item_id)
            }),
        );

        win.imp().search_entry.connect_show(|entry| {
            entry.grab_focus();
        });

        win.imp()
            .search_toggle_btn
            .connect_clicked(glib::clone!(@weak win as w => move |_| {
                // new_is_main reflects the state that we want after the toggle
                let new_is_main = w.imp().main_or_search.visible_child_name() == Some("search".into());
                w.imp()
                    .main_or_search
                    .set_visible_child_name(if new_is_main { "main" } else { "search" });
                w.imp()
                    .split_view.set_show_sidebar(new_is_main);
            }));

        win.imp().search_entry.connect_search_changed(
            glib::clone!(@weak win as w => move |entry| {
                let (sender, receiver) = async_channel::bounded(1);
                let search_text = entry.text().as_str().to_owned();

                let search_spec = search_engine::search_parse(&search_text);
                let f = search_spec.search_pattern;
                db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        let res = search_engine::run_search_filter(sql_conn, search_engine::SearchItemsType::All, &f, &None, false);
                        sender.send_blocking(res).unwrap();
                    }))
                    .unwrap();
                // let mut s = self.clone();
                    let mut sil = w.imp().search_item_list.clone();
                glib::spawn_future_local(async move {
                    let search_res = receiver.recv().await.unwrap();
                    // probably a switcher for the main window for the search mode and a new search
                    // widget
                    sil.set_search_items(search_res);

                });
                }
            ),
        );

        win.imp().search_item_list.connect_closure(
            "activate-item",
            false,
            glib::closure_local!(@strong win as w => move |_search_item_list: SearchItemList, project_id: i32, item_id: i32, search_item_type: u8| {
                w.imp().split_view.set_show_sidebar(true);
                w.imp()
                    .main_or_search
                    .set_visible_child_name("main");

                w.set_active_project_and_item(project_id, Some((item_id, search_item_type)));

                w.imp()
                    .search_toggle_btn.set_active(false);
            }),
            );

        win.bind_property(
            "edit-mode",
            win.imp().edit_btn_contents.upcast_ref::<gtk::Widget>(),
            "label",
        )
        .transform_to(|_, active: bool| Some(if active { "View" } else { "Edit" }.to_value()))
        .sync_create()
        .build();

        win.bind_property(
            "edit-mode",
            win.imp().edit_btn_contents.upcast_ref::<gtk::Widget>(),
            "icon-name",
        )
        .transform_to(|_, active: bool| {
            Some(
                if active {
                    "view-reveal-symbolic"
                } else {
                    "document-edit-symbolic"
                }
                .to_value(),
            )
        })
        .sync_create()
        .build();

        win.bind_property(
            "edit-mode",
            win.imp().edit_btn.upcast_ref::<gtk::Widget>(),
            "css-classes",
        )
        .transform_to(|_, active: bool| {
            Some(
                if active {
                    ["pill", "suggested-action"]
                } else {
                    ["pill", "destructive-action"]
                }
                .to_value(),
            )
        })
        .sync_create()
        .build();

        win.bind_property("edit-mode", &win.imp().project_item.get(), "edit_mode")
            .build();

        win.imp()
            .edit_btn
            .connect_clicked(glib::clone!(@weak win as w => move |_| {
                let edit_mode = w.property::<bool>("edit-mode");
                w.set_property("edit-mode", (!edit_mode).to_value());
            }));

        win
    }

    pub fn get_sql_channel(&self) -> mpsc::Sender<SqlFunc> {
        self.imp().sql_channel.borrow().clone().unwrap()
    }

    pub fn set_active_project_and_item(&self, project_id: i32, selected_item: Option<(i32, u8)>) {
        let db_sender = self.get_sql_channel();
        let (sender, receiver) = async_channel::bounded::<Project>(1);
        db_sender
            .send(SqlFunc::new(move |sql_conn| {
                use projectpadsql::schema::project::dsl as prj;
                let project = prj::project
                    .filter(prj::id.eq(project_id))
                    .first::<Project>(sql_conn)
                    .unwrap();
                sender.send_blocking(project).unwrap();
            }))
            .unwrap();
        let w = self.clone();
        glib::spawn_future_local(async move {
            let project = receiver.recv().await.unwrap();
            w.imp().project_menu_button.set_label(&project.name);
        });
        self.imp().project_item_list.get().fetch_project_items(
            &db_sender,
            project_id,
            selected_item.map(|(id, _type)| id),
        );
        if let Some((selected_id, selected_item_type)) = selected_item {
            self.imp()
                .project_item
                .set_project_item_type(selected_item_type);
            self.imp().project_item.set_item_id(selected_id)
        }
    }

    pub fn get_toast_overlay(&self) -> adw::ToastOverlay {
        self.imp().toast_overlay.get()
    }
}
