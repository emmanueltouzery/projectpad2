#[macro_use]
extern crate diesel;

mod widgets;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use relm::Widget;

fn main() {
    // TODO gui error if we fail connecting
    let db_conn =
        SqliteConnection::establish(&projectpadsql::database_path().to_string_lossy()).unwrap();
    let pass = projectpadsql::get_pass_from_keyring().unwrap();
    widgets::win::Win::run(db_conn).unwrap();
}
