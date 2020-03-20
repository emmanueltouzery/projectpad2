use crate::database::{ItemOfInterest, ItemType, SrvAccessType};
use std::borrow::Cow;

enum SshCommandType {
    Ssh,
    Scp,
}

fn try_prepare_ssh_command(
    item: &ItemOfInterest,
    ssh_command_type: SshCommandType,
) -> Option<String> {
    // TODO must be a cleaner way to express this...
    if let Some([addr, port]) = match item
        .server_info
        .as_ref()
        .unwrap()
        .server_ip
        .split(':')
        .collect::<Vec<&str>>()[..]
    {
        [addr, port] => Some([addr, port]),
        [addr] => Some([addr, "22"]),
        _ => None,
    } {
        Some(match ssh_command_type {
            SshCommandType::Ssh => format!(
                "ssh -p {} {}@{}",
                port,
                item.server_info.as_ref().unwrap().server_username,
                addr
            ),
            SshCommandType::Scp => format!(
                "scp -P {} {}@{}",
                port,
                item.server_info.as_ref().unwrap().server_username,
                addr
            ),
        })
    } else {
        None
    }
}

fn get_value_server_ssh(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if let Some(ssh_command) = try_prepare_ssh_command(item, SshCommandType::Ssh) {
        Cow::Owned(ssh_command)
    } else {
        Cow::Borrowed(&item.item_text)
    }
}

fn is_ssh_access(item: &ItemOfInterest) -> bool {
    match &item.server_info {
        Some(srv) => srv.server_access_type == SrvAccessType::SrvAccessSsh,
        None => false,
    }
}

#[derive(PartialEq)]
enum ForcePseudoTTY {
    Yes,
    No,
}

fn get_value_action_file(
    item: &ItemOfInterest,
    force_pseudo: ForcePseudoTTY,
    action: String,
) -> std::borrow::Cow<str> {
    if let Some(ssh_command) = try_prepare_ssh_command(item, SshCommandType::Ssh) {
        Cow::Owned(format!(
            "{} {}\"{} {}\"",
            ssh_command,
            if force_pseudo == ForcePseudoTTY::Yes {
                "-t "
            } else {
                ""
            },
            action,
            item.poi_info.as_ref().unwrap().path.to_str().unwrap()
        ))
    } else {
        Cow::Borrowed(&item.item_text)
    }
}

fn get_value_edit_file(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    get_value_action_file(item, ForcePseudoTTY::Yes, "vim".to_string())
}

fn get_value_tail_file(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    get_value_action_file(item, ForcePseudoTTY::No, "tail -f".to_string())
}

fn get_value_less_file(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    get_value_action_file(item, ForcePseudoTTY::Yes, "less".to_string())
}

fn get_value_fetch_file(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if let Some(scp_command) = try_prepare_ssh_command(item, SshCommandType::Scp) {
        Cow::Owned(format!(
            "{}:{} {}",
            scp_command,
            item.poi_info.as_ref().unwrap().path.to_str().unwrap(),
            dirs::download_dir().unwrap().to_str().unwrap()
        ))
    } else {
        Cow::Borrowed(&item.item_text)
    }
}

// https://serverfault.com/a/738797/176574
fn get_value_ssh_cd_in_folder(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if let Some(ssh_command) = try_prepare_ssh_command(item, SshCommandType::Ssh) {
        Cow::Owned(format!(
            "{} -t \"cd {}; exec \\$SHELL --login\"",
            ssh_command,
            item.poi_info.as_ref().unwrap().path.to_str().unwrap()
        ))
    } else {
        Cow::Borrowed(&item.item_text)
    }
}

fn get_value_text(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    Cow::Borrowed(&item.item_text)
}

fn get_value_ssh_run_on_ssh(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if let Some(ssh_command) = try_prepare_ssh_command(item, SshCommandType::Ssh) {
        Cow::Owned(format!("{} -t \"{}\"", ssh_command, &item.item_text))
    } else {
        Cow::Borrowed(&item.item_text)
    }
}

pub struct Action {
    pub desc: String,
    pub get_string: Box<dyn Fn(&ItemOfInterest) -> Cow<str>>,
}

impl Action {
    fn new(desc: String, get_string: Box<dyn Fn(&ItemOfInterest) -> Cow<str>>) -> Action {
        Action { desc, get_string }
    }
}

pub fn get_value(item: &ItemOfInterest) -> Vec<Action> {
    match item {
        i if i.item_type == ItemType::PoiLogFile && is_ssh_access(i) => vec![
            Action::new("tail log file".to_string(), Box::new(get_value_tail_file)),
            Action::new("less log file".to_string(), Box::new(get_value_less_file)),
            Action::new("fetch log file".to_string(), Box::new(get_value_fetch_file)),
        ],
        i if i.item_type == ItemType::PoiApplication && is_ssh_access(i) => vec![Action::new(
            "ssh in that folder".to_string(),
            Box::new(get_value_ssh_cd_in_folder),
        )],
        i if i.sql_table.as_str() == "server" && is_ssh_access(i) => vec![Action::new(
            "ssh on the server".to_string(),
            Box::new(get_value_server_ssh),
        )],
        i if [ItemType::PoiCommandToRun, ItemType::PoiCommandTerminal].contains(&i.item_type)
            && !is_ssh_access(i) =>
        {
            vec![Action::new(
                "run command".to_string(),
                Box::new(get_value_text),
            )]
        }
        i if [ItemType::PoiCommandToRun, ItemType::PoiCommandTerminal].contains(&i.item_type)
            && is_ssh_access(i) =>
        {
            vec![Action::new(
                "run command on server".to_string(),
                Box::new(get_value_ssh_run_on_ssh),
            )]
        }
        i if i.item_type == ItemType::PoiConfigFile && is_ssh_access(i) => vec![
            Action::new(
                "edit config file".to_string(),
                Box::new(get_value_edit_file),
            ),
            Action::new(
                "less config file".to_string(),
                Box::new(get_value_less_file),
            ),
            Action::new(
                "fetch config file".to_string(),
                Box::new(get_value_fetch_file),
            ),
        ],
        _ => Vec::new(),
    }
}
