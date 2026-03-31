import getpass
import json
import os
import subprocess
import threading
import webbrowser
from datetime import datetime, timezone

import requests
import rumps

CC_KEYCHAIN_SERVICE = "Claude Code-credentials"
OAUTH_USAGE_URL = "https://api.anthropic.com/api/oauth/usage"
OAUTH_BETA_HEADER = "oauth-2025-04-20"
POLL_INTERVAL = 90
CLAUDE_ICON = os.path.join(os.path.dirname(os.path.abspath(__file__)), "icon.png")

_claude_version_cache = None


def get_claude_code_version():
    global _claude_version_cache
    if _claude_version_cache is not None:
        return _claude_version_cache
    try:
        result = subprocess.run(
            ["claude", "--version"],
            capture_output=True, text=True, timeout=5,
        )
        if result.returncode == 0:
            _claude_version_cache = result.stdout.strip().split()[0]
            return _claude_version_cache
    except Exception:
        pass
    _claude_version_cache = "unknown"
    return _claude_version_cache



def read_oauth_from_keychain():
    for account in [getpass.getuser(), "root"]:
        result = subprocess.run(
            ["security", "find-generic-password",
             "-s", CC_KEYCHAIN_SERVICE, "-a", account, "-w"],
            capture_output=True, text=True,
        )
        if result.returncode != 0 or not result.stdout.strip():
            continue
        try:
            data = json.loads(result.stdout.strip())
            oauth = data.get("claudeAiOauth")
            if not oauth:
                continue
            access_token = (oauth.get("accessToken") or "").strip()
            if not access_token:
                continue
            expires_at_ms = oauth.get("expiresAt", 0) or 0
            expires_at = (
                datetime.fromtimestamp(expires_at_ms / 1000.0, tz=timezone.utc)
                if expires_at_ms else None
            )
            return {
                "access_token": access_token,
                "refresh_token": oauth.get("refreshToken"),
                "expires_at": expires_at,
                "scopes": oauth.get("scopes", []),
                "plan": oauth.get("subscriptionType", ""),
            }
        except (json.JSONDecodeError, KeyError, TypeError):
            continue
    return None


def is_token_expired(creds):
    if not creds or not creds.get("expires_at"):
        return True
    return datetime.now(timezone.utc) >= creds["expires_at"]



def fetch_usage(session, access_token):
    version = get_claude_code_version()
    resp = session.get(
        OAUTH_USAGE_URL,
        headers={
            "Authorization": f"Bearer {access_token}",
            "Accept": "application/json",
            "Content-Type": "application/json",
            "anthropic-beta": OAUTH_BETA_HEADER,
            "User-Agent": f"claude-code/{version}",
        },
        timeout=15,
    )
    resp.raise_for_status()
    return resp.json()



def format_time_until(iso_str):
    if not iso_str:
        return ""
    try:
        target = datetime.fromisoformat(iso_str)
        now = datetime.now(timezone.utc)
        secs = (target - now).total_seconds()
        if secs <= 0:
            return "resetting..."
        days, remainder = divmod(int(secs), 86400)
        hours, remainder = divmod(remainder, 3600)
        minutes = remainder // 60
        if days > 0:
            return f"{days}d {hours}h"
        if hours > 0:
            return f"{hours}h {minutes}m"
        return f"{minutes}m"
    except (ValueError, TypeError):
        return ""


def reset_suffix(iso_str):
    s = format_time_until(iso_str)
    return f"  |  resets in {s}" if s else ""


def usage_bar(pct, width=20):
    filled = round(pct / 100 * width)
    empty = width - filled
    return f"[{'█' * filled}{'░' * empty}] {round(pct)}%"


