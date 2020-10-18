#[macro_use]
extern crate diesel;

pub mod config;
pub mod icons;
pub mod notes;
mod sql_thread;
mod widgets;

use relm::Widget;
use std::panic;
use std::process;

fn main() {
    // https://stackoverflow.com/a/36031130/516188
    // close the app if we panic in the sql thread
    // instead of having that thread silently terminated
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        process::exit(1);
    }));

    let db_preexisted = projectpadsql::database_path().is_file();

    // TODO gui error if we fail connecting
    let sql_channel = sql_thread::start_sql_thread();

    let res_bytes = include_bytes!("icons.bin");
    let data = glib::Bytes::from(&res_bytes[..]);
    let resource = gio::Resource::from_data(&data).unwrap();
    gio::resources_register(&resource);

    widgets::win::Win::run((sql_channel, !db_preexisted)).unwrap();
}
