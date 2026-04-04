use crate::api::{self, ApiError};
use crate::config::Config;
use crate::keychain;
use crate::notify::NotifyManager;
use crate::types::{Credentials, UiUpdate};
use crate::{log_error, log_info, log_warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn interruptible_sleep(duration: Duration, refresh_trigger: &AtomicBool) {
    let deadline = Instant::now() + duration;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() || refresh_trigger.load(Ordering::Relaxed) {
            return;
        }
        thread::sleep(remaining.min(Duration::from_millis(500)));
    }
}

pub fn run(config: &Config, tx: Sender<UiUpdate>, refresh_trigger: Arc<AtomicBool>) {
    let poll_interval = Duration::from_secs(config.poll_interval);
    let mut notifier = NotifyManager::new(
        config.session_notify_threshold,
        config.weekly_notify_threshold,
    );

    let agent = api::create_agent();
    let mut cached_creds: Option<Credentials> = None;
    let mut skip_until: Option<Instant> = None;
    let mut backoff_level: u32 = 0;

    loop {
        if refresh_trigger.load(Ordering::Relaxed) {
            refresh_trigger.store(false, Ordering::Relaxed);
            skip_until = None;
            backoff_level = 0;
            cached_creds = None;
        }

        if let Some(until) = skip_until {
            if Instant::now() < until {
                interruptible_sleep(poll_interval, &refresh_trigger);
                continue;
            }
            skip_until = None;
        }

        // Refresh credentials if missing or expired
        let expired = cached_creds.as_ref().is_none_or(keychain::is_token_expired);
        if expired {
            cached_creds = keychain::read_credentials();
        }

        let creds = match &cached_creds {
            Some(c) if !expired || !keychain::is_token_expired(c) => c,
            Some(_) => {
                cached_creds = None;
                let _ = tx.send(UiUpdate::TokenExpired);
                interruptible_sleep(poll_interval, &refresh_trigger);
                continue;
            }
            None => {
                let _ = tx.send(UiUpdate::NotSignedIn);
                interruptible_sleep(poll_interval, &refresh_trigger);
                continue;
            }
        };

        match api::fetch_usage(&agent, &creds.access_token) {
            Ok(usage) => {
                backoff_level = 0;
                log_info!(
                    "fetched usage: session={}%, weekly={}%",
                    usage.five_hour.as_ref().and_then(|f| f.utilization).unwrap_or(0.0).round(),
                    usage.seven_day.as_ref().and_then(|f| f.utilization).unwrap_or(0.0).round()
                );

                if let Some(ref fh) = usage.five_hour
                    && let Some(pct) = fh.utilization {
                        notifier.check_session(pct);
                    }
                if let Some(ref sd) = usage.seven_day
                    && let Some(pct) = sd.utilization {
                        notifier.check_weekly(pct);
                    }

                let _ = tx.send(UiUpdate::UsageData {
                    usage,
                    plan: creds.plan.clone(),
                });
            }
            Err(ApiError::RateLimit { retry_after }) => {
                backoff_level = backoff_level.saturating_add(1).min(6);
                let backoff_secs =
                    (config.poll_interval * 2u64.pow(backoff_level)).max(retry_after);
                skip_until = Some(Instant::now() + Duration::from_secs(backoff_secs));
                log_warn!("rate limited, retrying in {}s", backoff_secs);
                let _ = tx.send(UiUpdate::RateLimited { retry_in_secs: backoff_secs });
            }
            Err(ApiError::Http(401)) => {
                cached_creds = None;
                log_error!("authentication failed (401)");
                let _ = tx.send(UiUpdate::AuthError);
            }
            Err(ApiError::Http(code)) => {
                log_error!("HTTP error: {}", code);
                let _ = tx.send(UiUpdate::HttpError(code));
            }
            Err(ApiError::Timeout) => {
                log_error!("request timed out");
                let _ = tx.send(UiUpdate::NetworkError("request timed out".into()));
            }
            Err(ApiError::Connection(msg)) => {
                log_error!("connection error: {}", msg);
                let _ = tx.send(UiUpdate::NetworkError(msg));
            }
            Err(ApiError::Parse(msg)) => {
                log_error!("parse error: {}", msg);
                let _ = tx.send(UiUpdate::NetworkError(format!("parse error: {}", msg)));
            }
        }

        interruptible_sleep(poll_interval, &refresh_trigger);
    }
}
