#[macro_use]
extern crate diesel;

pub mod models;
pub mod schema;

use diesel::prelude::*;
use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("Failed to get the home folder");
    path.push(".projectpad");
    path
}

pub fn database_path() -> PathBuf {
    let mut path = config_path();
    path.push("projectpad.db");
    path
}

// escape quote by doubling it
// https://github.com/rusqlite/rusqlite/blob/997e6d3cc37fa96f8edc3db9839c7e84246ee315/src/pragma.rs#L138
pub fn key_escape_param_value(key: &str) -> String {
    key.replace('\'', "''")
}

pub fn try_unlock_db(db_conn: &SqliteConnection, pass: &str) -> Result<(), String> {
    // https://www.zetetic.net/sqlcipher/sqlcipher-api/#PRAGMA_key
    db_conn
        // https://www.zetetic.net/blog/2018/11/30/sqlcipher-400-release/ on my machine at least, the
        // GUI app is built with sqlcipher3 and the CLI app with sqlcipher4, so I need these compatability
        // parameters for the sqlcipher4 version to read the DB
        .execute(&format!(
            "PRAGMA key='{}'; PRAGMA cipher_page_size = 1024; PRAGMA kdf_iter = 64000; PRAGMA cipher_hmac_algorithm = HMAC_SHA1; PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA1; SELECT count(*) FROM sqlite_master;",
            &key_escape_param_value(pass)
        ))
        .map(|_| ())
        .map_err(|x| x.to_string())
}
