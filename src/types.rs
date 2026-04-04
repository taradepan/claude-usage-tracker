use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Credentials {
    pub access_token: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub plan: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageResponse {
    pub five_hour: Option<UsageBucket>,
    pub seven_day: Option<UsageBucket>,
    pub extra_usage: Option<ExtraUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageBucket {
    pub utilization: Option<f64>,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtraUsage {
    pub is_enabled: bool,
    pub used_credits: Option<f64>,
    pub monthly_limit: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum UiUpdate {
    UsageData { usage: UsageResponse, plan: String },
    NotSignedIn,
    TokenExpired,
    RateLimited { retry_in_secs: u64 },
    AuthError,
    NetworkError(String),
    HttpError(u16),
}
