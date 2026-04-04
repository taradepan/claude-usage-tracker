use std::process::Command;

pub struct NotifyManager {
    session_threshold: f64,
    weekly_threshold: f64,
    has_notified_session: bool,
    has_notified_weekly: bool,
}

impl NotifyManager {
    pub fn new(session_threshold: f64, weekly_threshold: f64) -> Self {
        Self {
            session_threshold,
            weekly_threshold,
            has_notified_session: false,
            has_notified_weekly: false,
        }
    }

    pub fn check_session(&mut self, pct: f64) {
        self.check(pct, self.session_threshold, "Session", |s, v| s.has_notified_session = v, self.has_notified_session);
    }

    pub fn check_weekly(&mut self, pct: f64) {
        self.check(pct, self.weekly_threshold, "Weekly", |s, v| s.has_notified_weekly = v, self.has_notified_weekly);
    }

    fn check(&mut self, pct: f64, threshold: f64, label: &str, set_flag: fn(&mut Self, bool), notified: bool) {
        if pct < threshold {
            set_flag(self, false);
            return;
        }
        if notified {
            return;
        }
        set_flag(self, true);
        send_notification("Claude Code", &format!("{} usage at {}%", label, pct.round() as u64));
    }
}

fn send_notification(title: &str, body: &str) {
    let script = format!(
        r#"display notification "{}" with title "{}""#,
        body.replace('"', r#"\""#),
        title.replace('"', r#"\""#),
    );
    let _ = Command::new("osascript")
        .args(["-e", &script])
        .spawn();
}
