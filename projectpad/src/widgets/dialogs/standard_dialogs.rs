use gtk::prelude::*;
use relm::Widget;
use std::error::Error;

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
    dialog.show();
}

pub fn get_main_window(widget_for_window: gtk::Widget) -> gtk::Window {
    widget_for_window
        .get_toplevel()
        .and_then(|w| w.dynamic_cast::<gtk::Window>().ok())
        .unwrap()
}

#[derive(PartialEq, Eq)]
pub enum DialogActionResult {
    CloseDialog,
    DontCloseDialog,
}

pub fn prepare_custom_dialog<T: Widget>(
    widget_for_window: gtk::Widget,
    width: i32,
    height: i32,
    title: String,
    dialog_contents: relm::Component<T>,
    ok_callback: impl Fn(gtk::Button) -> DialogActionResult + 'static,
) -> (gtk::Dialog, relm::Component<T>, gtk::Button) {
    let (dialog, save) = prepare_custom_dialog_component_ref(
        widget_for_window,
        width,
        height,
        title,
        &dialog_contents,
        ok_callback,
    );
    (dialog, dialog_contents, save.clone())
}

pub fn prepare_custom_dialog_component_ref<T: Widget>(
    widget_for_window: gtk::Widget,
    width: i32,
    height: i32,
    title: String,
    dialog_contents: &relm::Component<T>,
    ok_callback: impl Fn(gtk::Button) -> DialogActionResult + 'static,
) -> (gtk::Dialog, gtk::Button) {
    let main_win = get_main_window(widget_for_window);
    let dialog = gtk::DialogBuilder::new()
        .use_header_bar(1)
        .default_width(width)
        .default_height(height)
        .title(&title)
        .transient_for(&main_win)
        .modal(true)
        .build();

    dialog_contents.widget().show();
    dialog
        .get_content_area()
        .pack_start(dialog_contents.widget(), true, true, 0);
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    let save = dialog
        .add_button("Save", gtk::ResponseType::Ok)
        .downcast::<gtk::Button>()
        .expect("error reading the dialog save button");
    save.get_style_context().add_class("suggested-action");
    let save_btn = save.clone();
    dialog.connect_response(move |d, r| {
        if r == gtk::ResponseType::Ok {
            if ok_callback(save_btn.clone()) == DialogActionResult::CloseDialog {
                d.close();
            }
        } else {
            d.close();
        }
    });
    (dialog, save.clone())
}
