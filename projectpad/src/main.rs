#[macro_use]
extern crate diesel;

mod sql_thread;
mod widgets;

use relm::Widget;

fn main() {
    // TODO gui error if we fail connecting
    let sql_channel = sql_thread::start_sql_thread();

    widgets::win::Win::run(sql_channel).unwrap();
}
