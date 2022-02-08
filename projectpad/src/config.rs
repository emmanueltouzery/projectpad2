use gtk::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::*;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Sync + Send>>;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub prefer_dark_theme: bool,
}

impl Config {
    pub fn default_config() -> Config {
        Config {
            prefer_dark_theme: false,
        }
    }

    pub fn config_folder() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().expect("Can't find your home folder?");
        let config_folder = home_dir.join(".projectpad");
        if !config_folder.is_dir() {
            fs::create_dir(&config_folder)?;
        }
        Ok(config_folder)
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_folder = Self::config_folder()?;
        Ok(config_folder.join("config.toml"))
    }

    fn read_config_file() -> Result<Config> {
        let config_file = Self::config_path()?;
        if !config_file.is_file() {
            return Ok(Self::default_config());
        }
        let mut contents = String::new();
        File::open(config_file)?.read_to_string(&mut contents)?;
        let r = toml::from_str(&contents)?;
        Ok(r)
    }

    pub fn read_config() -> Config {
        Config::read_config_file().unwrap_or_else(|e| {
            let dialog = gtk::MessageDialog::new(
                None::<&gtk::Window>,
                gtk::DialogFlags::all(),
                gtk::MessageType::Error,
                gtk::ButtonsType::Close,
                "Error loading the configuration",
            );
            dialog.set_secondary_text(Some(&format!(
                "{}: {:}",
                Config::config_path()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "".to_string()),
                e
            )));
            let _r = dialog.run();
            dialog.close();
            Config::default_config()
        })
    }

    fn save_config_file(&self) -> Result<()> {
        let mut file = File::create(Self::config_path()?)?;
        file.write_all(toml::to_string_pretty(self)?.as_bytes())?;
        Ok(())
    }

    pub fn save_config(&self, parent_win: &gtk::Window) {
        self.save_config_file().unwrap_or_else(|e| {
            let dialog = gtk::MessageDialog::new(
                Some(parent_win),
                gtk::DialogFlags::all(),
                gtk::MessageType::Error,
                gtk::ButtonsType::Close,
                "Error saving the configuration",
            );
            dialog.set_secondary_text(Some(&format!("{}", e)));
            let _r = dialog.run();
            dialog.close();
        });
    }
}
