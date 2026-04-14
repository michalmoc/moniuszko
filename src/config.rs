use crate::constants::APP_NAME;
use dirs::{audio_dir, config_local_dir};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub media_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
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

    pub fn load() -> anyhow::Result<Config> {
        if let Ok(file) = File::open(Self::config_path()) {
            Ok(serde_json::from_reader(file)?)
        } else {
            Ok(Default::default())
        }
    }
}

pub type ConfigPtr = Arc<RwLock<Config>>;
