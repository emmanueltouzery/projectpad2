use crate::database::ExecutedAction;
#[cfg(test)]
use crate::database::{ActionType, LinkedItem};
use database::DisplayMode;
use diesel::prelude::*;
use regex::Regex;
use skim::prelude::*;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use structopt::clap::arg_enum;
use structopt::StructOpt;
mod actions;
mod autoupgrade;
pub mod config;
mod database;
#[cfg_attr(target_os = "linux", path = "secretservice_linux.rs")]
#[cfg_attr(not(target_os = "linux"), path = "secretservice_generic.rs")]
mod secretservice;

const ZSH_FUNCTION: &str = include_str!("../shell/integration.zsh");

const MIN_SUPPORTED_DB_SCHEMA_VERSION: i32 = 21;
const MAX_SUPPORTED_DB_SCHEMA_VERSION: i32 = 22;

#[derive(StructOpt)]
#[structopt(version = env!("CARGO_PKG_VERSION"))]
struct Options {
    /// Upgrade ppcli
    #[structopt(long)]
    upgrade: bool,
    /// Disable color display
    #[structopt(long="no-color", parse(from_flag = display_from_no_color))]
    display_mode: DisplayMode,
    /// Disable the new version check
    #[structopt(long = "no-upgrade-check", parse(from_flag = std::ops::Not::not))]
    upgrade_check: bool,
    #[structopt(long = "shell-integration", hidden = true)]
    shell_integration_mode: bool,
    /// Print to stdout the function for a given shell
    #[structopt(long, default_value = "none")]
    print_shell_function: Shell,
}

arg_enum! {
    #[derive(PartialEq, Eq)]
    enum Shell {
        None,
        Zsh,
    }
}

pub struct MyItem {
    display: String,
    inner: actions::Action,
}

fn remove_ansi_escapes(input: &str) -> Cow<str> {
    let ansicode_regex = Regex::new("\x1b.*?m").unwrap();
    ansicode_regex.replace_all(input, "")
}

impl SkimItem for MyItem {
    fn display(&self, _context: DisplayContext) -> AnsiString {
        AnsiString::parse(self.display.as_str())
    }

    fn text(&self) -> Cow<str> {
        remove_ansi_escapes(&self.display)
            + "\n"
            // this text is used for filtering items when the user types.
            // also include the full POI descriptions so that we don't match
            // only against the truncated POI description
            + self.inner.item.poi_desc.as_deref().unwrap_or("")
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        ItemPreview::Text(
            "[enter] run, [alt-enter] paste to prompt, [ctl-y] copy to clipboard, [ctl-n/p] history".to_string(),
        )
    }
}

macro_rules! some_or_exit {
    ($op:expr, $msg: expr, $code: expr) => {{
        let val_r = $op;
        if val_r.is_none() {
            eprintln!($msg);
            std::process::exit($code);
        }
        val_r.unwrap()
    }};
}

macro_rules! ok_or_exit {
    ($op:expr, $msg: expr, $code: expr) => {{
        let val_r = $op;
        if let Result::Err(e) = val_r {
            eprintln!($msg, e);
            std::process::exit($code);
        }
        val_r.unwrap()
    }};
}

fn display_from_no_color(no_color: bool) -> DisplayMode {
    if no_color {
        DisplayMode::Plain
    } else {
        DisplayMode::Color
    }
}

enum UpgradeAvailableData {
    HasUpgrade(String),
    CheckedNoUpgrade,
    NoNeedToCheck,
}

