use crate::database::{ItemOfInterest, ItemType};
use projectpadsql::models::{InterestType, RunOn, ServerAccessType};
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
        let username = &item.server_info.as_ref().unwrap().server_username;
        let user_param = if username.is_empty() {
            Cow::Borrowed("")
        } else {
            Cow::Owned(format!("{}@", username))
        };
        Some(match ssh_command_type {
            SshCommandType::Ssh => format!("ssh -p {} {}{}", port, user_param, addr),
            SshCommandType::Scp => format!("scp -P {} {}{}", port, user_param, addr),
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
        Some(srv) => srv.server_access_type == ServerAccessType::SrvAccessSsh,
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

fn get_value_cd_in_folder(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    Cow::Owned(format!(
        "cd {}",
        item.poi_info.as_ref().unwrap().path.to_str().unwrap()
    ))
}

fn get_value_text(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    Cow::Borrowed(&item.item_text)
}

fn get_value_ssh_run_on_ssh(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if item.run_on == Some(RunOn::RunOnServer) {
        if let Some(ssh_command) = try_prepare_ssh_command(item, SshCommandType::Ssh) {
            return Cow::Owned(format!("{} -t \"{}\"", ssh_command, &item.item_text));
        }
    }
    Cow::Borrowed(&item.item_text)
}

#[derive(PartialEq)]
pub enum AllowedAction {
    Run,
    CopyToClipboard,
    CopyToPrompt,
}

pub struct Action {
    pub item: ItemOfInterest,
    pub desc: &'static str,
    pub get_string: fn(&ItemOfInterest) -> Cow<str>,
    pub allowed_actions: Vec<AllowedAction>,
}

impl Action {
    fn new(
        desc: &'static str,
        get_string: fn(&ItemOfInterest) -> Cow<str>,
        item: ItemOfInterest,
    ) -> Action {
        Action {
            item,
            desc,
            get_string,
            allowed_actions: vec![
                AllowedAction::Run,
                AllowedAction::CopyToClipboard,
                AllowedAction::CopyToPrompt,
            ],
        }
    }
}

pub fn get_value(item: ItemOfInterest) -> Vec<Action> {
    match &item {
        i if i.item_type == ItemType::InterestItemType(InterestType::PoiLogFile)
            && is_ssh_access(i) =>
        {
            vec![
                Action::new("tail log", get_value_tail_file, item.clone()),
                Action::new("less log", get_value_less_file, item.clone()),
                Action::new("fetch log", get_value_fetch_file, item),
            ]
        }
        i if i.item_type == ItemType::InterestItemType(InterestType::PoiApplication)
            && is_ssh_access(i) =>
        {
            vec![Action::new("ssh folder", get_value_ssh_cd_in_folder, item)]
        }
        i if i.item_type == ItemType::InterestItemType(InterestType::PoiApplication)
            && i.server_info.is_none() =>
        {
            vec![Action {
                desc: "go folder",
                get_string: get_value_cd_in_folder,
                // cannot change the folder of the parent shell
                allowed_actions: vec![AllowedAction::CopyToClipboard, AllowedAction::CopyToPrompt],
                item,
            }]
        }
        i if i.sql_table.as_str() == "server" && is_ssh_access(i) => {
            vec![Action::new("ssh shell", get_value_server_ssh, item)]
        }
        i if [
            ItemType::InterestItemType(InterestType::PoiCommandToRun),
            ItemType::InterestItemType(InterestType::PoiCommandTerminal),
        ]
        .contains(&i.item_type)
            && !is_ssh_access(i) =>
        {
            vec![Action::new("run cmd", get_value_text, item)]
        }
        i if [
            ItemType::InterestItemType(InterestType::PoiCommandToRun),
            ItemType::InterestItemType(InterestType::PoiCommandTerminal),
        ]
        .contains(&i.item_type)
            && is_ssh_access(i) =>
        {
            vec![Action::new("run cmd", get_value_ssh_run_on_ssh, item)]
        }
        i if i.item_type == ItemType::InterestItemType(InterestType::PoiConfigFile)
            && is_ssh_access(i) =>
        {
            vec![
                Action::new("edit cfg", get_value_edit_file, item.clone()),
                Action::new("less cfg", get_value_less_file, item.clone()),
                Action::new("fetch cfg", get_value_fetch_file, item),
            ]
        }
        _ => Vec::new(),
    }
}
