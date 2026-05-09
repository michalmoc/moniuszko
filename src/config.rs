use crate::constants::APP_NAME;
use anyhow::anyhow;
use dirs::{audio_dir, config_local_dir};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub window_width: i32,
    pub window_height: i32,
    pub window_maximized: bool,

    pub media_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_width: 640,
            window_height: 480,
            window_maximized: false,
            media_path: audio_dir().unwrap(),
        }
    }
}

impl Config {
    fn base_config_path() -> PathBuf {
        config_local_dir().unwrap().join(APP_NAME)
    }

    fn config_path() -> PathBuf {
        Self::base_config_path().join("config.json")
    }

    pub fn database_path(&self) -> PathBuf {
        Self::base_config_path().join("database.json")
    }

    pub fn playlists_path(&self) -> PathBuf {
        Self::base_config_path().join("playlists.json")
    }

    pub fn load() -> anyhow::Result<Config> {
        if let Ok(file) = File::open(Self::config_path()) {
            Ok(serde_json::from_reader(file)?)
        } else {
            Ok(Default::default())
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        fs::create_dir_all(
            Self::config_path()
                .parent()
                .ok_or(anyhow!("no config dir"))?,
        )?;

        let file = fs::File::create(Self::config_path())?;
        serde_json::to_writer(file, self)?;

        Ok(())
    }
}

pub type ConfigPtr = Arc<RwLock<Config>>;
