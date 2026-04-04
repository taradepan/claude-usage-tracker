use chrono::{DateTime, Utc};

pub fn format_time_until(iso_str: &str) -> String {
    let target: DateTime<Utc> = match iso_str.parse() {
        Ok(dt) => dt,
        Err(_) => return String::new(),
    };

    let now = Utc::now();
    let secs = (target - now).num_seconds();
    if secs <= 0 {
        return "resetting...".to_string();
    }

    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

pub fn reset_suffix(iso_str: &str) -> String {
    let s = format_time_until(iso_str);
    if s.is_empty() {
        String::new()
    } else {
        format!("  |  resets in {}", s)
    }
}

pub fn usage_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "[{}{}] {}%",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
        pct.round() as u64,
    )
}
