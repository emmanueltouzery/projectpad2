#[macro_use]
extern crate diesel;

pub mod models;
pub mod schema;

use diesel::prelude::*;
use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    // I've looked at $XDG_DATA_HOME: it's ~/.var/app/com.github.emmanueltouzery.projectpad/data
    // when running in flatpak, but ~/.local/share/ when running native
    // My problem is that I split my app in two in part due to flatpak. In reality, it's impossible to do
    // all that I want to do with this app within flatpak. So I made two apps: this one, which has a GUI
    // and doesn't need to run on the host, and a CLI statically linked companion app that must run
    // on the host. And they share the database. And I'd like to support the case of the flatpak app
    // being run on the metal too. That means that the CLI app must look in ~/.var/com.github... where
    // the flatpak app saves, as well as ~/. local/share. And if a user wants to contribute to the
    // app and run it on the host, I'd also like for the db to be found.
    //
    // https://github.com/flatpak/flatpak/wiki/Filesystem
    if std::env::var("XDG_DATA_HOME").is_err() {
        // assuming we're not in a flatpak
        // it's still possible the user installed the app in a
        // flatpak, and we are ppcli running on the host
        // => check if i find the flatpak app data folder
        let mut path = dirs::home_dir().expect("Failed to get the home folder");
        path.push(".var");
        path.push("app");
        path.push("com.github.emmanueltouzery.projectpad");
        path.push("data");
        if path.exists() {
            path.push("projectpad");
            return path;
        }
    }
    let mut path = dirs::data_local_dir().expect("Failed to get the data local folder");
    path.push("projectpad");
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
        // GUI app is built with sqlcipher3 and the CLI app with sqlcipher4, so I need these compatibility
        // parameters for the sqlcipher4 version to read the DB
        // I considered using the sqlcipher4 format, but many distributions ship only the sqlcipher3
        // command-line tools (the latest ubuntu, suse and fedora, as I write this), and these can be
        // handy for the user.
        .execute(&format!(
            "PRAGMA key='{}'; PRAGMA cipher_page_size = 1024; PRAGMA kdf_iter = 64000; PRAGMA cipher_hmac_algorithm = HMAC_SHA1; PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA1; SELECT count(*) FROM sqlite_master;",
            &key_escape_param_value(pass)
        ))
        .map(|_| ())
        .map_err(|x| x.to_string())
}

pub fn get_db_version(db_conn: &SqliteConnection) -> QueryResult<i32> {
    use schema::db_version::dsl as ver;
    ver::db_version
        .order(ver::code.desc())
        .select(ver::code)
        .first::<i32>(db_conn)
}

pub fn get_project_group_names(
    sql_conn: &diesel::SqliteConnection,
    project_id: i32,
) -> Vec<String> {
    use schema::project_point_of_interest::dsl as ppoi;
    use schema::server::dsl as srv;
    let server_group_names = srv::server
        .filter(
            srv::project_id
                .eq(project_id)
                .and(srv::group_name.is_not_null()),
        )
        .order(srv::group_name.asc())
        .select(srv::group_name)
        .load(sql_conn)
        .unwrap();
    let mut prj_poi_group_names = ppoi::project_point_of_interest
        .filter(
            ppoi::project_id
                .eq(project_id)
                .and(ppoi::group_name.is_not_null()),
        )
        .order(ppoi::group_name.asc())
        .select(ppoi::group_name)
        .load(sql_conn)
        .unwrap();
    let mut project_group_names = server_group_names;
    project_group_names.append(&mut prj_poi_group_names);
    let mut project_group_names_no_options: Vec<_> = project_group_names
        .into_iter()
        .map(|n: Option<String>| n.unwrap())
        .collect();
    project_group_names_no_options.sort();
    project_group_names_no_options.dedup();
    project_group_names_no_options
}

pub fn get_server_group_names(sql_conn: &diesel::SqliteConnection, server_id: i32) -> Vec<String> {
    use schema::server_database::dsl as db;
    use schema::server_extra_user_account::dsl as usr;
    use schema::server_note::dsl as not;
    use schema::server_point_of_interest::dsl as poi;
    use schema::server_website::dsl as www;
    let server_poi_group_names = poi::server_point_of_interest
        .filter(
            poi::server_id
                .eq(server_id)
                .and(poi::group_name.is_not_null()),
        )
        .order(poi::group_name.asc())
        .select(poi::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_www_group_names = www::server_website
        .filter(
            www::server_id
                .eq(server_id)
                .and(www::group_name.is_not_null()),
        )
        .order(www::group_name.asc())
        .select(www::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_db_group_names = db::server_database
        .filter(
            db::server_id
                .eq(server_id)
                .and(db::group_name.is_not_null()),
        )
        .order(db::group_name.asc())
        .select(db::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_usr_group_names = usr::server_extra_user_account
        .filter(
            usr::server_id
                .eq(server_id)
                .and(usr::group_name.is_not_null()),
        )
        .order(usr::group_name.asc())
        .select(usr::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_notes_group_names = not::server_note
        .filter(
            not::server_id
                .eq(server_id)
                .and(not::group_name.is_not_null()),
        )
        .order(not::group_name.asc())
        .select(not::group_name)
        .load(sql_conn)
        .unwrap();
    let mut server_group_names = server_poi_group_names;
    server_group_names.append(&mut server_www_group_names);
    server_group_names.append(&mut server_db_group_names);
    server_group_names.append(&mut server_usr_group_names);
    server_group_names.append(&mut server_notes_group_names);
    let mut server_group_names_no_options: Vec<_> = server_group_names
        .into_iter()
        .map(|n: Option<String>| n.unwrap())
        .collect();
    server_group_names_no_options.sort();
    server_group_names_no_options.dedup();
    server_group_names_no_options
}
