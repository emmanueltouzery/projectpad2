use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Builder, Button, MessageDialog, ResponseType};
use widgets::project_list::ProjectList;
mod widgets;

#[derive(Default)]
pub struct Project {
    name: String,
}

#[derive(Default)]
pub struct ProjectItem {
    name: String,
}

fn main() -> glib::ExitCode {
    let res_bytes = include_bytes!("resources.bin");
    let data = glib::Bytes::from(&res_bytes[..]);
    let resource = gio::Resource::from_data(&data).unwrap();
    gio::resources_register(&resource);

    let application = gtk::Application::new(
        Some("com.github.gtk-rs.examples.builder_basics"),
        Default::default(),
    );
    application.connect_activate(build_ui);
    application.run()
}

fn build_ui(application: &Application) {
    // https://github.com/gtk-rs/gtk4-rs/issues/116
    // must call before using in UI files
    widgets::project_item_row::ProjectItemRow::static_type();
    ProjectList::static_type();

    let ui_src = include_str!("gtk_builder.ui");
    let builder = Builder::from_string(ui_src);

    let window: ApplicationWindow = builder.object("window").expect("Couldn't get window");
    window.set_application(Some(application));
    // let bigbutton: Button = builder.object("button").expect("Couldn't get button");
    let dialog: MessageDialog = builder
        .object("messagedialog")
        .expect("Couldn't get messagedialog");

    dialog.connect_response(move |d: &MessageDialog, _: ResponseType| {
        d.hide();
    });

    // bigbutton.connect_clicked(move |_| {
    //     dialog.show();
    // });

    window.show();
}
