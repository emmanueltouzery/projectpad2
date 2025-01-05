use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use projectpadsql::models::{EnvironmentType, ServerAccessType};
use std::str::FromStr;

use crate::widgets::project_item::WidgetMode;

use super::{
    common::{self, DetailsRow, PasswordMode, SuffixAction},
    password_action_row::PasswordActionRow,
};

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use gtk::{
        subclass::{
            prelude::{ObjectImpl, ObjectSubclass},
            widget::{CompositeTemplateInitializingExt, WidgetImpl},
        },
        CompositeTemplate, TemplateChild,
    };

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::ServerViewEdit)]
    pub struct ServerViewEdit {
        #[property(get, set)]
        ip: Rc<RefCell<String>>,

        #[property(get, set)]
        access_type: Rc<RefCell<String>>,

        #[property(get, set)]
        username: Rc<RefCell<String>>,

        #[property(get, set)]
        password: Rc<RefCell<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerViewEdit {
        const NAME: &'static str = "ServerViewEdit";
        type ParentType = adw::Bin;
        type Type = super::ServerViewEdit;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServerViewEdit {
        fn constructed(&self) {}
    }

    impl WidgetImpl for ServerViewEdit {}

    impl adw::subclass::prelude::BinImpl for ServerViewEdit {}
}

glib::wrapper! {
    pub struct ServerViewEdit(ObjectSubclass<imp::ServerViewEdit>)
        @extends gtk::Widget, adw::Bin;
}

impl ServerViewEdit {
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

        let ip = self.property::<String>("ip");
        let access_type =
            ServerAccessType::from_str(&self.property::<String>("access_type")).unwrap();
        let username = self.property::<String>("username");
        let password = self.property::<String>("password");

        let address_suffix_www = [SuffixAction::link(&ip)];
        let (address, address_val_prop) = common::text_row(
            widget_mode,
            "Address",
            SuffixAction::copy(&ip),
            if access_type == ServerAccessType::SrvAccessWww {
                &address_suffix_www
            } else {
                &[]
            },
        );
        self.bind_property("ip", &address, address_val_prop)
            .bidirectional()
            .sync_create()
            .build();
        server_item0.add(&address);

        let (username, username_val_prop) =
            common::text_row(widget_mode, "Username", SuffixAction::copy(&username), &[]);
        self.bind_property("username", &username, username_val_prop)
            .bidirectional()
            .sync_create()
            .build();
        server_item0.add(&username);

        let (password, password_val_prop) =
            common::password_row(widget_mode, "Password", SuffixAction::copy(&password), &[]);
        self.bind_property("password", &password, password_val_prop)
            .bidirectional()
            .sync_create()
            .build();
        server_item0.add(&password);

        vbox.append(&server_item0);

        self.set_child(Some(&vbox));
    }
}
