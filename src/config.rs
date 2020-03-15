// bits lifted from the skim project
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

fn config_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("Failed to get the home folder");
    path.push(".projectpad");
    path
}

fn history_file_path() -> PathBuf {
    let mut path = config_path();
    path.push("cli-history");
    path
}

pub fn database_path() -> PathBuf {
    let mut path = config_path();
    path.push("projectpad.db");
    path
}

pub fn read_history() -> Result<Vec<String>, std::io::Error> {
    let file = File::open(history_file_path())?;
    BufReader::new(file).lines().collect()
}

pub fn write_history(
    orig_history: &[String],
    latest: &str,
    limit: usize,
) -> Result<(), std::io::Error> {
    let additional_lines = if latest.trim().is_empty() { 0 } else { 1 };
    let start_index = if orig_history.len() + additional_lines > limit {
        orig_history.len() + additional_lines - limit
    } else {
        0
    };

    let mut history = orig_history[start_index..].to_vec();
    history.push(latest.to_string());

    let file = File::create(history_file_path())?;
    let mut file = BufWriter::new(file);
    file.write_all(history.join("\n").as_bytes())?;
    Ok(())
}
