#[macro_use]
extern crate diesel;

pub mod models;
pub mod schema;

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

pub fn get_pass_from_keyring() -> Option<String> {
    let service = "projectpad-cli";
    let kr = keyring::Keyring::new(&service, &service);
    // kr.set_password("mc");
    kr.get_password().ok()
}
