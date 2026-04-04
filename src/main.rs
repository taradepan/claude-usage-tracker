mod api;
mod app;
mod config;
mod format;
mod install;
mod keychain;
mod logging;
mod notify;
mod poller;
mod types;

use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--install".to_string()) {
        install::install();
        return;
    }
    if args.contains(&"--uninstall".to_string()) {
        install::uninstall();
        return;
    }

    let log_enabled = args.contains(&"--log".to_string());
    logging::init(log_enabled);

    log_info!("claude-usage starting");

    let config = config::load_config();
    log_info!(
        "config: poll_interval={}s, session_threshold={}%, weekly_threshold={}%",
        config.poll_interval,
        config.session_notify_threshold,
        config.weekly_notify_threshold
    );

    let (tx, rx) = mpsc::channel();
    let refresh_trigger = Arc::new(AtomicBool::new(false));
    let refresh_trigger_poller = refresh_trigger.clone();

    // Spawn poller on background thread
    std::thread::spawn(move || {
        poller::run(&config, tx, refresh_trigger_poller);
    });

    // Run tray app on main thread (macOS requirement)
    app::run(rx, refresh_trigger);
}
