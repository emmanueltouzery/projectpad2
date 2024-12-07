use adw::prelude::*;
use gtk::{gdk, subclass::prelude::*};

mod imp {
    use std::{cell::RefCell, sync::OnceLock};

    use glib::subclass::Signal;
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
            SIGNALS.get_or_init(|| vec![Signal::builder("esc-pressed").build()])
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

        let prev_btn = gtk::Button::builder()
            .margin_top(5)
            .margin_bottom(5)
            .icon_name("go-up")
            .build();
        hbox.append(&prev_btn);

        let next_btn = gtk::Button::builder()
            .margin_top(5)
            .margin_bottom(5)
            .icon_name("go-down")
            .build();
        hbox.append(&next_btn);

        this.set_child(Some(&hbox));
        // https://discourse.gnome.org/t/overlay-widget-is-on-top-of-contents-but-translucent/25490/2
        this.set_css_classes(&["view"]);
        this
    }

    pub fn grab_focus(&self) {
        dbg!("grab!");
        dbg!(self.imp().search_entry.borrow().grab_focus());
    }
}
