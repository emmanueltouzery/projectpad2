use adw::prelude::*;

pub fn display_server(parent: &adw::Bin, id: i32) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(10)
        .margin_top(10)
        .build();

    // https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/boxed-lists.html
    let lb = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(vec!["boxed-list"])
        .build();

    lb.append(&adw::ActionRow::builder().title("Server name").build());
    lb.append(
        &adw::ActionRow::builder()
            .title("Address")
            .subtitle("hostname")
            .build(),
    );
    lb.append(
        &adw::ActionRow::builder()
            .title("Username")
            .subtitle("root")
            .build(),
    );
    vbox.append(&lb);

    let server_item1 = adw::PreferencesGroup::builder()
        .title("Website")
        .description("service1")
        .build();
    server_item1.add(
        &adw::ActionRow::builder()
            .title("Address")
            .subtitle("https://service1.com")
            .build(),
    );
    server_item1.add(
        &adw::ActionRow::builder()
            .title("Username")
            .subtitle("admin")
            .build(),
    );
    server_item1.add(
        &adw::ActionRow::builder()
            .title("Password")
            .subtitle("●●●●")
            .build(),
    );
    vbox.append(&server_item1);

    // lb.set_property("halign", gtk::Align::Fill);
    // parent.set_property("halign", gtk::Align::Fill);

    parent.set_child(Some(&vbox));
}
