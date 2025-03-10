use adw::prelude::*;
use glib::*;
use gtk::subclass::prelude::*;

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::PasswordActionRow)]
    pub struct PasswordActionRow {
        #[property(get, set)]
        text: Rc<RefCell<String>>,

        // #[property(get, set, override_class = adw::ActionRow)]
        // subtitle: Rc<RefCell<String>>,
        #[property(get, set)]
        show_password: Rc<RefCell<bool>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PasswordActionRow {
        const NAME: &'static str = "PasswordActionRow";
        type ParentType = adw::ActionRow;
        type Type = super::PasswordActionRow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PasswordActionRow {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().upcast_ref::<adw::ActionRow>();

            // self.obj()
            //     .bind_property(
            //         "text",
            //         self.obj().upcast_ref::<adw::PreferencesRow>(),
            //         "subtitle",
            //     )
            //     .transform_to(|w, str| {
            //         dbg!(if w.source().unwrap().property("show_password") {
            //             Some(glib::markup_escape_text(str).to_value())
            //         } else {
            //             Some("●●●●".to_value())
            //         })
            //     })
            //     .sync_create()
            //     .build();
        }
    }

    impl WidgetImpl for PasswordActionRow {}

    impl gtk::subclass::prelude::ListBoxRowImpl for PasswordActionRow {}
    impl adw::subclass::prelude::PreferencesRowImpl for PasswordActionRow {}
    impl adw::subclass::prelude::ActionRowImpl for PasswordActionRow {}
}

glib::wrapper! {
    pub struct PasswordActionRow(ObjectSubclass<imp::PasswordActionRow>)
        @extends gtk::Widget, adw::PreferencesRow, adw::ActionRow;
}

impl PasswordActionRow {
    pub fn new() -> Self {
        let this = glib::Object::new::<Self>();

        // .title(glib::markup_escape_text(self.title))
        // .subtitle(glib::markup_escape_text(subtitle))
        // https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/boxed-lists.html#property-rows
        // When used together with the .property style class, AdwActionRow and
        // AdwExpanderRow deemphasize their title and emphasize their subtitle instead
        this.set_css_classes(&["property"]);

        let widget = gtk::Button::builder()
            .css_classes(["flat"])
            .icon_name("view-reveal-symbolic")
            .build();
        widget.connect_closure(
            "clicked",
            false,
            glib::closure_local!(@strong this as s => move |b: gtk::Button| {
                if b.icon_name() == Some("view-reveal-symbolic".into()) {
                    s.set_property("show_password", true);
                    // force subtitle refresh since the hide_password changed
                    s.upcast_ref::<adw::ActionRow>().set_subtitle(&s.property::<String>("text"));
                    // ar.set_subtitle(&st);
                    b.set_icon_name("view-conceal-symbolic");
                } else {
                    s.set_property("show_password", false);
                    // force subtitle refresh since the hide_password changed
                    // s.set_property("subtitle", s.property::<String>("subtitle"));
                    s.upcast_ref::<adw::ActionRow>().set_subtitle("●●●●");
                    // ar.set_subtitle("●●●●");
                    b.set_icon_name("view-reveal-symbolic");
                }
            }),
        );
        this.add_suffix(&widget);

        this.bind_property("text", this.upcast_ref::<adw::PreferencesRow>(), "subtitle")
            .transform_to(|w, str| {
                if w.source().unwrap().property("show_password") {
                    Some(glib::markup_escape_text(str).to_value())
                } else {
                    Some("●●●●".to_value())
                }
            })
            .sync_create()
            .build();
        this
    }
}
