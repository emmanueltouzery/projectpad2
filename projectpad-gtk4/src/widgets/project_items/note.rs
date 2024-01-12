use gtk::prelude::*;

pub fn get_note_toolbar() -> gtk::Box {
    let toolbar = gtk::Box::builder().css_classes(["toolbar"]).build();

    toolbar.append(&gtk::Button::builder().icon_name("heading").build());
    toolbar.append(&gtk::Button::builder().icon_name("list-ul").build());
    toolbar.append(&gtk::Button::builder().icon_name("list-ol").build());
    toolbar.append(&gtk::Button::builder().icon_name("bold").build());
    toolbar.append(&gtk::Button::builder().icon_name("italic").build());
    toolbar.append(&gtk::Button::builder().icon_name("strikethrough").build());
    toolbar.append(&gtk::Button::builder().icon_name("link").build());
    toolbar.append(&gtk::Button::builder().icon_name("lock").build());
    toolbar.append(&gtk::Button::builder().icon_name("code").build());
    toolbar.append(&gtk::Button::builder().icon_name("quote").build());
    return toolbar;
}
