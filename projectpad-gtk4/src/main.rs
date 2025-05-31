use std::{panic, process};

use app::ProjectpadApplication;
use gtk::glib;
mod widgets;

mod app;
mod keyring_helpers;
pub mod notes;
mod search_engine;
mod sql_thread;
pub mod string_sidecar_object;
#[macro_use]
pub mod sql_util;
mod export;
mod import;
mod import_export_dtos;
mod import_export_ui;
mod unlock_db_dialog;
mod win;

fn main() -> glib::ExitCode {
    let res_bytes = include_bytes!("resources.bin");
    let data = glib::Bytes::from(&res_bytes[..]);
    let resource = gio::Resource::from_data(&data).unwrap();
    gio::resources_register(&resource);

    // https://stackoverflow.com/a/36031130/516188
    // close the app if we panic in the sql thread
    // instead of having that thread silently terminated
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        process::exit(1);
    }));

    let db_path = projectpadsql::database_path();

    // See https://github.com/emmanueltouzery/projectpad2/issues/1
    // if you start the app, and close the login screen without
    // unlocking the DB, we leave a DB file of zero bytes, and at
    // next startup we ask you for the unlock password, we don't
    // anymore ask you for a confirm password, because we assume
    // there's already a DB around => check that the db file is
    // present AND not empty.
    // if reading the file length fails, assume a non-empty file.
    let db_preexisted = db_path.is_file()
        && std::fs::metadata(db_path)
            .map(|m| m.len())
            .unwrap_or_else(|e| {
                eprintln!("Failed reading file metadata? {:?}", e);
                1
            })
            > 0;

    let sql_channel = sql_thread::start_sql_thread();

    ProjectpadApplication::run(sql_channel, !db_preexisted)
}
