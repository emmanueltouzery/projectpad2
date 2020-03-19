use clipboard_ext::prelude::*;
use clipboard_ext::x11_fork::ClipboardContext;
use skim::prelude::*;
use std::process::Command;
pub mod config;
use std::path::PathBuf;
mod actions;
mod database;

pub struct MyItem {
    display: String,
    inner: database::ItemOfInterest,
}

impl SkimItem for MyItem {
    fn display(&self) -> Cow<AnsiString> {
        Cow::Owned(self.display.as_str().into())
    }

    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.display)
    }

    fn preview(&self) -> ItemPreview {
        ItemPreview::Text(
            actions::get_value(&self.inner)
                .into_iter()
                .map(|a| a.desc)
                .collect::<Vec<String>>()
                .join(", ")
                .into(),
        )
        // if self.display.starts_with("color") {
        //     ItemPreview::AnsiText(format!("\x1b[31mhello:\x1b[m\n{}", self.display))
        // } else {
        //     ItemPreview::Text(format!("hello:\n{}", self.display))
        // }
    }
}

pub fn main() {
    let history = config::read_history().unwrap_or_else(|_| vec![]);
    let options = SkimOptionsBuilder::default()
        .bind(vec!["ctrl-p:previous-history", "ctrl-n:next-history"])
        .expect(Some("ctrl-j,ctrl-k".to_string()))
        .height(Some("50%"))
        // .multi(true)
        .preview(Some("")) // preview should be specified to enable preview window
        .preview_window(Some("up:2"))
        .query_history(&history)
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    std::thread::spawn(move || load_items(tx_item));

    let (selected_items, query, accept_key) = Skim::run_with(&options, Some(rx_item))
        .map(|out| (out.selected_items, out.query, out.accept_key))
        .unwrap_or_else(|| (Vec::new(), "".to_string(), Some("other".to_string())));

    if let Some(item) = selected_items.iter().next() {
        // this pattern from the skim apidocs for SkimItem, and also
        // https://stackoverflow.com/a/26128001/516188
        let myitem = (**item).as_any().downcast_ref::<MyItem>().unwrap();

        let val_actions = actions::get_value(&myitem.inner);
        let val_fn = &val_actions.first().unwrap().get_string;
        let val = val_fn(&myitem.inner);
        match accept_key.as_deref() {
            Some("ctrl-j") => write_command_line_to_terminal(&val),
            Some("ctrl-k") => copy_command_to_clipboard(&val),
            _ => run_command(
                &val,
                &Some(&myitem.inner)
                    .filter(|p| p.server_info.is_none()) // remote paths are not relevant!
                    .and_then(|i| i.poi_info.as_ref())
                    .map(|p| p.path.clone())
                    .unwrap_or_else(|| dirs::home_dir().unwrap()),
            ),
        }
        if !query.is_empty() {
            config::write_history(&history, &query, 100).unwrap();
        }
    }
}

fn run_command(command_line: &str, cur_dir: &PathBuf) {
    let cl_elts = shell_words::split(command_line).unwrap_or_else(|e| {
        println!("Couldn't parse the command: {}: {}", command_line, e);
        Vec::new()
    });
    if !cl_elts.is_empty() {
        // the reason for the println is that some commands need
        // some time before they print out any output -- for instance
        // ssh on a far, slow server. With this println we give some
        // feedback to the user.
        println!("Running {} in folder {:?}...", command_line, cur_dir);
        Command::new(cl_elts[0].clone())
            .args(cl_elts.iter().skip(1))
            .current_dir(cur_dir)
            .status()
            .map(|_| ())
            .unwrap_or_else(|e| {
                println!("Error launching process: {}", e);
            });
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

fn load_items(item_sender: Sender<Arc<dyn SkimItem>>) {
    let service = "projectpad-cli";
    let kr = keyring::Keyring::new(&service, &service);
    // kr.set_password("mc");
    database::load_items(&kr.get_password().unwrap(), &item_sender);
}