class ClaudeUsageApp(rumps.App):
    def __init__(self):
        super().__init__("Claude", quit_button=None)

        if os.path.exists(CLAUDE_ICON):
            self.icon = CLAUDE_ICON
            self.template = True
        self.title = " ..."

        def noop(_): pass
        self.session_label = rumps.MenuItem("Session (5h): loading...", callback=noop)
        self.session_bar = rumps.MenuItem("", callback=noop)
        self.weekly_label = rumps.MenuItem("Weekly (7d): loading...", callback=noop)
        self.extra_label = rumps.MenuItem("", callback=noop)
        self.plan_label = rumps.MenuItem("", callback=noop)

        self.menu = [
            self.session_label,
            self.session_bar,
            self.weekly_label,
            rumps.separator,
            self.extra_label,
            self.plan_label,
            rumps.separator,
            rumps.MenuItem("Refresh Now", callback=self.on_refresh),
            rumps.MenuItem("Open Usage Page", callback=self.on_open_usage),
            rumps.separator,
            rumps.MenuItem("Quit", callback=self.on_quit),
        ]

        self._http = requests.Session()
        self._cached_creds = None
        self._fetch_lock = threading.Lock()
        self._skip_until = 0  # epoch time — skip polls until this time (rate limit backoff)

        self.timer = rumps.Timer(self._poll, POLL_INTERVAL)
        self.timer.start()
        threading.Thread(target=self._fetch_and_update, daemon=True).start()

    def _poll(self, _):
        import time
        if time.time() < self._skip_until:
            return
        threading.Thread(target=self._fetch_and_update, daemon=True).start()

    def _set_labels(self, title, session="", bar="", weekly="", extra="", plan=""):
        updates = [
            (self, "title", title),
            (self.session_label, "title", session),
            (self.session_bar, "title", bar),
            (self.weekly_label, "title", weekly),
            (self.extra_label, "title", extra),
            (self.plan_label, "title", plan),
        ]
        for obj, attr, value in updates:
            if getattr(obj, attr) != value:
                setattr(obj, attr, value)

    def _fetch_and_update(self):
        if not self._fetch_lock.acquire(blocking=False):
            return
        try:
            self._do_fetch()
        finally:
            self._fetch_lock.release()

    def _do_fetch(self):
        try:
            if self._cached_creds is None or is_token_expired(self._cached_creds):
                self._cached_creds = read_oauth_from_keychain()

            creds = self._cached_creds
            if not creds:
                self._set_labels(
                    title=" --",
                    session="Not signed in to Claude Code",
                    bar="Run `claude` in terminal to authenticate",
                )
                return

            if is_token_expired(creds):
                self._cached_creds = None
                self._set_labels(
                    title=" exp",
                    session="OAuth token expired",
                    bar="Run `claude` to re-authenticate",
                )
                return

            data = fetch_usage(self._http, creds["access_token"])

            five_hour = data.get("five_hour") or {}
            session_pct = five_hour.get("utilization")
            session_reset = five_hour.get("resets_at", "")

            seven_day = data.get("seven_day") or {}
            weekly_pct = seven_day.get("utilization")
            weekly_reset = seven_day.get("resets_at", "")

            extra = data.get("extra_usage") or {}

            if session_pct is not None:
                pct = round(session_pct)
                reset_time = format_time_until(session_reset)
                title = f" {pct}% | {reset_time}" if reset_time else f" {pct}%"
                session_text = f"Session (5h): {pct}%{reset_suffix(session_reset)}"
                bar_text = f"  {usage_bar(session_pct)}"
            else:
                title = " OK"
                session_text = "Session (5h): no limit"
                bar_text = ""

            if weekly_pct is not None:
                weekly_text = f"Weekly (7d): {round(weekly_pct)}%{reset_suffix(weekly_reset)}"
            else:
                weekly_text = "Weekly (7d): no limit"

            if extra.get("is_enabled") and extra.get("used_credits") is not None:
                used = extra["used_credits"]
                limit = extra.get("monthly_limit")
                extra_text = (
                    f"Extra usage: ${used:.2f} / ${limit:.2f}" if limit
                    else f"Extra usage: ${used:.2f} (no cap)"
                )
            else:
                extra_text = ""

            plan = creds.get("plan", "")
            plan_text = f"Plan: {plan}" if plan else ""

            self._set_labels(
                title=title,
                session=session_text,
                bar=bar_text,
                weekly=weekly_text,
                extra=extra_text,
                plan=plan_text,
            )

        except Exception as e:
            import time
            if hasattr(e, "response") and hasattr(e.response, "status_code"):
                code = e.response.status_code
                if code == 401:
                    self._cached_creds = None
                if code == 429:
                    retry_after = int(e.response.headers.get("Retry-After", 0)) or 300
                    self._skip_until = time.time() + retry_after
                    self.session_bar.title = f"Error: HTTP 429 (retry in {retry_after}s)"
                else:
                    self.session_bar.title = f"Error: HTTP {code}"
            elif isinstance(e, requests.exceptions.ConnectionError):
                self.session_bar.title = "Error: no connection"
            elif isinstance(e, requests.exceptions.Timeout):
                self.session_bar.title = "Error: request timed out"
            else:
                self.session_bar.title = f"Error: {type(e).__name__}"

    def on_refresh(self, _):
        self._skip_until = 0
        self.title = " ..."
        self._cached_creds = None
        threading.Thread(target=self._fetch_and_update, daemon=True).start()

    def on_open_usage(self, _):
        webbrowser.open("https://claude.ai/settings/usage")

    def on_quit(self, _):
        rumps.quit_application()


if __name__ == "__main__":
    ClaudeUsageApp().run()
