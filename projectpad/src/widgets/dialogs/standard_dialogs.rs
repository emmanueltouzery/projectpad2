use gtk::prelude::*;
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
