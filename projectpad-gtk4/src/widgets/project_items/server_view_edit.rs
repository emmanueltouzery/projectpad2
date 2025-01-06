use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use gtk::subclass::widget::CompositeTemplate;
use projectpadsql::models::{EnvironmentType, ServerAccessType, ServerType};
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
        server_type: Rc<RefCell<String>>,

        #[property(get, set)]
        access_type: Rc<RefCell<String>>,

        #[property(get, set)]
        username: Rc<RefCell<String>>,

        #[property(get, set)]
        password: Rc<RefCell<String>>,

        #[property(get, set)]
        text: Rc<RefCell<String>>,
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
        let server_type = ServerType::from_str(&self.property::<String>("server_type")).unwrap();
        let access_type =
            ServerAccessType::from_str(&self.property::<String>("access_type")).unwrap();
        let username = self.property::<String>("username");
        let password = self.property::<String>("password");
        let text = self.property::<String>("text");

        let address_suffix_www = [SuffixAction::link(&ip)];
        let address = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "ip",
            widget_mode,
            "Address",
            SuffixAction::copy(&ip),
            if access_type == ServerAccessType::SrvAccessWww {
                &address_suffix_www
            } else {
                &[]
            },
        );
        server_item0.add(&address);

        let username = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "username",
            widget_mode,
            "Username",
            SuffixAction::copy(&username),
            &[],
        );
        server_item0.add(&username);

        let password = common::password_row(
            self.upcast_ref::<glib::Object>(),
            "password",
            widget_mode,
            "Password",
            SuffixAction::copy(&password),
            &[],
        );
        server_item0.add(&password);

        let text = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "text",
            widget_mode,
            "Text",
            SuffixAction::copy(&text),
            &[],
        );
        server_item0.add(&text);

        if widget_mode == WidgetMode::Edit {
            // server type
            let server_type_combo = adw::ComboRow::new();
            server_type_combo.set_title("Server Type");
            let server_type_model = gtk::StringList::new(&[
                "Application",
                "Database",
                "HTTP server or proxy",
                "Monitoring",
                "Reporting",
            ]);
            server_type_combo.set_model(Some(&server_type_model));
            server_type_combo.set_selected(server_type as u32);
            server_type_combo
                .bind_property("selected", self, "server_type")
                .transform_to(|_, number: u32| {
                    Some(
                        ServerType::from_repr(number.try_into().unwrap())
                            .unwrap()
                            .to_string()
                            .to_value(),
                    )
                })
                .sync_create()
                .build();

            server_item0.add(&server_type_combo);

            // access type
            let access_type_combo = adw::ComboRow::new();
            access_type_combo.set_title("Access Type");
            let access_type_model =
                gtk::StringList::new(&["Remote Desktop (RDP)", "SSH", "SSH Tunnel", "Website"]);
            access_type_combo.set_model(Some(&access_type_model));
            access_type_combo.set_selected(access_type as u32);
            access_type_combo
                .bind_property("selected", self, "access_type")
                .transform_to(|_, number: u32| {
                    Some(
                        ServerAccessType::from_repr(number.try_into().unwrap())
                            .unwrap()
                            .to_string()
                            .to_value(),
                    )
                })
                .sync_create()
                .build();

            server_item0.add(&access_type_combo);
        }

        vbox.append(&server_item0);

        self.set_child(Some(&vbox));
    }
}
