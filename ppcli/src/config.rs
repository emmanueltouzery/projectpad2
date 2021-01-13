// bits lifted from the skim project
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

fn upgrade_check_time_path() -> PathBuf {
    let mut path = projectpadsql::config_path();
    path.push("upgrade-check-date");
    path
}

pub fn upgrade_check_mark_done() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = BufWriter::new(File::create(upgrade_check_time_path())?);
    file.write_all(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string()
            .as_bytes(),
    )?;
    Ok(())
}

pub fn upgrade_days_since_last_check() -> Result<u64, Box<dyn std::error::Error>> {
    let file_path = upgrade_check_time_path();
    if file_path.exists() {
        let file = File::open(file_path)?;
        let mut contents_str = String::new();
        BufReader::new(file).read_to_string(&mut contents_str)?;
        let trimmed = contents_str.trim();
        let previous_seconds = Duration::from_secs(trimmed.parse::<u64>()?);
        let previous_systime = SystemTime::UNIX_EPOCH + previous_seconds;
        Ok(SystemTime::now()
            .duration_since(previous_systime)?
            .as_secs()
            / 3600
            / 24)
    } else {
        Ok(365)
    }
}

fn history_file_path() -> PathBuf {
    let mut path = projectpadsql::config_path();
    path.push("cli-history");
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
    if orig_history.last().map(|l| l.as_str()) == Some(latest) {
        // no point of having at the end of the history 5x the same command...
        return Ok(());
    }
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
