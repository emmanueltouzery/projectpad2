use rusqlite::{params, Connection};
use skim::prelude::*;
use std::borrow::Cow;

#[derive(Debug)]
pub struct ItemOfInterest {
    id: i32,
    sql_table: String,
    project_name: String,
    env: String,
    item_type: String,
    server_desc: String,
    poi_desc: Option<String>,
    item_text: String,
}

const SERVER_POIS_QUERY: &str = r#"SELECT server_point_of_interest.id, 'server_point_of_interest',
                     project.name, server.desc, server_point_of_interest.desc,
                     server.environment, server_point_of_interest.text, server_point_of_interest.interest_type
                 from server_point_of_interest
                 join server on server.id = server_point_of_interest.server_id
                 join project on project.id = server.project_id"#;

const SERVERS_QUERY: &str = r#"SELECT server.id, 'server', project.name, server.desc, NULL,
                     server.environment, server.ip, server.type || ' ' || server.access_type
                 from server
                 join project on project.id = server.project_id"#;

pub fn load_items(db_pass: &str, item_sender: &Sender<Arc<dyn SkimItem>>) {
    let conn = Connection::open(crate::config::database_path()).unwrap(); // TODO react better if no DB
    conn.pragma_update(None, "key", &db_pass).unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "{} UNION ALL {} order by project.name",
            SERVER_POIS_QUERY, SERVERS_QUERY
        ))
        .unwrap();
    let server_poi_iter = stmt
        .query_map(params![], |row| {
            Ok(ItemOfInterest {
                id: row.get(0).unwrap(),
                sql_table: row.get(1).unwrap(),
                project_name: row.get(2).unwrap(),
                server_desc: row.get(3).unwrap(),
                poi_desc: row.get(4).unwrap(),
                env: row.get(5).unwrap(),
                item_text: row.get(6).unwrap(),
                item_type: row.get(7).unwrap(),
            })
        })
        .unwrap();
    for server_poi in server_poi_iter {
        let poi = server_poi.unwrap();
        let _ = item_sender.send(Arc::new(crate::MyItem {
            display: poi.project_name.clone()
                + " "
                + &poi.env
                + " ▶ "
                + &poi.item_type
                + " ▶ "
                + &poi.server_desc
                + &poi
                    .poi_desc
                    .as_ref()
                    .map(|d| format!(" ▶ {}", d))
                    .unwrap_or_else(|| "".to_string()),
            inner: poi,
        }));
    }
}

fn get_value_server(item: &ItemOfInterest) -> std::borrow::Cow<str> {
    if let [addr, port] = item.item_text.split(":").collect::<Vec<&str>>()[..] {
        Cow::Owned(format!("ssh -p {} {}", port, addr))
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
