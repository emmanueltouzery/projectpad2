use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;
use projectpadsql::models::{ServerAccessType, ServerType};
use std::str::FromStr;

use crate::widgets::project_item::WidgetMode;

use super::{
    common::{self, SuffixAction},
    file_picker_action_row::FilePickerActionRow,
};

mod imp {
    use std::{cell::RefCell, rc::Rc, sync::OnceLock};

    use super::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };
    use subclass::Signal;

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::ServerViewEdit)]
    pub struct ServerViewEdit {
        #[property(get, set)]
        ip: Rc<RefCell<String>>,

        #[property(get, set)]
        is_retired: Rc<RefCell<bool>>,

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

        #[property(get, set)]
        auth_key_filename: Rc<RefCell<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerViewEdit {
        const NAME: &'static str = "ServerViewEdit";
        type ParentType = adw::Bin;
        type Type = super::ServerViewEdit;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServerViewEdit {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("save-auth-key-to-disk")
                    .param_types([String::static_type()])
                    .build()]
            })
        }
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

        let ip = self.ip();
        let is_retired = self.is_retired();
        let server_type = ServerType::from_str(&self.server_type()).unwrap();
        let access_type = ServerAccessType::from_str(&self.access_type()).unwrap();
        let username = self.username();
        let password = self.password();
        let text = self.text();
        let auth_key_filename = self
            .property::<Option<String>>("auth_key_filename")
            .filter(|s| !s.is_empty());

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

        if widget_mode == WidgetMode::Edit || is_retired {
            let retired_switch = adw::SwitchRow::builder()
                .title("Retired")
                .active(is_retired)
                .build();
            server_item0.add(&retired_switch);
        }

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

        // dbg!(&auth_key_filename);
        if widget_mode == WidgetMode::Edit || auth_key_filename.is_some() {
            let auth_key_entry = FilePickerActionRow::new(widget_mode);
            auth_key_entry.set_title("Authentication key");
            if let Some(k) = auth_key_filename {
                auth_key_entry.set_filename(k);
            }
            auth_key_entry
                .bind_property("filename", self, "auth_key_filename")
                .sync_create()
                .build();

            if widget_mode == WidgetMode::Show {
                auth_key_entry.connect_closure(
                    "file-picked",
                    false,
                    glib::closure_local!(@strong self as s => move |_: FilePickerActionRow, p: String| {
                        s.emit_by_name::<()>("save-auth-key-to-disk", &[&p]);
                    }),
                );
            }

            server_item0.add(&auth_key_entry);
        }

        let text = common::text_row(
            self.upcast_ref::<glib::Object>(),
            "text",
            widget_mode,
            "Text",
            SuffixAction::copy(&text),
            &[],
        );
        server_item0.add(&text);

        let server_type_row = common::combo_row(
            self.upcast_ref::<glib::Object>(),
            "server_type",
            widget_mode,
            "Server Type",
            &[
                "Application",
                "Database",
                "HTTP server or proxy",
                "Monitoring",
                "Reporting",
            ],
            |v| ServerType::from_str(&v.get::<String>().unwrap()).unwrap() as u8 as u32,
            |i| {
                ServerType::from_repr(TryInto::<u8>::try_into(i).unwrap())
                    .unwrap()
                    .to_string()
                    .to_value()
            },
        );
        server_item0.add(&server_type_row);

        let access_type_row = common::combo_row(
            self.upcast_ref::<glib::Object>(),
            "access_type",
            widget_mode,
            "Access Type",
            &["Remote Desktop (RDP)", "SSH", "SSH Tunnel", "Website"],
            |v| ServerAccessType::from_str(&v.get::<String>().unwrap()).unwrap() as u8 as u32,
            |i| {
                ServerAccessType::from_repr(TryInto::<u8>::try_into(i).unwrap())
                    .unwrap()
                    .to_string()
                    .to_value()
            },
        );
        server_item0.add(&access_type_row);

        vbox.append(&server_item0);

        self.set_child(Some(&vbox));
    }
}
