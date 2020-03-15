use rusqlite::{params, Connection};
use skim::prelude::*;

#[derive(Debug)]
struct ServerPoi {
    project_name: String,
    server_desc: String,
    server_poi_desc: String,
    server_env: String,
    server_poi_text: String,
}

pub fn load_items(db_pass: &str, item_sender: &Sender<Arc<dyn SkimItem>>) {
    let conn = Connection::open(crate::config::database_path()).unwrap(); // TODO react better if no DB
    conn.pragma_update(None, "key", &db_pass).unwrap();

    let mut stmt = conn
        .prepare(
            r#"SELECT project.name, server.desc, server_point_of_interest.desc,
                     server.environment, server_point_of_interest.text from server_point_of_interest
                 join server on server.id = server_point_of_interest.server_id
                 join project on project.id = server.project_id
                 order by project.name"#,
        )
        .unwrap();
    let server_poi_iter = stmt
        .query_map(params![], |row| {
            Ok(ServerPoi {
                project_name: row.get(0).unwrap(),
                server_desc: row.get(1).unwrap(),
                server_poi_desc: row.get(2).unwrap(),
                server_env: row.get(3).unwrap(),
                server_poi_text: row.get(4).unwrap(),
            })
        })
        .unwrap();
    for server_poi in server_poi_iter {
        let poi = server_poi.unwrap();
        let _ = item_sender.send(Arc::new(crate::MyItem {
            inner: poi.project_name
                + " "
                + &poi.server_env
                + " ▶ "
                + &poi.server_desc
                + " ▶ "
                + &poi.server_poi_desc,
            command: poi.server_poi_text,
        }));
    }
}
