use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

use crate::widgets::{
    project_item::WidgetMode,
    project_items::common::{self, SuffixAction},
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

        // TODO server database

        vbox.append(&server_item0);

        self.set_child(Some(&vbox));
    }
}
