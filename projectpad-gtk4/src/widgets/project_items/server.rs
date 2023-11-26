use adw::prelude::*;

pub fn display_server(parent: &adw::Bin, id: i32, edit_mode: bool) {
    if edit_mode {
        display_server_edit(parent);
    } else {
        display_server_show(parent);
    }
}

fn display_server_edit(parent: &adw::Bin) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(10)
        .margin_top(10)
        .build();

    let header_box = gtk::Box::builder().spacing(10).build();

    let server_icon = gtk::Image::builder()
        .icon_name("server")
        .pixel_size(48)
        .build();
    header_box.append(&server_icon);

    let header_second_col = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .valign(gtk::Align::Center)
        .build();

    let server = gtk::Entry::builder()
        .text("Server")
        .halign(gtk::Align::Start)
        .css_classes(["title-1"])
        // .description("desc")
        .build();
    header_second_col.append(&server);

    header_box.append(&header_second_col);

    let delete_btn = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .halign(gtk::Align::End)
        .hexpand(true)
        .build();
    header_box.append(&delete_btn);

    vbox.append(&header_box);

    // let server_ar = adw::EntryRow::builder().title("Server name").build();
    // server_ar.add_suffix(
    //     &gtk::Button::builder()
    //         .icon_name("open-menu-symbolic")
    //         .has_frame(false)
    //         .valign(gtk::Align::Center)
    //         .build(),
    // );
    // server.add(&server_ar);

    let server_item0 = adw::PreferencesGroup::builder().build();

    let address_ar = adw::EntryRow::builder()
        .title("Address")
        .text("hostname")
        .build();
    server_item0.add(&address_ar);
    // server.add(&address_ar);

    let server_username_ar = adw::EntryRow::builder()
        .title("Username")
        .text("root")
        .build();
    // server.add(&server_username_ar);
    server_item0.add(&server_username_ar);

    vbox.append(&server_item0);

    let server_item1 = adw::PreferencesGroup::builder()
        .title("Website")
        .description("service1")
        .build();
    let website_ar = adw::EntryRow::builder()
        .title("Address")
        .text("https://service1.com")
        .build();
    server_item1.add(&website_ar);

    let username_ar = adw::EntryRow::builder()
        .title("Username")
        .text("admin")
        .build();
    server_item1.add(&username_ar);
    let password_ar = adw::PasswordEntryRow::builder()
        .title("Password")
        .text("pass")
        .build();
    server_item1.add(&password_ar);
    vbox.append(&server_item1);

    // lb.set_property("halign", gtk::Align::Fill);
    // parent.set_property("halign", gtk::Align::Fill);

    let add_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .hexpand(true)
        .build();
    vbox.append(&add_btn);

    parent.set_child(Some(&vbox));
}

fn display_server_show(parent: &adw::Bin) {
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(10)
        .margin_top(10)
        .build();

    let header_box = gtk::Box::builder().spacing(10).build();

    let server_icon = gtk::Image::builder()
        .icon_name("server")
        .pixel_size(48)
        .build();
    header_box.append(&server_icon);

    let header_second_col = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .valign(gtk::Align::Center)
        .build();

    let server = gtk::Label::builder()
        .label("Server")
        .halign(gtk::Align::Start)
        .css_classes(["title-1"])
        // .description("desc")
        .build();
    header_second_col.append(&server);

    header_box.append(&header_second_col);

    vbox.append(&header_box);

    // let server_ar = adw::ActionRow::builder().title("Server name").build();
    // server_ar.add_suffix(
    //     &gtk::Button::builder()
    //         .icon_name("open-menu-symbolic")
    //         .has_frame(false)
    //         .valign(gtk::Align::Center)
    //         .build(),
    // );
    // server.add(&server_ar);

    let server_item0 = adw::PreferencesGroup::builder().build();

    let address_ar = adw::ActionRow::builder()
        .title("Address")
        .subtitle("hostname")
        .build();
    address_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("edit-copy-symbolic")
            .build(),
    );
    server_item0.add(&address_ar);
    // server.add(&address_ar);

    let server_username_ar = adw::ActionRow::builder()
        .title("Username")
        .subtitle("root")
        .build();
    server_username_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("edit-copy-symbolic")
            .build(),
    );
    // server.add(&server_username_ar);
    server_item0.add(&server_username_ar);

    vbox.append(&server_item0);

    let server_item1 = adw::PreferencesGroup::builder()
        .title("Website")
        .description("service1")
        .build();
    let website_ar = adw::ActionRow::builder()
        .title("Address")
        .subtitle("https://service1.com")
        .build();
    website_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("web-browser-symbolic")
            .build(),
    );
    server_item1.add(&website_ar);

    let username_ar = adw::ActionRow::builder()
        .title("Username")
        .subtitle("admin")
        .build();
    username_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("edit-copy-symbolic")
            .build(),
    );
    server_item1.add(&username_ar);
    let password_ar = adw::ActionRow::builder()
        .title("Password")
        .subtitle("●●●●")
        .build();
    password_ar.add_suffix(
        &gtk::Image::builder()
            .icon_name("edit-copy-symbolic")
            .build(),
    );
    server_item1.add(&password_ar);
    vbox.append(&server_item1);

    // lb.set_property("halign", gtk::Align::Fill);
    // parent.set_property("halign", gtk::Align::Fill);

    parent.set_child(Some(&vbox));
}
