use crate::database::ActionType;
use crate::database::{ItemOfInterest, ItemType, LinkedItemId};
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
        Some(match (ssh_command_type, port) {
            // don't pass in the -p/-P parameter if we're using the default port
            // I sometimes use alt-enter to edit a ssh command into a scp command
            // and the -p/-P difference gets in the way...
            (SshCommandType::Ssh, "22") => format!("ssh {}{}", user_param, addr),
            (SshCommandType::Scp, "22") => format!("scp {}{}", user_param, addr),
            (SshCommandType::Ssh, _) => format!("ssh -p {} {}{}", port, user_param, addr),
            (SshCommandType::Scp, _) => format!("scp -P {} {}{}", port, user_param, addr),
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

fn get_value_action_file<'a>(
    item: &'a ItemOfInterest,
    force_pseudo: ForcePseudoTTY,
    action: Cow<'static, str>,
) -> std::borrow::Cow<'a, str> {
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
    let local_editor = std::env::var("EDITOR");
    // first try $EDITOR of the remote, if not specified, fallback on the local editor,
    // and if not given, finally vim
    get_value_action_file(
        item,
        ForcePseudoTTY::Yes,
        Cow::Owned(format!(
            "\\${{EDITOR:-{}}}",
            local_editor.as_deref().unwrap_or("vim")
        )),
    )
}

fn get_value_tail_file(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    get_value_action_file(item, ForcePseudoTTY::No, Cow::Borrowed("tail -f"))
}

fn get_value_less_file(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    get_value_action_file(item, ForcePseudoTTY::Yes, Cow::Borrowed("less"))
}

fn get_value_fetch_file(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if let Some(scp_command) = try_prepare_ssh_command(item, SshCommandType::Scp) {
        let filename = item.poi_info.as_ref().unwrap().path.to_str().unwrap();
        let base_command = format!(
            "{}:{} {}",
            scp_command,
            filename,
            dirs::download_dir().unwrap().to_str().unwrap()
        );
        Cow::Owned(if filename.contains('`') {
            // support shell expansion with ` in filenames, so that you can for instance
            // have as a file name /opt/app/myapp/logs/myfile.`date "+%Y-%m-%d"`.log
            // -- dynamic date parameter in the filename.
            format!("sh -c \"{}\"", base_command)
        } else {
            base_command
        })
    } else {
        Cow::Borrowed(&item.item_text)
    }
}

// https://serverfault.com/a/738797/176574
fn get_value_ssh_cd_in_folder(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if let Some(ssh_command) = try_prepare_ssh_command(item, SshCommandType::Ssh) {
        // $SHELL is the shell of the machine i'm on now, may not be
        // installed on the remove server, so fallback on 'sh'
        Cow::Owned(format!(
            "{} -t \"cd {}; \\$SHELL --login || sh --login\"",
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
    pub desc: ActionType,
    pub get_string: fn(&ItemOfInterest) -> Cow<str>,
    pub allowed_actions: Vec<AllowedAction>,
}

impl Action {
    fn new(
        desc: ActionType,
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
                Action::new(ActionType::TailLog, get_value_tail_file, item.clone()),
                Action::new(ActionType::LessLog, get_value_less_file, item.clone()),
                Action::new(ActionType::FetchLog, get_value_fetch_file, item),
            ]
        }
        i if i.item_type == ItemType::InterestItemType(InterestType::PoiApplication)
            && is_ssh_access(i) =>
        {
            vec![Action::new(
                ActionType::SshFolder,
                get_value_ssh_cd_in_folder,
                item,
            )]
        }
        i if i.item_type == ItemType::InterestItemType(InterestType::PoiApplication)
            && i.server_info.is_none() =>
        {
            vec![Action {
                desc: ActionType::GoFolder,
                get_string: get_value_cd_in_folder,
                // cannot change the folder of the parent shell
                allowed_actions: vec![AllowedAction::CopyToClipboard, AllowedAction::CopyToPrompt],
                item,
            }]
        }
        i if matches!(i.linked_item, LinkedItemId::Server(_)) && is_ssh_access(i) => {
            vec![Action::new(
                ActionType::SshShell,
                get_value_server_ssh,
                item,
            )]
        }
        i if [
            ItemType::InterestItemType(InterestType::PoiCommandToRun),
            ItemType::InterestItemType(InterestType::PoiCommandTerminal),
        ]
        .contains(&i.item_type)
            && !is_ssh_access(i) =>
        {
            vec![Action::new(ActionType::RunCmd, get_value_text, item)]
        }
        i if [
            ItemType::InterestItemType(InterestType::PoiCommandToRun),
            ItemType::InterestItemType(InterestType::PoiCommandTerminal),
        ]
        .contains(&i.item_type)
            && is_ssh_access(i) =>
        {
            vec![Action::new(
                ActionType::RunCmd,
                get_value_ssh_run_on_ssh,
                item,
            )]
        }
        i if i.item_type == ItemType::InterestItemType(InterestType::PoiConfigFile)
            && is_ssh_access(i) =>
        {
            vec![
                Action::new(ActionType::EditCfg, get_value_edit_file, item.clone()),
                Action::new(ActionType::LessCfg, get_value_less_file, item.clone()),
                Action::new(ActionType::FetchCfg, get_value_fetch_file, item),
            ]
        }
        _ => Vec::new(),
    }
}
