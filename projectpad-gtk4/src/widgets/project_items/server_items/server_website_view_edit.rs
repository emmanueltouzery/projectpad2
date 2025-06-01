use adw::prelude::*;
use diesel::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

use crate::{
    search_engine::SearchItemsType,
    widgets::{
        project_item::WidgetMode,
        project_items::{
            common::{self, SuffixAction},
            projectpad_item_action_row::ProjectpadItemActionRow,
        },
        search::search_item_model::SearchItemType,
    },
};

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::ServerWebsiteViewEdit)]
    pub struct ServerWebsiteViewEdit {
        #[property(get, set)]
        url: Rc<RefCell<String>>,

        #[property(get, set)]
        username: Rc<RefCell<String>>,

        #[property(get, set)]
        password: Rc<RefCell<String>>,

        #[property(get, set)]
        text: Rc<RefCell<String>>,

        #[property(get, set)]
        database_id: Rc<RefCell<i32>>,

        #[property(get, set)]
        database_desc: Rc<RefCell<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerWebsiteViewEdit {
        const NAME: &'static str = "ServerWebsiteViewEdit";
        type ParentType = adw::Bin;
        type Type = super::ServerWebsiteViewEdit;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServerWebsiteViewEdit {
        fn constructed(&self) {}
    }

    impl WidgetImpl for ServerWebsiteViewEdit {}

    impl adw::subclass::prelude::BinImpl for ServerWebsiteViewEdit {}
}

glib::wrapper! {
    pub struct ServerWebsiteViewEdit(ObjectSubclass<imp::ServerWebsiteViewEdit>)
        @extends gtk::Widget, adw::Bin;
}

impl ServerWebsiteViewEdit {
    pub fn new() -> Self {
        let this = glib::Object::new::<Self>();
        this
    }

    // call this after setting all the properties
    pub fn prepare(&self, widget_mode: WidgetMode) {
        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(20)
            .build();
        let server_item0 = adw::PreferencesGroup::builder().build();

        let url = self.url();
        let url_row = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "url",
            widget_mode,
            "url",
            SuffixAction::copy(&url),
            &[SuffixAction::link(&url)],
        );
        server_item0.add(&url_row);

        let username = self.username();
        let username_row = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "username",
            widget_mode,
            "Username",
            SuffixAction::copy(&username),
            &[],
        );
        server_item0.add(&username_row);

        let password = self.password();
        let password_row = common::password_row(
            self.upcast_ref::<glib::Object>(),
            "password",
            widget_mode,
            "Password",
            SuffixAction::copy(&password),
            &[],
        );
        server_item0.add(&password_row);

        let text = self.text();
        let text_row = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "text",
            widget_mode,
            "Text",
            SuffixAction::copy(&text),
            &[],
        );
        server_item0.add(&text_row);

        if widget_mode == WidgetMode::Edit || self.database_id() > 0 {
            let projectpad_item_action_row = ProjectpadItemActionRow::new(widget_mode);
            projectpad_item_action_row
                .set_search_items_type(SearchItemsType::ServerDbsOnly.to_string());
            projectpad_item_action_row.set_search_item_type(SearchItemType::ServerDatabase as u8);
            projectpad_item_action_row.set_item_id(self.database_id());
            projectpad_item_action_row.set_text(self.database_desc());
            projectpad_item_action_row.connect_closure(
                "item-picked",
                false,
                glib::closure_local!(
                    #[strong(rename_to = s)]
                    self,
                    move |item_action_row: ProjectpadItemActionRow, item_id: i32| {
                        let db_name_recv = common::run_sqlfunc(Box::new(move |sql_conn| {
                            if item_id <= 0 {
                                "".to_string()
                            } else {
                                use projectpadsql::schema::server_database::dsl as srv_db;

                                srv_db::server_database
                                    .filter(srv_db::id.eq(&item_id))
                                    .select(srv_db::desc)
                                    .first::<String>(sql_conn)
                                    .unwrap()
                            }
                        }));
                        let s = s.clone();
                        glib::spawn_future_local(async move {
                            let db_name = db_name_recv.recv().await.unwrap();
                            let _guard = s.freeze_notify();
                            item_action_row.set_item_id(item_id);
                            item_action_row.set_text(db_name.clone());
                            s.set_database_id(item_id);
                            s.set_database_desc(db_name);
                        });
                    }
                ),
            );
            server_item0.add(&projectpad_item_action_row);
        }

        vbox.append(&server_item0);

        self.set_child(Some(&vbox));
    }
}
