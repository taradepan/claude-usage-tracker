use crate::config;
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

static LOG_ENABLED: OnceLock<bool> = OnceLock::new();

fn log_path() -> PathBuf {
    config::config_dir().join("usage.log")
}

pub fn init(enabled: bool) {
    LOG_ENABLED.set(enabled).ok();
    if enabled {
        let dir = config::config_dir();
        let _ = fs::create_dir_all(&dir);
    }
}

pub fn log(level: &str, message: &str) {
    if !LOG_ENABLED.get().copied().unwrap_or(false) {
        return;
    }
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let line = format!("[{}] {}: {}\n", timestamp, level, message);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let _ = file.write_all(line.as_bytes());
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::logging::log("INFO", &format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::logging::log("WARN", &format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::logging::log("ERROR", &format!($($arg)*)) };
}
