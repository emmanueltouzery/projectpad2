use rusqlite::{params, Connection};
use skim::prelude::*;
use std::borrow::Cow;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ServerInfo {
    server_desc: String,
    server_username: String,
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
    item_type: String,
    poi_desc: Option<String>,
    item_text: String,
    server_info: Option<ServerInfo>,
    pub poi_info: Option<PoiInfo>,
}

const SERVER_POIS_QUERY: &str = r#"SELECT server_point_of_interest.id, 'server_point_of_interest',
                     project.name, server.desc, server_point_of_interest.desc,
                     server.environment, server_point_of_interest.text, server_point_of_interest.interest_type,
                     server.username, server_point_of_interest.path
                 from server_point_of_interest
                 join server on server.id = server_point_of_interest.server_id
                 join project on project.id = server.project_id"#;

const PROJECT_POIS_QUERY: &str = r#"SELECT project_point_of_interest.id, 'project_point_of_interest',
                     project.name, NULL, project_point_of_interest.desc,
                     NULL, project_point_of_interest.text, project_point_of_interest.interest_type,
                     NULL, project_point_of_interest.path
                 from project_point_of_interest
                 join project on project.id = project_point_of_interest.project_id"#;

const SERVERS_QUERY: &str = r#"SELECT server.id, 'server', project.name, server.desc, NULL,
                     server.environment, server.ip, server.type || ' ' || server.access_type,
                     server.username, NULL
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
            let server_info = match (server_desc, server_username) {
                (Some(d), Some(u)) => Some(ServerInfo {
                    server_desc: d,
                    server_username: u,
                }),
                _ => None,
            };
            let path: Option<String> = row.get(9).unwrap();
            let poi_info = path.map(|p| PoiInfo { path: p.into() });
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
                + &poi.item_type
                + " ▶ "
                + &render_optional_field(poi.server_info.as_ref().map(|si| &si.server_desc))
                + &render_optional_field(poi.poi_desc.as_ref()),
            inner: poi,
        }));
    }
}

fn render_optional_field(field: Option<&String>) -> String {
    field
        .as_ref()
        .map(|d| format!(" ▶ {}", d))
        .unwrap_or_else(|| "".to_string())
}

fn get_value_server(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if let [addr, port] = item.item_text.split(':').collect::<Vec<&str>>()[..] {
        Cow::Owned(format!(
            "ssh -p {} {}@{}",
            port,
            item.server_info.as_ref().unwrap().server_username,
            addr
        ))
    } else {
        Cow::Borrowed(&item.item_text)
    }
}

pub fn get_value(item: &ItemOfInterest) -> Cow<str> {
    match item.sql_table.as_str() {
        "server" => get_value_server(item),
        _ => Cow::Borrowed(&item.item_text),
    }
}