pub fn main() {
    let flag_options = Options::from_args();
    if flag_options.upgrade {
        match autoupgrade::try_upgrade() {
            Ok(()) => {
                // don't bug the user about this upgrade for
                // some time now (even if the user rejected the upgrade)
                let _ = config::upgrade_check_mark_done();
            }
            Err(e) => {
                eprintln!("Error in auto-upgrade: {}", e);
                std::process::exit(1);
            }
        }
        std::process::exit(0);
    }
    if flag_options.print_shell_function == Shell::Zsh {
        println!("\n{}", ZSH_FUNCTION);
        std::process::exit(0);
    }
    let db_pass = ok_or_exit!(
        secretservice::get_keyring_pass().and_then(|r| r.ok_or_else(|| "no matching credentials".into())),
        "Cannot find the database password in the OS keyring, aborting: did you run the projectpad GUI app to create a database first? {}",
        1
    );

    let db_path_raw = projectpadsql::database_path();
    let db_path = some_or_exit!(
        db_path_raw.to_str(),
        "Cannot find the database path on disk, aborting",
        2
    );

    let conn = ok_or_exit!(
        SqliteConnection::establish(db_path),
        "Cannot open the database, aborting. {}",
        3
    );

    ok_or_exit!(
        projectpadsql::try_unlock_db(&conn, &db_pass),
        "Failed unlocking the database with the password, aborting. {}",
        4
    );

    ok_or_exit!(
        check_db_version(&conn),
        "{} https://github.com/emmanueltouzery/projectpad2",
        5
    );

    // start a thread to, if we didn't check for 7 days, check whether there is
    // a new version of ppcli available (in a thread not to block the GUI).
    // We write to a channel and check the contents of the channel at the end
    // of the runtime of the application
    let (has_upgrade_tx, has_upgrade_rx) = mpsc::channel::<UpgradeAvailableData>();
    if flag_options.upgrade_check {
        std::thread::spawn(move || {
            has_upgrade_tx
                .send(match config::upgrade_days_since_last_check() {
                    Ok(days) if days > 7 => {
                        if let Ok(Some(download_url)) = autoupgrade::is_upgrade_available() {
                            UpgradeAvailableData::HasUpgrade(download_url)
                        } else {
                            // also applies in case of errors. I could handle that
                            // and make it set NoNeedToCheck to avoid skipping another
                            // seven days, but I'm not concerned about that.
                            UpgradeAvailableData::CheckedNoUpgrade
                        }
                    }
                    // this also applies in case of errors, but that's OK,
                    // we want to hide errors for that feature, and NoNeedToCheck
                    // is a NOP.
                    _ => UpgradeAvailableData::NoNeedToCheck,
                })
                .unwrap()
        });
    } else {
        has_upgrade_tx
            .send(UpgradeAvailableData::NoNeedToCheck)
            .unwrap();
    }

    let history_strs = config::read_string_history().unwrap_or_else(|_| vec![]);
    let history_executed_actions = config::read_action_history().unwrap_or_else(|_| vec![]);
    let options = SkimOptionsBuilder::default()
        .bind(vec!["ctrl-p:previous-history", "ctrl-n:next-history"])
        .expect(Some("ctrl-y,alt-enter".to_string()))
        // .height(Some("50%"))
        // .multi(true)
        .preview(Some("")) // preview should be specified to enable preview window
        .preview_window(Some("up:2"))
        // .layout("reverse-list")
        // .reverse(true)
        .query_history(&history_strs)
        .exact(true)
        .case(CaseMatching::Ignore)
        .build()
        .unwrap();

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    let display_mode = flag_options.display_mode;
    let ranked_items = get_ranked_items(&history_executed_actions);
    std::thread::spawn(move || database::load_items(&conn, display_mode, &tx_item, &ranked_items));

    let (selected_items, query, accept_key) = Skim::run_with(&options, Some(rx_item))
        .map(|out| (out.selected_items, out.query, out.final_key))
        .unwrap_or_else(|| (Vec::new(), "".to_string(), Key::Enter));

    if let Some(item) = selected_items.get(0) {
        // this pattern from the skim apidocs for SkimItem, and also
        // https://stackoverflow.com/a/26128001/516188
        let myitem = (**item).as_any().downcast_ref::<MyItem>().unwrap();

        let item_of_interest = &myitem.inner.item;
        if !query.is_empty() {
            config::write_string_history(&history_strs, &query, 100).unwrap();
        }
        config::write_actions_history(
            &history_executed_actions,
            ExecutedAction::new(item_of_interest.linked_item, myitem.inner.desc),
            100,
        )
        .unwrap();

        let action = &myitem.inner;
        let action_str = &(action.get_string)(&action.item);
        let upgrade_url = if flag_options.shell_integration_mode {
            // in shell integration mode, we check for upgrades before handling
            // the command, because we just print out the command, the shell
            // will execute it.
            handle_upgrade_info_and_get_download_url(&has_upgrade_rx)
                .map(Cow::Owned)
                .unwrap_or(Cow::Borrowed(""))
        } else {
            Cow::Borrowed("")
        };
        match accept_key {
            Key::Ctrl('y') if flag_options.shell_integration_mode => println!("C\x00{}\x00\x00{}", action_str, upgrade_url),
            Key::Ctrl('y') => copy_command_to_clipboard(action_str),
            Key::AltEnter if flag_options.shell_integration_mode => println!("P\x00{}\x00\x00{}", action_str, upgrade_url),
            Key::AltEnter =>
            // copy to command-line if run is not allowed for that action
                    // if !val_action.allowed_actions.contains(&AllowedAction::Run) =>
            {
                write_command_line_to_terminal(action_str)
            }
            Key::Enter if flag_options.shell_integration_mode => println!(
                "R\x00{}\x00{}\x00{}", action_str, &run_command_folder(action)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "".to_string()), upgrade_url),
            Key::Enter => run_command(
                action_str,
                &run_command_folder(action)
                    .unwrap_or_else(|| dirs::home_dir().unwrap()),
            ),
            _ => {}
        }
    }
    if !flag_options.shell_integration_mode {
        if let Some(download_url) = handle_upgrade_info_and_get_download_url(&has_upgrade_rx) {
            if let Ok(()) = autoupgrade::apply_upgrade(&download_url) {
                let _ = config::upgrade_check_mark_done();
            }
        }
    }
}

