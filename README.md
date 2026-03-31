# Claude Usage Tracker

A lightweight macOS menubar app that shows your Claude Code session and weekly usage at a glance.

![macOS](https://img.shields.io/badge/macOS-14%2B-blue) ![Python](https://img.shields.io/badge/python-3.11%2B-green) ![License](https://img.shields.io/badge/license-MIT-gray)

## What it does

- Shows your **5-hour session usage %** right in the menubar with the Claude icon
- Dropdown shows session progress bar, weekly usage, reset countdown, extra usage spend, and plan type
- **Zero config** — reads OAuth tokens directly from Claude Code's macOS Keychain
- Polls every 60 seconds, uses persistent HTTP connections, caches credentials until they expire

## How it works

```
macOS Keychain (Claude Code-credentials)
    → OAuth access token
        → GET api.anthropic.com/api/oauth/usage
            → menubar display
```

No browser cookies, no Cloudflare issues, no API keys needed.

## Prerequisites

- **macOS 14+**
- **Claude Code** installed and signed in (`claude` CLI available in PATH)
- **[uv](https://docs.astral.sh/uv/)** package manager

## Quick start

```bash
git clone https://github.com/YOUR_USERNAME/claude-usage.git
cd claude-usage

# Run in background
./start.sh

# Or run in foreground (see logs directly)
uv run python main.py
```

## Auto-start on login

```bash
# Install as a macOS Launch Agent (auto-starts on login, auto-restarts on crash)
./start.sh --install

# Remove it
./start.sh --uninstall
```

## Commands

```
./start.sh              Start in background
./start.sh --install    Install as Login Item (auto-start + auto-restart)
./start.sh --uninstall  Remove Login Item and stop
./start.sh --stop       Stop running instance
./start.sh --status     Show current status
./start.sh --help       Show help
```

## Menubar display

| Menubar | Meaning |
|---------|---------|
| `[icon] 42%` | Session is at 42% of 5-hour limit |
| `[icon] OK`  | No session limit active |
| `[icon] --`  | Not signed in or no internet |
| `[icon] exp` | OAuth token expired — run `claude` to re-auth |
| `[icon] !`   | API returned 401 |

The dropdown menu shows:
- **Session (5h):** percentage with reset countdown
- **Progress bar:** visual representation of session usage
- **Weekly (7d):** percentage with reset countdown
- **Extra usage:** dollar amount spent (if enabled on your plan)
- **Plan:** your subscription type

## How authentication works

Claude Code stores OAuth credentials in the macOS Keychain under the service name `Claude Code-credentials`. This app reads that token using `/usr/bin/security`.

No API keys, session cookies, or manual configuration required. If your token expires, just run `claude` in your terminal to re-authenticate.

## Project structure

```
claude-usage/
├── main.py        # Menubar app (rumps + requests)
├── start.sh       # Setup, run, and Launch Agent management
├── icon.png       # Claude tray icon
├── pyproject.toml # Dependencies (requests, rumps)
└── uv.lock        # Lock file
```

## License

MIT
