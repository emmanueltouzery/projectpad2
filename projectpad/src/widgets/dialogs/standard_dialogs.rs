use gtk::prelude::*;
use std::error::Error;

pub fn get_main_window(widget_for_window: gtk::Widget) -> gtk::Window {
    widget_for_window
        .get_toplevel()
        .and_then(|w| w.dynamic_cast::<gtk::Window>().ok())
        .unwrap()
}

pub fn display_error(msg: &str, e: Option<Box<dyn Error>>) {
    display_error_str(msg, e.map(|e| e.to_string()))
}

pub fn display_error_str(msg: &str, e: Option<String>) {
    let builder = gtk::MessageDialogBuilder::new()
        .buttons(gtk::ButtonsType::Ok)
        .message_type(gtk::MessageType::Error)
        .modal(true)
        .text(msg);
    let dlg = if let Some(err) = e {
        builder.secondary_text(&err)
    } else {
        builder
    }
    .build();
    dlg.connect_response(|d, _r| d.close());
    dlg.show_all();
}

pub fn confirm_deletion(
    summary: &str,
    msg: &str,
    widget: gtk::Widget,
    confirm_cb: impl Fn() + 'static,
) {
    let main_win = get_main_window(widget);
    let dialog = gtk::MessageDialogBuilder::new()
        .title("Confirmation")
        .text(summary)
        .secondary_text(msg)
        .message_type(gtk::MessageType::Warning)
        .transient_for(&main_win)
        .modal(true)
        .build();
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    let save = dialog.add_button("Delete", gtk::ResponseType::Ok);
    save.get_style_context().add_class("destructive-action");
    dialog.connect_response(move |d, r| {
        d.close();
        if r == gtk::ResponseType::Ok {
            confirm_cb();
        }
    });
    dialog.show_all();
}
