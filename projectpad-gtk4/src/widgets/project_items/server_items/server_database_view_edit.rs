use adw::prelude::*;
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
    #[properties(wrapper_type = super::ServerDatabaseViewEdit)]
    pub struct ServerDatabaseViewEdit {
        #[property(get, set)]
        name: Rc<RefCell<String>>,

        #[property(get, set)]
        username: Rc<RefCell<String>>,

        #[property(get, set)]
        password: Rc<RefCell<String>>,

        #[property(get, set)]
        text: Rc<RefCell<String>>,

        #[property(get, set)]
        website_names: Rc<RefCell<Vec<String>>>,

        // should be Vec<i32> but not supported by gobject
        // https://gtk-rs.org/gtk-rs-core/stable/latest/docs/glib/trait.HasParamSpec.html#foreign-impls
        #[property(get, set)]
        website_ids: Rc<RefCell<Vec<String>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerDatabaseViewEdit {
        const NAME: &'static str = "ServerDatabaseViewEdit";
        type ParentType = adw::Bin;
        type Type = super::ServerDatabaseViewEdit;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServerDatabaseViewEdit {
        fn constructed(&self) {}
    }

    impl WidgetImpl for ServerDatabaseViewEdit {}

    impl adw::subclass::prelude::BinImpl for ServerDatabaseViewEdit {}
}

glib::wrapper! {
    pub struct ServerDatabaseViewEdit(ObjectSubclass<imp::ServerDatabaseViewEdit>)
        @extends gtk::Widget, adw::Bin;
}

impl ServerDatabaseViewEdit {
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

        let name = self.property::<String>("name");
        let name_row = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "name",
            widget_mode,
            "name",
            SuffixAction::copy(&name),
            &[],
        );
        server_item0.add(&name_row);

        let username = self.property::<String>("username");
        let username_row = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "username",
            widget_mode,
            "Username",
            SuffixAction::copy(&username),
            &[],
        );
        server_item0.add(&username_row);

        let password = self.property::<String>("password");
        let password_row = common::password_row(
            self.upcast_ref::<glib::Object>(),
            "password",
            widget_mode,
            "Password",
            SuffixAction::copy(&password),
            &[],
        );
        server_item0.add(&password_row);

        let text = self.property::<String>("text");
        let text_row = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "text",
            widget_mode,
            "Text",
            SuffixAction::copy(&text),
            &[],
        );
        server_item0.add(&text_row);

        if widget_mode == WidgetMode::Show {
            for (www_name, www_id_str) in self.website_names().iter().zip(self.website_ids().iter())
            {
                let www_id = www_id_str.parse().unwrap_or(0);
                let action_row = ProjectpadItemActionRow::new(widget_mode);
                action_row.set_text(www_name.to_owned());
                action_row.set_search_items_type(SearchItemsType::All.to_string()); // only used in edit mode, so
                                                                                    // not here
                action_row.set_search_item_type(SearchItemType::ServerWebsite as u8);
                action_row.set_item_id(www_id);
                server_item0.add(&action_row);
            }
        }

        vbox.append(&server_item0);

        self.set_child(Some(&vbox));
    }
}
