# Claude Usage Tracker

A lightweight **macOS menubar app** (Rust) that shows your Claude Code session and weekly usage at a glance.

![macOS](https://img.shields.io/badge/macOS-14%2B-blue) ![Rust](https://img.shields.io/badge/rust-1.85%2B-orange) ![License](https://img.shields.io/badge/license-MIT-gray)

## What it does

- Shows your **5-hour session usage %** in the menu bar with the Claude icon
- Dropdown menu includes:
  - session usage + reset countdown
  - weekly usage + reset countdown
  - progress bar
  - extra usage spend (if available)
  - plan type
- **Zero manual auth setup**: reads Claude Code OAuth credentials from macOS Keychain
- Polls usage periodically (configurable), with efficient HTTP usage and credential caching

## How it works

```text
macOS Keychain (service: Claude Code-credentials)
  -> OAuth access token
  -> GET https://api.anthropic.com/api/oauth/usage
  -> menu bar + dropdown UI
```

No API keys, browser cookies, or Cloudflare workarounds required.

## Requirements

- **macOS 14+**
- **Claude Code CLI** installed and authenticated (`claude` available in `PATH`)
- **Rust toolchain** (edition 2024 project; rustup/cargo recommended)

## Quick start

```bash
git clone https://github.com/YOUR_USERNAME/claude-usage.git
cd claude-usage
cargo run --release
```

This launches the tray app in the foreground.

## CLI options

Run the binary directly (or via `cargo run -- ...`):

```bash
# install LaunchAgent (start on login, restart on crash)
cargo run --release -- --install

# remove LaunchAgent
cargo run --release -- --uninstall

# run app with file logging enabled
cargo run --release -- --log
```

## Build

```bash
cargo build --release
```

Binary output will be in:

```text
target/release/claude-usage
```

## Auto-start on login (LaunchAgent)

The app can install/uninstall a user LaunchAgent from the CLI:

- `--install`: writes and loads LaunchAgent plist
- `--uninstall`: unloads and removes it

This is the preferred way to run continuously in the background on macOS.

## Configuration

The app loads configuration at startup (poll interval, notification thresholds, etc.).  
If no config is present, sane defaults are used.

Typical configurable areas include:

- poll interval (seconds)
- session usage notification threshold (%)
- weekly usage notification threshold (%)

## Logging

Use `--log` to enable file logging for easier debugging and monitoring.

## Authentication details

Claude Code stores OAuth credentials in macOS Keychain under:

- service: `Claude Code-credentials`

If authentication expires, re-authenticate by running:

```bash
claude
```

Then restart (or let the app refresh on next poll).

## Project structure

```text
claude-usage/
├── Cargo.toml
├── README.md
├── icon.png
├── src/
│   ├── main.rs        # entrypoint + CLI arg handling
│   ├── app.rs         # tray app + menu handling
│   ├── poller.rs      # periodic usage polling loop
│   ├── api.rs         # OAuth usage API client
│   ├── keychain.rs    # macOS keychain token retrieval
│   ├── config.rs      # config loading/defaults
│   ├── notify.rs      # user notifications
│   ├── logging.rs     # logging setup
│   ├── install.rs     # LaunchAgent install/uninstall
│   ├── format.rs      # display formatting helpers
│   └── types.rs       # shared data models
├── logs/
└── start.sh           # legacy helper script (older Python flow)
```

## Notes

- The current implementation is Rust-based.
- If you still have old Python-oriented commands in local scripts/docs, prefer the Rust CLI commands above.

## License

MIT