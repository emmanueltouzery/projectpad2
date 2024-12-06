use adw::prelude::*;

mod imp {
    use gtk::subclass::{
        prelude::{ObjectImpl, ObjectSubclass},
        widget::WidgetImpl,
    };

    #[derive(Debug, Default)]
    pub struct SearchBar {}

    #[glib::object_subclass]
    impl ObjectSubclass for SearchBar {
        const NAME: &'static str = "SearchBar";
        type ParentType = adw::Bin;
        type Type = super::SearchBar;
    }

    impl ObjectImpl for SearchBar {}

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

        // let revealer = gtk::Revealer::builder().child(&hbox).build();

        // this.set_child(Some(&revealer));
        this.set_child(Some(&hbox));
        this.set_halign(gtk::Align::End);
        this.set_valign(gtk::Align::Start);
        this.set_vexpand(false);
        this
    }
}
