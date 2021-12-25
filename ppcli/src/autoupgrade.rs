use std::env;
use std::path::Path;
use std::process::Command;

type UResult<T> = Result<T, Box<dyn std::error::Error>>;

fn has_requirements() -> bool {
    is_in_path("curl")
        && is_in_path("jq")
        && is_in_path("grep")
        && is_in_path("head")
        && is_in_path("tar")
}

pub fn apply_upgrade(download_url: &str) -> UResult<()> {
    println!(
        "ppcli has detected a ppcli version at:\n   {}\n   current version: {}\nUpgrade? y/n",
        download_url,
        env!("CARGO_PKG_VERSION")
    );
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input_trimmed = input.trim();
    if input_trimmed != "y" && input_trimmed != "Y" {
        return Ok(());
    }
    let parent_str = ppcli_path()?;

    Command::new("sh")
        .args(&[
            "-c",
            &("cd ".to_string() + &parent_str + " && curl -L " + download_url + " | tar -xz"),
        ])
        .spawn()?
        .wait()?;

    Ok(())
}

fn ppcli_path() -> UResult<String> {
    // https://stackoverflow.com/a/4025426/516188 linuxism...
    let full_path = match std::fs::read_link("/proc/self/exe") {
        Ok(p) => Ok(p),
        Err(_) => {
            // presumably, not linux... get argv[0] and assume it's the
            // path of the executable.
            let path = env::args().next().ok_or("can't get the current app path")?;
            // TODO the user did not necessarily launch through the full path
            // should go through 'which' if path doesn't exist on disk.
            std::fs::canonicalize(&path)
        }
    }?;
    let parent = full_path
        .parent()
        .ok_or("can't get the parent folder of the ppcli install")?;
    let parent_str = parent
        .to_str()
        .ok_or("the ppcli parent folder is an invalid string")?;
    Ok(parent_str.to_owned())
}

fn get_latest_download_url() -> UResult<String> {
    if !has_requirements() {
        return Err("Auto-upgrade requires the following programs in the path: curl, jq, grep, head and tar.".into());
    }

    let download_bytes = Command::new("sh")
        .args(&[
            "-c",
            &[
                "curl -sSf https://api.github.com/repos/emmanueltouzery/projectpad2/releases \
                               | jq '.[] | .assets | select(length > 0)' \
                               | jq -r '.[] | .browser_download_url' \
                               | grep cli \
                               | grep ",
                std::env::consts::OS,
                "_",
                std::env::consts::ARCH,
                " | head -n 1",
            ]
            .join(""),
        ])
        .output()?
        .stdout;
    let download_url = std::str::from_utf8(&download_bytes)?.trim();
    if !download_url.starts_with("https://") {
        return Err("can't find a URL of a newer version of ppcli".into());
    }
    Ok(download_url.to_owned())
}

fn download_url_extract_version(download_url: &str) -> UResult<&str> {
    let fname = download_url
        .rsplitn(2, '/')
        .next()
        .ok_or_else(|| format!("failed parsing download URL: {}", download_url))?;
    fname
        .splitn(3, '-')
        .nth(1)
        .ok_or_else(|| format!("failed parsing download URL: {}", download_url).into())
}

// i don't want to add too many dependencies to ppcli itself,
// just because of the auto-upgrade...
// i would need to depend on a http library (ok, i already have
// openssl), and a tar.gz extracting library. Not that bad, but...
pub fn try_upgrade() -> UResult<()> {
    let download_url = get_latest_download_url()?;
    apply_upgrade(&download_url)
}

pub fn is_upgrade_available() -> UResult<Option<String>> {
    let download_url = get_latest_download_url()?;
    let version = download_url_extract_version(&download_url)?;
    let is_new_version = version != env!("CARGO_PKG_VERSION");
    Ok(Some(download_url).filter(|_| is_new_version))
}

// https://stackoverflow.com/a/37499032/516188
fn is_in_path<P>(exe_name: P) -> bool
where
    P: AsRef<Path>,
{
    env::var_os("PATH")
        .and_then(|paths| {
            env::split_paths(&paths)
                .filter_map(|dir| {
                    let full_path = dir.join(&exe_name);
                    if full_path.is_file() {
                        Some(full_path)
                    } else {
                        None
                    }
                })
                .next()
        })
        .is_some()
}

#[test]
fn parses_version_from_url() {
    assert_eq!("2.1.0", download_url_extract_version(
        "https://github.com/emmanueltouzery/projectpad2/releases/download/v2.1.0/ppcli-2.1.0-linux_x86_64.tgz").unwrap());
}
