use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub poll_interval: u64,
    pub session_notify_threshold: f64,
    pub weekly_notify_threshold: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            poll_interval: 120,
            session_notify_threshold: 80.0,
            weekly_notify_threshold: 80.0,
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("could not find home directory")
        .join("Library/Application Support/claude-usage")
}

pub fn load_config() -> Config {
    let path = config_dir().join("config.toml");
    match fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str::<Config>(&contents) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("warning: invalid config file {}: {}", path.display(), e);
                Config::default()
            }
        },
        Err(_) => Config::default(),
    }
}
