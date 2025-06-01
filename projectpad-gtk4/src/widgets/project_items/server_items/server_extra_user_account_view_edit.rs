use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

use crate::widgets::{
    project_item::WidgetMode,
    project_items::{
        common::{self, SuffixAction},
        file_picker_action_row::FilePickerActionRow,
    },
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
    #[properties(wrapper_type = super::ServerExtraUserAccountViewEdit)]
    pub struct ServerExtraUserAccountViewEdit {
        #[property(get, set)]
        username: Rc<RefCell<String>>,

        #[property(get, set)]
        password: Rc<RefCell<String>>,

        #[property(get, set)]
        auth_key_filename: Rc<RefCell<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerExtraUserAccountViewEdit {
        const NAME: &'static str = "ServerExtraUserAccountViewEdit";
        type ParentType = adw::Bin;
        type Type = super::ServerExtraUserAccountViewEdit;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServerExtraUserAccountViewEdit {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("save-auth-key-to-disk")
                    .param_types([String::static_type()])
                    .build()]
            })
        }
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

        let auth_key_filename = self
            .property::<Option<String>>("auth_key_filename")
            .filter(|s| !s.is_empty());

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
                    glib::closure_local!(
                        #[strong(rename_to = s)]
                        self,
                        move |_: FilePickerActionRow, p: String| {
                            s.emit_by_name::<()>("save-auth-key-to-disk", &[&p]);
                        }
                    ),
                );
            }

            server_item0.add(&auth_key_entry);
        }

        vbox.append(&server_item0);

        self.set_child(Some(&vbox));
    }
}
