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
    #[properties(wrapper_type = super::ServerExtraUserAccountViewEdit)]
    pub struct ServerExtraUserAccountViewEdit {
        #[property(get, set)]
        username: Rc<RefCell<String>>,

        #[property(get, set)]
        password: Rc<RefCell<String>>,
        // TODO other fields. at least auth key
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerExtraUserAccountViewEdit {
        const NAME: &'static str = "ServerExtraUserAccountViewEdit";
        type ParentType = adw::Bin;
        type Type = super::ServerExtraUserAccountViewEdit;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServerExtraUserAccountViewEdit {
        fn constructed(&self) {}
    }

    impl WidgetImpl for ServerExtraUserAccountViewEdit {}

    impl adw::subclass::prelude::BinImpl for ServerExtraUserAccountViewEdit {}
}

glib::wrapper! {
    pub struct ServerExtraUserAccountViewEdit(ObjectSubclass<imp::ServerExtraUserAccountViewEdit>)
        @extends gtk::Widget, adw::Bin;
}

impl ServerExtraUserAccountViewEdit {
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

        vbox.append(&server_item0);

        self.set_child(Some(&vbox));
    }
}
