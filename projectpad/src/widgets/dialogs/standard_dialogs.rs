use gtk::prelude::*;
use relm::Widget;
use std::error::Error;

pub fn display_error(msg: &str, e: Option<Box<dyn Error>>) {
    display_error_str(msg, e.map(|e| e.to_string()))
}

pub fn display_error_str(msg: &str, e: Option<String>) {
    let builder = gtk::builders::MessageDialogBuilder::new()
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
    let dialog = gtk::builders::MessageDialogBuilder::new()
        .title("Confirmation")
        .text(summary)
        .secondary_text(msg)
        .message_type(gtk::MessageType::Warning)
        .transient_for(&main_win)
        .modal(true)
        .build();
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    let save = dialog.add_button("Delete", gtk::ResponseType::Ok);
    save.style_context().add_class("destructive-action");
    dialog.connect_response(move |d, r| {
        d.close();
        if r == gtk::ResponseType::Ok {
            confirm_cb();
        }
    });
    dialog.show();
}

pub fn get_main_window(widget_for_window: gtk::Widget) -> gtk::Window {
    widget_for_window
        .toplevel()
        .and_then(|w| w.dynamic_cast::<gtk::Window>().ok())
        .unwrap()
}

pub fn prepare_custom_dialog<T: Widget>(
    dialog: gtk::Dialog,
    dialog_contents: relm::Component<T>,
    ok_callback: impl Fn(gtk::Button) + 'static,
) -> (gtk::Dialog, relm::Component<T>, gtk::Button) {
    let save = dialog
        .add_button("Save", gtk::ResponseType::Ok)
        .downcast::<gtk::Button>()
        .expect("error reading the dialog save button");
    save.set_has_default(true);
    save.style_context().add_class("suggested-action");
    prepare_custom_dialog_component_ref(&dialog, &dialog_contents);
    let save_btn = save.clone();
    dialog.connect_response(move |d, r| {
        if r == gtk::ResponseType::Ok {
            ok_callback(save_btn.clone());
        } else {
            d.close();
        }
    });
    (dialog, dialog_contents, save)
}

pub fn modal_dialog(
    widget_for_window: gtk::Widget,
    width: i32,
    height: i32,
    title: String,
) -> gtk::Dialog {
    let main_win = get_main_window(widget_for_window);
    gtk::builders::DialogBuilder::new()
        .use_header_bar(1)
        .default_width(width)
        .default_height(height)
        .title(&title)
        .transient_for(&main_win)
        .modal(true)
        .build()
}

pub fn prepare_custom_dialog_component_ref<T: Widget>(
    dialog: &gtk::Dialog,
    dialog_contents: &relm::Component<T>,
) {
    dialog_contents.widget().show();
    dialog
        .content_area()
        .pack_start(dialog_contents.widget(), true, true, 0);
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
}
