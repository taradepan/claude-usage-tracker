use crate::types::UsageResponse;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;
use ureq::Agent;

const OAUTH_USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const OAUTH_BETA_HEADER: &str = "oauth-2025-04-20";

static CLAUDE_VERSION: OnceLock<String> = OnceLock::new();

fn get_claude_version() -> &'static str {
    CLAUDE_VERSION.get_or_init(|| {
        Command::new("claude")
            .arg("--version")
            .output()
            .ok()
            .and_then(|out| {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    stdout.split_whitespace().next().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown".to_string())
    })
}

#[derive(Debug)]
pub enum ApiError {
    Http(u16),
    RateLimit { retry_after: u64 },
    Connection(String),
    Timeout,
    Parse(String),
}

pub fn fetch_usage(agent: &Agent, access_token: &str) -> Result<UsageResponse, ApiError> {
    let response = agent
        .get(OAUTH_USAGE_URL)
        .header("Authorization", &format!("Bearer {}", access_token))
        .header("Accept", "application/json")
        .header("anthropic-beta", OAUTH_BETA_HEADER)
        .header("User-Agent", &format!("claude-code/{}", get_claude_version()))
        .call();

    match response {
        Ok(mut resp) => {
            let body: UsageResponse = resp
                .body_mut()
                .read_json()
                .map_err(|e| ApiError::Parse(e.to_string()))?;
            Ok(body)
        }
        Err(ureq::Error::StatusCode(429)) => {
            Err(ApiError::RateLimit { retry_after: 0 })
        }
        Err(ureq::Error::StatusCode(code)) => Err(ApiError::Http(code)),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("timed out") || msg.contains("Timeout") {
                Err(ApiError::Timeout)
            } else {
                Err(ApiError::Connection(msg))
            }
        }
    }
}

pub fn create_agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into()
}
