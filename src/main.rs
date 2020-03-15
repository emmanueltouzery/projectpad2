use clipboard_ext::prelude::*;
use clipboard_ext::x11_fork::ClipboardContext;
use skim::prelude::*;
pub mod config;
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
        if self.display.starts_with("color") {
            ItemPreview::AnsiText(format!("\x1b[31mhello:\x1b[m\n{}", self.display))
        } else {
            ItemPreview::Text(format!("hello:\n{}", self.display))
        }
    }
}

pub fn main() {
    let history = config::read_history().unwrap_or(vec![]);
    let options = SkimOptionsBuilder::default()
        .bind(vec!["ctrl-p:previous-history", "ctrl-n:next-history"])
        .height(Some("50%"))
        .multi(true)
        .preview(Some("")) // preview should be specified to enable preview window
        .query_history(&history)
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    std::thread::spawn(move || load_items(tx_item));

    let (selected_items, query) = Skim::run_with(&options, Some(rx_item))
        .map(|out| (out.selected_items, out.query))
        .unwrap_or_else(|| (Vec::new(), "".to_string()));

    for item in selected_items.iter() {
        // this pattern from the skim apidocs for SkimItem, and also
        // https://stackoverflow.com/a/26128001/516188
        let myitem = (**item).as_any().downcast_ref::<MyItem>().unwrap();

        let val = database::get_value(&myitem.inner);
        write_command_line_to_terminal(&val);
        copy_command_to_clipboard(&val);
        if !query.is_empty() {
            config::write_history(&history, &query, 100).unwrap();
        }
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
