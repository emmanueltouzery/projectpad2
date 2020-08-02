#[macro_use]
extern crate diesel;

pub mod icons;
mod sql_thread;
mod widgets;

use relm::Widget;

fn main() {
    // TODO gui error if we fail connecting
    let sql_channel = sql_thread::start_sql_thread();

    let res_bytes = include_bytes!("icons.bin");
    let data = glib::Bytes::from(&res_bytes[..]);
    let resource = gio::Resource::from_data(&data).unwrap();
    gio::resources_register(&resource);

    widgets::win::Win::run(sql_channel).unwrap();
}
