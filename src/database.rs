use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{params, Connection};
use skim::prelude::*;
use std::borrow::Cow;
use std::path::PathBuf;
use std::str::FromStr;
use strum_macros::{Display, EnumString};

#[derive(Debug, EnumString, Display, PartialEq)]
pub enum SrvAccessType {
    SrvAccessSsh,
    SrvAccessRdp,
    SrvAccessWww,
    SrvAccessSshTunnel,
}

#[derive(Debug, EnumString, Display, PartialEq)]
pub enum ItemType {
    PoiApplication,
    PoiLogFile,
    PoiConfigFile,
    PoiCommandToRun,
    PoiCommandTerminal,
    PoiBackupArchive,
    SrvDatabase,
    SrvApplication,
    SrvHttpOrProxy,
    SrvMonitoring,
    SrvReporting,
}

// i want to convert string->enum
// https://www.reddit.com/r/rust/comments/7vxmmy/macro_for_generating_string_enum_parser/
// serde could be an option but it seems overkill...
macro_rules! from_sql_from_str(
    ($t:ident) => (
        impl FromSql for $t {
            fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        value.as_str().and_then(|s| {
            $t::from_str(s).map_err(|strum_err| FromSqlError::Other(Box::new(strum_err)))
        })
            }
        }
    )
);

from_sql_from_str!(ItemType);
from_sql_from_str!(SrvAccessType);

#[derive(Debug)]
pub struct ServerInfo {
    server_desc: String,
    server_username: String,
    server_ip: String,
    server_access_type: SrvAccessType,
}

#[derive(Debug)]
pub struct PoiInfo {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ItemOfInterest {
    id: i32,
    sql_table: String,
    project_name: String,
    env: Option<String>,
    item_type: ItemType,
    poi_desc: Option<String>,
    item_text: String,
    pub server_info: Option<ServerInfo>,
    pub poi_info: Option<PoiInfo>,
}

const SERVER_POIS_QUERY: &str = r#"SELECT server_point_of_interest.id, 'server_point_of_interest',
                     project.name, server.desc, server_point_of_interest.desc,
                     server.environment, server_point_of_interest.text, server_point_of_interest.interest_type,
                     server.username, server_point_of_interest.path, server.access_type, server.ip
                 from server_point_of_interest
                 join server on server.id = server_point_of_interest.server_id
                 join project on project.id = server.project_id"#;

const PROJECT_POIS_QUERY: &str = r#"SELECT project_point_of_interest.id, 'project_point_of_interest',
                     project.name, NULL, project_point_of_interest.desc,
                     NULL, project_point_of_interest.text, project_point_of_interest.interest_type,
                     NULL, project_point_of_interest.path, NULL, NULL
                 from project_point_of_interest
                 join project on project.id = project_point_of_interest.project_id"#;

const SERVERS_QUERY: &str = r#"SELECT server.id, 'server', project.name, server.desc, NULL,
                     server.environment, server.ip, server.type,
                     server.username, NULL, server.access_type, server.ip
                 from server
                 join project on project.id = server.project_id"#;

pub fn load_items(db_pass: &str, item_sender: &Sender<Arc<dyn SkimItem>>) {
    let conn = Connection::open(crate::config::database_path()).unwrap(); // TODO react better if no DB
    conn.pragma_update(None, "key", &db_pass).unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "{} UNION ALL {} UNION ALL {} order by project.name",
            SERVER_POIS_QUERY, PROJECT_POIS_QUERY, SERVERS_QUERY
        ))
        .unwrap();
    let server_poi_iter = stmt
        .query_map(params![], |row| {
            let server_desc: Option<String> = row.get(3).unwrap();
            let server_username: Option<String> = row.get(8).unwrap();
            let server_access_type: Option<SrvAccessType> = row.get(10).unwrap();
            let server_ip: Option<String> = row.get(11).unwrap();
            let server_info = match (server_desc, server_username, server_access_type, server_ip) {
                (Some(d), Some(u), Some(a), Some(i)) => Some(ServerInfo {
                    server_desc: d,
                    server_username: u,
                    server_ip: i,
                    server_access_type: a,
                }),
                _ => None,
            };
            let path: Option<String> = row.get(9).unwrap();
            let poi_info = path
                .filter(|p| !p.is_empty())
                .map(|p| PoiInfo { path: p.into() });
            Ok(ItemOfInterest {
                id: row.get(0).unwrap(),
                sql_table: row.get(1).unwrap(),
                project_name: row.get(2).unwrap(),
                poi_desc: row.get(4).unwrap(),
                env: row.get(5).unwrap(),
                item_text: row.get(6).unwrap(),
                item_type: row.get(7).unwrap(),
                server_info,
                poi_info,
            })
        })
        .unwrap();
    for server_poi in server_poi_iter {
        let poi = server_poi.unwrap();
        let _ = item_sender.send(Arc::new(crate::MyItem {
            display: poi.project_name.clone()
                + &render_optional_field(poi.env.as_ref())
                + " ▶ "
                + &poi.item_type.to_string()
                + &render_optional_field(poi.server_info.as_ref().map(|si| &si.server_desc))
                + &render_optional_field(
                    poi.server_info
                        .as_ref()
                        .map(|si| si.server_access_type.to_string()),
                )
                + &render_optional_field(poi.poi_desc.as_ref()),
            inner: poi,
        }));
    }
}

fn render_optional_field<S: std::fmt::Display>(field: Option<S>) -> String {
    field
        .as_ref()
        .map(|d| format!(" ▶ {}", d))
        .unwrap_or_else(|| "".to_string())
}

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
