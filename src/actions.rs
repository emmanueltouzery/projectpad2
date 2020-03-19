use crate::database::{ItemOfInterest, ItemType, SrvAccessType};
use std::borrow::Cow;

fn try_prepare_ssh_command(item: &ItemOfInterest) -> Option<String> {
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
        Some(format!(
            "ssh -p {} {}@{}",
            port,
            item.server_info.as_ref().unwrap().server_username,
            addr
        ))
    } else {
        None
    }
}

fn get_value_server_ssh(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if let Some(ssh_command) = try_prepare_ssh_command(item) {
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

fn get_value_ssh_log_file(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if let Some(ssh_command) = try_prepare_ssh_command(item) {
        Cow::Owned(format!(
            "{} \"{}{}\"",
            ssh_command,
            "tail -f ",
            item.poi_info.as_ref().unwrap().path.to_str().unwrap()
        ))
    } else {
        Cow::Borrowed(&item.item_text)
    }
}

pub fn get_value(item: &ItemOfInterest) -> Cow<str> {
    match item {
        i if i.item_type == ItemType::PoiLogFile && is_ssh_access(i) => {
            get_value_ssh_log_file(item)
        }
        i if i.sql_table.as_str() == "server" && is_ssh_access(i) => get_value_server_ssh(item),
        _ => Cow::Borrowed(&item.item_text),
    }
}
