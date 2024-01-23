use std::panic;
use std::sync::mpsc;

use crate::search_engine;
use crate::sql_thread::SqlFunc;

use super::widgets::project_item_list::ProjectItemList;
use adw::subclass::prelude::*;
use glib::subclass;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::widget::CompositeTemplate;

mod imp {
    use std::collections::HashMap;

    use crate::widgets::{project_item::ProjectItem, search::search_item_list::SearchItemList};

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
        #[template_child]
        pub search_toggle_btn: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_item_list: TemplateChild<SearchItemList>,
        #[template_child]
        pub main_or_search: TemplateChild<gtk::Stack>,
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

            self.project_item_list
                .get()
                .set_project_items(&Vec::new(), HashMap::new());
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

impl ProjectpadApplicationWindow {
    pub fn new(db_sender: mpsc::Sender<SqlFunc>) -> Self {
        let win = glib::Object::new::<Self>();
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
                w.imp()
                    .main_or_search
                    .set_visible_child_name(if w.imp().main_or_search.visible_child_name() == Some("main".into()) { "search" } else { "main" });
            }));

        win.imp().search_entry.connect_search_changed(
            glib::clone!(@weak win as w => move |entry| {
                        dbg!(entry.text());

                let (sender, receiver) = async_channel::bounded(1);
                let search_text = entry.text().as_str().to_owned();
                db_sender
                    .send(SqlFunc::new(move |sql_conn| {
                        let res = search_engine::run_search_filter(sql_conn, search_engine::SearchItemsType::All, &search_text, &None, false);
                        sender.send_blocking(res).unwrap();
                    }))
                    .unwrap();
                // let mut s = self.clone();
                glib::spawn_future_local(async move {
                    let search_res = receiver.recv().await.unwrap();
                    // probably a switcher for the main window for the search mode and a new search
                    // widget
                    dbg!(&search_res);
                });
                }
            ),
        );

        win
    }
}
