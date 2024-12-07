use adw::prelude::*;
use gtk::{gdk, subclass::prelude::*};

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::{subclass::Signal, types::StaticType};
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };

    #[derive(Debug, Default)]
    pub struct SearchBar {
        pub search_entry: RefCell<gtk::SearchEntry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchBar {
        const NAME: &'static str = "SearchBar";
        type ParentType = adw::Bin;
        type Type = super::SearchBar;
    }

    impl ObjectImpl for SearchBar {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("esc-pressed").build(),
                    Signal::builder("prev-pressed")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("next-pressed")
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("search-changed")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for SearchBar {}

    impl adw::subclass::prelude::BinImpl for SearchBar {}
}

glib::wrapper! {
    pub struct SearchBar(ObjectSubclass<imp::SearchBar>)
        @extends gtk::Widget, adw::Bin;

}

impl SearchBar {
    pub fn new() -> Self {
        let this = glib::Object::new::<Self>();

        let hbox = gtk::Box::builder().css_classes(["linked"]).build();

        let search_entry = gtk::SearchEntry::builder()
            .margin_start(5)
            .margin_top(5)
            .margin_bottom(5)
            .build();
        hbox.append(&search_entry);

        let t1 = this.clone();
        search_entry.connect_search_changed(move |entry| {
            t1.emit_by_name::<()>("search-changed", &[&entry.text()]);
        });

        this.imp().search_entry.replace(search_entry.clone());

        let key_controller = gtk::EventControllerKey::new();
        let t = this.clone();
        key_controller.connect_key_pressed(move |_controller, keyval, _keycode, _state| {
            if keyval == gdk::Key::Escape {
                t.emit_by_name::<()>("esc-pressed", &[]);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed // Allow other handlers to process the event
        });
        search_entry.add_controller(key_controller);

        let t1 = this.clone();
        search_entry.connect_activate(move |se| {
            dbg!("enter!!");
            t1.emit_by_name::<()>("next-pressed", &[&se.text()]);
        });

        let prev_btn = gtk::Button::builder()
            .margin_top(5)
            .margin_bottom(5)
            .icon_name("go-up")
            .build();
        hbox.append(&prev_btn);

        let t3 = this.clone();
        let se2 = search_entry.clone();
        prev_btn.connect_clicked(move |_| {
            t3.emit_by_name::<()>("prev-pressed", &[&se2.text()]);
        });

        let next_btn = gtk::Button::builder()
            .margin_top(5)
            .margin_bottom(5)
            .icon_name("go-down")
            .build();
        hbox.append(&next_btn);

        let t4 = this.clone();
        let se3 = search_entry.clone();
        next_btn.connect_clicked(move |_| {
            t4.emit_by_name::<()>("next-pressed", &[&se3.text()]);
        });

        this.set_child(Some(&hbox));
        // https://discourse.gnome.org/t/overlay-widget-is-on-top-of-contents-but-translucent/25490/2
        this.set_css_classes(&["view"]);
        this
    }

    pub fn grab_focus(&self) {
        self.imp().search_entry.borrow().grab_focus();
    }
}
