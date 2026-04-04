use crate::types::Credentials;
use chrono::{TimeZone, Utc};
use std::env;
use std::process::Command;

const KEYCHAIN_SERVICE: &str = "Claude Code-credentials";

pub fn read_credentials() -> Option<Credentials> {
    let accounts = [
        env::var("USER").unwrap_or_default(),
        "root".to_string(),
    ];

    for account in &accounts {
        if account.is_empty() {
            continue;
        }
        if let Some(creds) = try_read_account(account) {
            return Some(creds);
        }
    }
    None
}

fn try_read_account(account: &str) -> Option<Credentials> {
    // Use the `security` CLI like the Python version — avoids Keychain ACL prompts
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s", KEYCHAIN_SERVICE,
            "-a", account,
            "-w",
        ])
        .output()
        .ok()?;

    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let data: serde_json::Value = serde_json::from_str(&raw).ok()?;

    let oauth = data.get("claudeAiOauth")?;

    let access_token = oauth.get("accessToken")?.as_str()?.trim().to_string();
    if access_token.is_empty() {
        return None;
    }

    let expires_at = oauth
        .get("expiresAt")
        .and_then(|v| v.as_i64())
        .filter(|&ms| ms > 0)
        .and_then(|ms| Utc.timestamp_millis_opt(ms).single());

    let plan = oauth
        .get("subscriptionType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(Credentials {
        access_token,
        expires_at,
        plan,
    })
}

pub fn is_token_expired(creds: &Credentials) -> bool {
    match creds.expires_at {
        Some(exp) => Utc::now() >= exp,
        None => true,
    }
}
