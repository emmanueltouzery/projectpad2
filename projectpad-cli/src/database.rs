use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{params, Connection};
use skim::prelude::*;
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

#[derive(Debug, EnumString, Display, PartialEq)]
pub enum EnvironmentType {
    EnvDevelopment,
    EnvUat,
    EnvStage,
    EnvProd,
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
from_sql_from_str!(EnvironmentType);

#[derive(Debug)]
pub struct ServerInfo {
    pub server_desc: String,
    pub server_username: String,
    pub server_ip: String,
    pub server_access_type: SrvAccessType,
}

#[derive(Debug)]
pub struct PoiInfo {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ItemOfInterest {
    pub id: i32,
    pub sql_table: String,
    pub project_name: String,
    pub env: Option<EnvironmentType>,
    pub item_type: ItemType,
    pub poi_desc: Option<String>,
    pub item_text: String,
    pub server_info: Option<ServerInfo>,
    pub poi_info: Option<PoiInfo>,
}

// for now exclude RDP & WWW
const SERVER_POIS_QUERY: &str = r#"SELECT server_point_of_interest.id, 'server_point_of_interest',
                     project.name, server.desc, server_point_of_interest.desc,
                     server.environment, server_point_of_interest.text, server_point_of_interest.interest_type,
                     server.username, server_point_of_interest.path, server.access_type, server.ip
                 from server_point_of_interest
                 join server on server.id = server_point_of_interest.server_id
                 join project on project.id = server.project_id
                 where server.access_type not in ('SrvAccessRdp', 'SrvAccessWww')"#;

const PROJECT_POIS_QUERY: &str = r#"SELECT project_point_of_interest.id, 'project_point_of_interest',
                     project.name, NULL, project_point_of_interest.desc,
                     NULL, project_point_of_interest.text, project_point_of_interest.interest_type,
                     NULL, project_point_of_interest.path, NULL, NULL
                 from project_point_of_interest
                 join project on project.id = project_point_of_interest.project_id"#;

// for now exclude RDP & WWW
const SERVERS_QUERY: &str = r#"SELECT server.id, 'server', project.name, server.desc, NULL,
                     server.environment, server.ip, server.type,
                     server.username, NULL, server.access_type, server.ip
                 from server
                 join project on project.id = server.project_id
                 where server.access_type not in ('SrvAccessRdp', 'SrvAccessWww')"#;

pub fn load_items(db_pass: &str, item_sender: &Sender<Arc<dyn SkimItem>>) {
    let conn = Connection::open(projectpadsql::database_path()).unwrap(); // TODO react better if no DB
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
    let cols_spec = vec![7, 3, 12, 40, 10, 50];
    for server_poi in server_poi_iter {
        let poi = server_poi.unwrap();
        let _ = item_sender.send(Arc::new(crate::MyItem {
            display: render_row(&cols_spec, &poi),
            inner: poi,
        }));
    }
}

fn render_row(cols_spec: &[usize], item: &ItemOfInterest) -> String {
    let mut col1 = item.project_name.clone();
    col1.truncate(cols_spec[0]);
    let mut col2 = item
        .env
        .as_ref()
        .map(display_env)
        .unwrap_or("-")
        .to_string();
    col2.truncate(cols_spec[1]);
    let mut col3 =
        render_type_emoji(&item.item_type).to_string() + " " + render_item_type(&item.item_type);
    col3.truncate(cols_spec[2]);
    let mut col4 = item
        .server_info
        .as_ref()
        .map(|si| si.server_desc.clone())
        .unwrap_or_else(|| "-".to_string());
    col4.truncate(cols_spec[3]);
    let mut col5 = item
        .server_info
        .as_ref()
        .map(|si| render_access_type(&si.server_access_type))
        .unwrap_or_else(|| "-")
        .to_string();
    col5.truncate(cols_spec[4]);
    let mut col6 = item
        .poi_desc
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "".to_string());
    col6.truncate(cols_spec[5]);
    format!(
        "{:<w1$} {:<w2$} {:<w3$} {:<w4$} {:<w5$} {:<w6$}",
        col1,
        col2,
        col3,
        col4,
        col5,
        col6,
        w1 = cols_spec[0],
        w2 = cols_spec[1],
        w3 = cols_spec[2],
        w4 = cols_spec[3],
        w5 = cols_spec[4],
        w6 = cols_spec[5],
    )
}

fn display_env(env: &EnvironmentType) -> &'static str {
    match env {
        EnvironmentType::EnvDevelopment => "Dev",
        EnvironmentType::EnvUat => "Uat",
        EnvironmentType::EnvStage => "Stg",
        EnvironmentType::EnvProd => "Prod",
    }
}

fn render_access_type(access: &SrvAccessType) -> &'static str {
    match access {
        SrvAccessType::SrvAccessSsh => "ssh",
        SrvAccessType::SrvAccessRdp => "RDP",
        SrvAccessType::SrvAccessWww => "www",
        SrvAccessType::SrvAccessSshTunnel => "ssh tunnel",
    }
}

fn render_item_type(item_type: &ItemType) -> &'static str {
    match item_type {
        ItemType::PoiCommandToRun => "Command",
        ItemType::PoiCommandTerminal => "Command",
        ItemType::PoiConfigFile => "Config",
        ItemType::PoiLogFile => "Log",
        ItemType::PoiApplication => "App",
        ItemType::PoiBackupArchive => "Backup",
        ItemType::SrvApplication => "App",
        ItemType::SrvDatabase => "DB",
        ItemType::SrvHttpOrProxy => "HttpOrProxy",
        ItemType::SrvReporting => "Reporting",
        ItemType::SrvMonitoring => "Monitoring",
    }
}

fn render_type_emoji(item_type: &ItemType) -> &'static str {
    match item_type {
        ItemType::PoiCommandToRun => "🚀",    // ⚙ 🖥
        ItemType::PoiCommandTerminal => "🚀", // ⚙ 🖥
        ItemType::PoiConfigFile => "🔧",      // 🗄
        ItemType::PoiLogFile => "📼",
        ItemType::PoiApplication => "📂",
        ItemType::PoiBackupArchive => "💾", // 📦
        ItemType::SrvApplication => "🏭",
        ItemType::SrvDatabase => "💽",
        ItemType::SrvHttpOrProxy => "🌐",
        ItemType::SrvReporting => "📰",
        ItemType::SrvMonitoring => "📈", //🌡
    }
}
