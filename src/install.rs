use std::fs;
use std::path::PathBuf;
use std::process::Command;

const PLIST_LABEL: &str = "com.claude-usage";

fn plist_path() -> PathBuf {
    dirs::home_dir()
        .expect("could not find home directory")
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", PLIST_LABEL))
}

pub fn install() {
    let binary = std::env::current_exe().expect("could not determine binary path");
    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#,
        label = PLIST_LABEL,
        binary = binary.display(),
    );

    let path = plist_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, plist_content).expect("failed to write plist");
    println!("Installed launch agent: {}", path.display());
    println!("Claude Usage will start automatically on login.");
}

pub fn uninstall() {
    let path = plist_path();
    if path.exists() {
        let _ = Command::new("launchctl")
            .args(["unload", &path.to_string_lossy()])
            .status();
        fs::remove_file(&path).expect("failed to remove plist");
        println!("Removed launch agent: {}", path.display());
    } else {
        println!("No launch agent found at: {}", path.display());
    }
}
