use rusqlite::{params, Connection};

#[derive(Debug)]
struct Project {
    name: String,
}

fn main() {
    let service = "projectpad-cli";
    let conn = Connection::open("/home/emmanuel/.projectpad/projectpad.db").unwrap();
    let kr = keyring::Keyring::new(&service, &service);
    // kr.set_password("mc");
    // conn.execute("PRAGMA key=?1", params![kr.get_password().unwrap()])
    conn.pragma_update(None, "key", &kr.get_password().unwrap())
        .unwrap();

    let mut stmt = conn.prepare("SELECT name from project").unwrap();
    let project_iter = stmt
        .query_map(params![], |row| {
            Ok(Project {
                name: row.get(0).unwrap(),
            })
        })
        .unwrap();
    for project in project_iter {
        println!("Found project: {:?}", project.unwrap());
    }
}
