use skim::prelude::*;
use std::io::Write;
use std::process::{Command, Stdio};
pub mod config;
use std::path::PathBuf;
mod actions;
mod database;

pub struct MyItem {
    display: String,
    inner: actions::Action,
}

impl SkimItem for MyItem {
    fn display(&self, _context: DisplayContext) -> AnsiString {
        self.display.as_str().into()
    }

    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.display)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(
            "[enter]: run, [alt-enter]: paste to prompt, [ctrl-y]: copy to clipboard".to_string(),
        )
    }
}

pub fn main() {
    let history = config::read_history().unwrap_or_else(|_| vec![]);
    let options = SkimOptionsBuilder::default()
        .bind(vec!["ctrl-p:previous-history", "ctrl-n:next-history"])
        .expect(Some("ctrl-y,alt-enter".to_string()))
        .height(Some("50%"))
        // .multi(true)
        .preview(Some("")) // preview should be specified to enable preview window
        .preview_window(Some("up:2"))
        .query_history(&history)
        .exact(true)
        .case(CaseMatching::Ignore)
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    std::thread::spawn(move || load_items(tx_item));

    let (selected_items, query, accept_key) = Skim::run_with(&options, Some(rx_item))
        .map(|out| (out.selected_items, out.query, out.final_key))
        .unwrap_or_else(|| (Vec::new(), "".to_string(), Key::Enter));

    if let Some(item) = selected_items.get(0) {
        // this pattern from the skim apidocs for SkimItem, and also
        // https://stackoverflow.com/a/26128001/516188
        let myitem = (**item).as_any().downcast_ref::<MyItem>().unwrap();

        let action = &myitem.inner;
        let action_str = &(action.get_string)(&action.item);
        match accept_key {
            Key::Ctrl('y') => copy_command_to_clipboard(action_str),
            Key::AltEnter =>
            // copy to command-line if run is not allowed for that action
                    // if !val_action.allowed_actions.contains(&AllowedAction::Run) =>
            {
                write_command_line_to_terminal(action_str)
            }
            Key::Enter => run_command(
                action_str,
                &Some(&action.item)
                    .filter(|p| p.server_info.is_none()) // remote paths are not relevant!
                    .and_then(|i| i.poi_info.as_ref())
                    .map(|p| p.path.clone())
                    .unwrap_or_else(|| dirs::home_dir().unwrap()),
            ),
            _ => {}
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
    // there are libraries for that in rust, earlier i was using
    // clibpoard-ext, but:
    // - there are issues with keeping the contents of the clipboard
    //   after the app exits (need to fork, stay alive..)
    // - must link to a series of X11 or wayland-related libraries,
    //   on linux. But I want a static build so that i can distribute
    //   a cross-distro binary.
    // due to that, rather leverage wl-copy and xsel
    // it seems xsel is a better choice than xclip:
    // https://askubuntu.com/questions/705620/xclip-vs-xsel/898094#898094

    // detect wayland or X11 https://unix.stackexchange.com/a/559950/36566
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        match Command::new("wl-copy")
            .arg(command_line)
            .spawn()
            .and_then(|mut p| p.wait())
        {
            Result::Err(e) => eprintln!("Failed to invoke wl-copy: {}", e),
            Result::Ok(s) if !s.success() => eprintln!("Got error status from wl-copy: {}", s),
            _ => {}
        }
    } else if std::env::var("DISPLAY").is_ok() {
        // https://stackoverflow.com/a/49597789/516188
        if let Result::Err(e) = Command::new("xsel")
            .arg("--clipboard")
            .stdin(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                let child_stdin = child.stdin.as_mut().unwrap();
                let write_res = child_stdin.write_all(command_line.as_bytes());
                if write_res.is_err() {
                    write_res
                } else {
                    let wait_res = child.wait();
                    match wait_res {
                        Result::Ok(s) if !s.success() => Result::Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("error status: {}", s),
                        )),
                        _ => wait_res.map(|_| ()),
                    }
                }
            })
        {
            eprintln!("Error in xsel: {:?}", e);
        }
    } else {
        eprintln!("The system seems to be neither wayland nor X11, don't know how to copy to the clipboard");
    }
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
    database::load_items(
        &projectpadsql::get_pass_from_keyring().unwrap(),
        &item_sender,
    );
}
