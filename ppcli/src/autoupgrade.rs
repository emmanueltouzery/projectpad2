use std::env;
use std::path::Path;
use std::process::Command;

// i don't want to add too many dependencies to ppcli itself,
// just because of the auto-upgrade...
// i would need to depend on a http library (ok, i already have
// openssl), and a tar.gz extracting library. Not that bad, but...
pub fn try_upgrade() -> Result<(), Box<dyn std::error::Error>> {
    if !is_in_path("curl")
        || !is_in_path("jq")
        || !is_in_path("grep")
        || !is_in_path("head")
        || !is_in_path("tar")
    {
        return Err("Auto-upgrade requires the following programs in the path: curl, jq, grep, head and tar.".into());
    }

    let path = env::args().next().ok_or("can't get the current app path")?;
    let full_path = std::fs::canonicalize(&path)?;
    let parent = full_path
        .parent()
        .ok_or("can't get the parent folder of the ppcli install")?;
    let parent_str = parent
        .to_str()
        .ok_or("the ppcli parent folder is an invalid string")?;
    println!("{}", parent_str);
    let download_bytes = Command::new("sh")
        .args(&[
            "-c",
            &[
                "curl -sSf https://api.github.com/repos/emmanueltouzery/projectpad2/releases \
                               | jq '.[] | .assets | select(length > 0)' \
                               | jq -r '.[] | .browser_download_url' \
                               | grep cli \
                               | grep ",
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

    Command::new("sh")
        .args(&[
            "-c",
            &("cd ".to_string() + parent_str + " && curl -L " + download_url + " | tar -xz"),
        ])
        .spawn()?
        .wait()?;

    Ok(())
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
