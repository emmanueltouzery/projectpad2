use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::str;
use std::sync::mpsc;
use std::thread;

// https://github.com/tilpner/includedir
include!(concat!(env!("OUT_DIR"), "/data.rs"));

// we do sql requests in a separate thread not to block the GUI thread
// - i considered that spawning a new thread everytime the GUI wants to fetch
//   from SQL seems more heavyweight than reusing a thread
// - setting up the connection is messy, requires to fetch the password from
//   the OS secret storage -- so we'd like to set up the connection once
//   then reuse it.

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
        std::fs::create_dir_all(projectpadsql::config_path()).unwrap();
        let db_conn =
            SqliteConnection::establish(&projectpadsql::database_path().to_string_lossy()).unwrap();
        rx.into_iter().for_each(|fun| (fun.0)(&db_conn));
    });

    tx
}

pub fn migrate_db_if_needed(db_conn: &SqliteConnection) -> Result<(), Box<dyn std::error::Error>> {
    use projectpadsql::schema::db_version::dsl as ver;
    let mut version = projectpadsql::get_db_version(db_conn).unwrap_or(0) + 1;
    let get_migration_name = |version| format!("resources/migrations/{:03}.sql", version);
    loop {
        let migration_name = get_migration_name(version);
        if !MIGRATIONS.is_available(&migration_name) {
            break;
        }
        println!("applying migration {}", version);
        let migration_bytes = MIGRATIONS.get(&migration_name)?;
        let migration_str = str::from_utf8(&migration_bytes)?;
        db_conn.execute(migration_str)?;
        diesel::insert_into(ver::db_version)
            .values((
                ver::code.eq(version),
                ver::upgrade_date.eq(diesel::dsl::now),
            ))
            .execute(db_conn)?;
        version += 1;
    }
    Ok(())
}
