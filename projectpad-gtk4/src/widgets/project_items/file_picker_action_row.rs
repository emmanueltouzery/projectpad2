use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

use crate::widgets::project_item::WidgetMode;

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };

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
            glib::closure_local!(move |b: gtk::Button| {
                if b.icon_name() == Some("document-open-symbolic".into()) {
                } else {
                }
            }),
        );
        this.add_suffix(&widget);

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
