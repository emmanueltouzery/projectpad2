// bits lifted from the skim project
use crate::database::ActionType;
use crate::database::{ExecutedAction, LinkedItem};
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, ErrorKind};
use std::path::PathBuf;
use std::str::FromStr;
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

fn actions_file_path() -> PathBuf {
    let mut path = projectpadsql::config_path();
    path.push("action-history");
    path
}

pub fn read_action_history() -> Result<Vec<ExecutedAction>, std::io::Error> {
    let actions_file = File::open(actions_file_path())?;
    BufReader::new(actions_file)
        .lines()
        .map(|line| parse_action_history_line(&line?))
        .collect()
}

pub fn read_string_history() -> Result<Vec<String>, std::io::Error> {
    let hist_file = File::open(history_file_path())?;
    BufReader::new(hist_file).lines().collect()
}

fn parse_action_history_line(line: &str) -> Result<ExecutedAction, std::io::Error> {
    let elts: Vec<_> = line.split(';').collect();
    match (
        elts.len(),
        elts.get(0),
        elts.get(1).and_then(|i| i.parse::<i32>().ok()),
        elts.get(2).and_then(|a| ActionType::from_str(a).ok()),
    ) {
        (3, Some(&"S"), Some(id), Some(action_desc)) => {
            Ok(ExecutedAction::new(LinkedItem::ServerId(id), action_desc))
        }
        (3, Some(&"P"), Some(id), Some(action_desc)) => Ok(ExecutedAction::new(
            LinkedItem::ProjectPoiId(id),
            action_desc,
        )),
        (3, Some(&"SP"), Some(id), Some(action_desc)) => Ok(ExecutedAction::new(
            LinkedItem::ServerPoiId(id),
            action_desc,
        )),
        _ => Err(std::io::Error::new(
            ErrorKind::Other,
            format!("couldn't parse {}", line),
        )),
    }
}

fn serialize_action_history_line(action: &ExecutedAction) -> String {
    match action {
        ExecutedAction {
            item: LinkedItem::ServerId(id),
            action_desc,
        } => format!("S;{};{}", id, action_desc),
        ExecutedAction {
            item: LinkedItem::ProjectPoiId(id),
            action_desc,
        } => format!("P;{};{}", id, action_desc),
        ExecutedAction {
            item: LinkedItem::ServerPoiId(id),
            action_desc,
        } => format!("SP;{};{}", id, action_desc),
    }
}

pub fn write_actions_history(
    orig_actions: &[ExecutedAction],
    latest: ExecutedAction,
    limit: usize,
) -> Result<(), std::io::Error> {
    write_history(
        &actions_file_path(),
        &orig_actions
            .iter()
            .map(serialize_action_history_line)
            .collect::<Vec<_>>(),
        &serialize_action_history_line(&latest),
        limit,
    )
}

pub fn write_string_history(
    orig_history: &[String],
    latest: &str,
    limit: usize,
) -> Result<(), std::io::Error> {
    if orig_history.last().map(|l| l.as_str()) == Some(latest) {
        // no point of having at the end of the history 5x the same command...
        return Ok(());
    }
    write_history(&history_file_path(), orig_history, latest, limit)
}

fn write_history(
    pathbuf: &PathBuf,
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

    let file = File::create(pathbuf)?;
    let mut file = BufWriter::new(file);
    file.write_all(history.join("\n").as_bytes())?;
    Ok(())
}
