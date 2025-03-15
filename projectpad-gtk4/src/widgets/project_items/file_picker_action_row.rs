use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

use crate::{app::ProjectpadApplication, widgets::project_item::WidgetMode};

mod imp {
    use std::{cell::RefCell, rc::Rc, sync::OnceLock};

    use super::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };
    use subclass::Signal;

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::FilePickerActionRow)]
    pub struct FilePickerActionRow {
        #[property(get, set)]
        text: Rc<RefCell<String>>,

        #[property(get, set)]
        filename: Rc<RefCell<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FilePickerActionRow {
        const NAME: &'static str = "FilePickerActionRow";
        type ParentType = adw::ActionRow;
        type Type = super::FilePickerActionRow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FilePickerActionRow {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("file-picked")
                    .param_types([String::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for FilePickerActionRow {}

    impl gtk::subclass::prelude::ListBoxRowImpl for FilePickerActionRow {}
    impl adw::subclass::prelude::PreferencesRowImpl for FilePickerActionRow {}
    impl adw::subclass::prelude::ActionRowImpl for FilePickerActionRow {}
}

glib::wrapper! {
    pub struct FilePickerActionRow(ObjectSubclass<imp::FilePickerActionRow>)
        @extends gtk::Widget, adw::PreferencesRow, adw::ActionRow;
}

impl FilePickerActionRow {
    pub fn new(widget_mode: WidgetMode) -> Self {
        let this = glib::Object::new::<Self>();

        // this.bind_property("text", this.upcast_ref::<adw::PreferencesRow>(), "text")
        //     .sync_create()
        //     .build();

        // .title(glib::markup_escape_text(self.title))
        // .subtitle(glib::markup_escape_text(subtitle))
        // https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/boxed-lists.html#property-rows
        // When used together with the .property style class, AdwActionRow and
        // AdwExpanderRow deemphasize their title and emphasize their subtitle instead
        this.set_css_classes(&["property"]);

        let widget = gtk::Button::builder()
            .css_classes(["flat"])
            .icon_name(if widget_mode == WidgetMode::Show {
                "document-save-symbolic"
            } else {
                "document-open-symbolic"
            })
            .build();
        widget.connect_closure(
            "clicked",
            false,
            glib::closure_local!(@strong this as s => move |b: gtk::Button| {
                let app = gio::Application::default()
                    .expect("Failed to retrieve application singleton")
                    .downcast::<ProjectpadApplication>()
                    .unwrap();
                let window = app.imp().window.get().unwrap();
                let win_binding = window.upgrade();
                let win_binding_ref = win_binding.as_ref().unwrap();
                let file_dialog = gtk::FileDialog::builder().build();
                if b.icon_name() == Some("document-open-symbolic".into()) {
                    let _s = s.clone();
                    file_dialog.open(Some(win_binding_ref), None::<&gio::Cancellable>, move |r| {
                        if let Ok(f) = r {
                            if let Some(p) = f.path() {
                                // TODO a little crappy unwrap, could be invalid filename
                                _s.set_filename(p.to_str().unwrap());
                            }
                        }
                    });
                } else {
                    let _s = s.clone();
                    file_dialog.save(Some(win_binding_ref), None::<&gio::Cancellable>, move |r| {
                        if let Ok(f) = r {
                            if let Some(p) = f.path() {
                                dbg!(&p);
                                _s.emit_by_name::<()>("file-picked", &[&p.display().to_string()]);
                            }
                        }
                    });
                }
            }),
        );
        this.add_suffix(&widget);

        if widget_mode == WidgetMode::Edit {
            let delete_widget = gtk::Button::builder()
                .css_classes(["flat"])
                .icon_name("edit-delete-symbolic")
                .build();
            this.add_suffix(&delete_widget);
            delete_widget.connect_closure(
                "clicked",
                false,
                glib::closure_local!(@strong this as s => move |_b: gtk::Button| {
                    s.set_filename("");
                }),
            );
        }

        this.set_activatable_widget(Some(&widget));

        this.bind_property(
            "filename",
            this.upcast_ref::<adw::PreferencesRow>(),
            "subtitle",
        )
        .sync_create()
        .build();

        this
    }
}
