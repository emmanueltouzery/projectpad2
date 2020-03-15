use rusqlite::{params, Connection};
extern crate skim;
use clipboard_ext::prelude::*;
use clipboard_ext::x11_fork::ClipboardContext;
use skim::prelude::*;

struct MyItem {
    inner: String,
    command: String,
}

impl SkimItem for MyItem {
    fn display(&self) -> Cow<AnsiString> {
        Cow::Owned(self.inner.as_str().into())
    }

    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.inner)
    }

    fn preview(&self) -> ItemPreview {
        if self.inner.starts_with("color") {
            ItemPreview::AnsiText(format!("\x1b[31mhello:\x1b[m\n{}", self.inner))
        } else {
            ItemPreview::Text(format!("hello:\n{}", self.inner))
        }
    }
}

pub fn main() {
    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(true)
        .preview(Some("")) // preview should be specified to enable preview window
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    std::thread::spawn(move || load_projects(tx_item));

    let selected_items = Skim::run_with(&options, Some(rx_item))
        .map(|out| out.selected_items)
        .unwrap_or_else(|| Vec::new());

    for item in selected_items.iter() {
        // this pattern from the skim apidocs for SkimItem, and also
        // https://stackoverflow.com/a/26128001/516188
        let myitem = (**item).as_any().downcast_ref::<MyItem>().unwrap();

        write_command_line_to_terminal(&myitem.command);
        copy_command_to_clipboard(&myitem.command);
    }
}

fn copy_command_to_clipboard(command_line: &str) {
    let mut ctx = ClipboardContext::new().unwrap();
    ctx.set_contents(command_line.into()).unwrap();
}

fn write_command_line_to_terminal(command_line: &str) {
    // https://unix.stackexchange.com/questions/213799/can-bash-write-to-its-own-input-stream/213821#213821
    unsafe {
        for byte in command_line.bytes() {
            libc::ioctl(libc::STDIN_FILENO, libc::TIOCSTI, &byte);
        }
    }

    // this code requires tmux. the ioctl is considered unsafe by some,
    // the tmux way could become more portable in the future possible?
    //
    // std::process::Command::new("tmux")
    //     .arg("send-key")
    //     .arg(&myitem.command)
    //     .status()
    //     .unwrap();
}

#[derive(Debug)]
struct ServerPoi {
    project_name: String,
    server_desc: String,
    server_poi_desc: String,
    server_env: String,
    server_poi_text: String,
}

fn load_projects(tx_sender: Sender<Arc<dyn SkimItem>>) {
    let service = "projectpad-cli";
    let conn = Connection::open("/home/emmanuel/.projectpad/projectpad.db").unwrap();
    let kr = keyring::Keyring::new(&service, &service);
    // kr.set_password("mc");
    conn.pragma_update(None, "key", &kr.get_password().unwrap())
        .unwrap();

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
        let _ = tx_sender.send(Arc::new(MyItem {
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
