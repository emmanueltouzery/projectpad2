use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::sync::mpsc;
use std::thread;

// https://stackoverflow.com/a/49122850/516188
pub struct SqlFunc(Box<dyn Fn(&SqliteConnection) + Send + 'static>);

impl SqlFunc {
    pub fn new<T>(func: T) -> SqlFunc
    where
        T: Fn(&SqliteConnection) + Send + 'static,
    {
        SqlFunc(Box::new(func))
    }
}

pub fn start_sql_thread() -> mpsc::Sender<SqlFunc> {
    let (tx, rx) = mpsc::channel::<SqlFunc>();

    thread::spawn(move || {
        let db_conn =
            SqliteConnection::establish(&projectpadsql::database_path().to_string_lossy()).unwrap();
        db_conn
            .execute(&format!(
                "PRAGMA key='{}'",
                projectpadsql::get_pass_from_keyring().unwrap()
            ))
            .unwrap();
        // TODO foreign key pragma, check what the original PP does
        rx.into_iter().for_each(|fun| (fun.0)(&db_conn));
    });

    tx
}