fn handle_upgrade_info_and_get_download_url(
    has_upgrade_rx: &mpsc::Receiver<UpgradeAvailableData>,
) -> Option<String> {
    // try_recv, don't want to block there... don't want
    // ppcli to block at the end of the runtime if it's run
    // on a system without network for instance.
    match has_upgrade_rx.try_recv() {
        Ok(UpgradeAvailableData::HasUpgrade(download_url)) => Some(download_url),
        Ok(UpgradeAvailableData::CheckedNoUpgrade) => {
            let _ = config::upgrade_check_mark_done();
            None
        }
        _ => None,
    }
}

/// we want that between two with the same count the latest wins.
/// so we rank by a tuple: first the number of uses of that command,
/// second the index of the last use of that command.
fn get_ranked_items(
    history_executed_actions: &[ExecutedAction],
) -> HashMap<ExecutedAction, (usize, usize)> {
    history_executed_actions
        .iter()
        .enumerate()
        .fold(HashMap::new(), |mut sofar, (i, cur)| {
            let sofar_pair = sofar.entry(*cur).or_insert((0, i));
            sofar_pair.0 += 1;
            sofar_pair.1 = i;
            sofar
        })
}

fn run_command_folder(action: &actions::Action) -> Option<PathBuf> {
    Some(&action.item)
        .filter(|p| p.server_info.is_none()) // remote paths are not relevant!
        .and_then(|i| i.poi_info.as_ref())
        .map(|p| p.path.clone())
}

fn check_db_version(conn: &SqliteConnection) -> Result<(), Box<dyn std::error::Error>> {
    let version = projectpadsql::get_db_version(conn)?;
    if version < MIN_SUPPORTED_DB_SCHEMA_VERSION {
        return Err(format!("The database version ({}), is older than the oldest version supported by this application. Please upgrade the main projectpad application.", version).into());
    }
    if version > MAX_SUPPORTED_DB_SCHEMA_VERSION {
        println!("The database version ({}), is newer than the newest version supported by this application. Please upgrade this CLI application.", version);
        if let Err(e) = autoupgrade::try_upgrade() {
            eprintln!("Error in auto-upgrade: {}", e);
        }
        std::process::exit(1);
    }
    Ok(())
}

fn run_command(command_line: &str, cur_dir: &Path) {
    let cl_elts = shell_words::split(command_line).unwrap_or_else(|e| {
        println!("Couldn't parse the command: {}: {}", command_line, e);
        Vec::new()
    });
    if !cl_elts.is_empty() {
        // the reason for the println is that some commands need
        // some time before they print out any output -- for instance
        // ssh on a far, slow server. With this println we give some
        // feedback to the user.
        let actual_dir = if cur_dir.as_os_str().is_empty() {
            Cow::Owned(std::env::current_dir().unwrap())
        } else {
            Cow::Borrowed(cur_dir)
        };
        println!("Running {} in folder {:?}...", command_line, actual_dir);
        Command::new(cl_elts[0].clone())
            .args(cl_elts.iter().skip(1))
            .current_dir::<&Path>(actual_dir.borrow())
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

#[test]
fn remove_ansi_escapes_prd() {
    assert_eq!("❚PRD", remove_ansi_escapes("\x1b[31m\x1b[1m❚P\x1b[0mRD"));
}

#[test]
fn get_ranked_items_should_work() {
    let action1 = ExecutedAction::new(LinkedItemId::Server(6), ActionType::SshShell);
    let action2 = ExecutedAction::new(LinkedItemId::ServerPoi(2), ActionType::FetchCfg);
    let action3 = ExecutedAction::new(LinkedItemId::Server(3), ActionType::SshShell);
    assert_eq!(
        vec![(action1, (3, 4)), (action3, (2, 5)), (action2, (1, 1))]
            .into_iter()
            .collect::<HashMap<_, _>>(),
        get_ranked_items(&[action1, action2, action3, action1, action1, action3])
    )
}
