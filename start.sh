#!/bin/bash
# Claude Usage Tracker - Setup & Run
# Usage: ./start.sh [--install | --uninstall | --status]

APP_DIR="$(cd "$(dirname "$0")" && pwd)"
PLIST_NAME="com.claude-usage-tracker.plist"
PLIST_PATH="$HOME/Library/LaunchAgents/$PLIST_NAME"
LOG_DIR="$APP_DIR/logs"
PID_FILE="$APP_DIR/.pid"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

check_deps() {
    if ! command -v uv &>/dev/null; then
        echo -e "${RED}uv not found.${NC} Install: curl -LsSf https://astral.sh/uv/install.sh | sh"
        exit 1
    fi

    # Install dependencies if .venv doesn't exist
    if [ ! -d "$APP_DIR/.venv" ]; then
        echo "Installing dependencies..."
        cd "$APP_DIR" && uv sync --quiet
    fi
}

stop_app() {
    # Kill any running instance
    if [ -f "$PID_FILE" ]; then
        pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null
            echo -e "${YELLOW}Stopped existing instance (PID $pid)${NC}"
        fi
        rm -f "$PID_FILE"
    fi
    pkill -f "python.*claude-usage.*main.py" 2>/dev/null
}

start_app() {
    check_deps
    stop_app

    mkdir -p "$LOG_DIR"

    echo "Starting Claude Usage Tracker..."
    cd "$APP_DIR"
    nohup uv run python main.py \
        >"$LOG_DIR/stdout.log" 2>"$LOG_DIR/stderr.log" &
    echo $! > "$PID_FILE"
    echo -e "${GREEN}Running in background (PID $!)${NC}"
    echo "Logs: $LOG_DIR/"
}

install_launchagent() {
    check_deps
    stop_app

    # Resolve uv path
    UV_PATH="$(command -v uv)"

    mkdir -p "$HOME/Library/LaunchAgents" "$LOG_DIR"

    cat > "$PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>$PLIST_NAME</string>
    <key>ProgramArguments</key>
    <array>
        <string>$UV_PATH</string>
        <string>run</string>
        <string>python</string>
        <string>$APP_DIR/main.py</string>
    </array>
    <key>WorkingDirectory</key>
    <string>$APP_DIR</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>$LOG_DIR/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>$LOG_DIR/stderr.log</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin</string>
    </dict>
</dict>
</plist>
EOF

    launchctl load "$PLIST_PATH"
    echo -e "${GREEN}Installed and started as Launch Agent${NC}"
    echo "  Auto-starts on login"
    echo "  Auto-restarts on crash"
    echo "  Logs: $LOG_DIR/"
}

uninstall_launchagent() {
    if [ -f "$PLIST_PATH" ]; then
        launchctl unload "$PLIST_PATH" 2>/dev/null
        rm -f "$PLIST_PATH"
        echo -e "${GREEN}Launch Agent removed${NC}"
    else
        echo "Launch Agent not installed"
    fi
    stop_app
}

status() {
    echo "Claude Usage Tracker"
    echo "--------------------"

    # Check if running
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        echo -e "Process:      ${GREEN}running (PID $(cat "$PID_FILE"))${NC}"
    elif pgrep -f "python.*claude-usage.*main.py" >/dev/null 2>&1; then
        echo -e "Process:      ${GREEN}running${NC}"
    else
        echo -e "Process:      ${RED}not running${NC}"
    fi

    # Check Launch Agent
    if [ -f "$PLIST_PATH" ]; then
        echo -e "Launch Agent: ${GREEN}installed${NC} (auto-starts on login)"
    else
        echo -e "Launch Agent: ${YELLOW}not installed${NC}"
    fi

    # Check OAuth token (delegates to main.py to match its keychain logic)
    if cd "$APP_DIR" && uv run python -c "from main import read_oauth_from_keychain; import sys; sys.exit(0 if read_oauth_from_keychain() else 1)" 2>/dev/null; then
        echo -e "OAuth token:  ${GREEN}found in Keychain${NC}"
    else
        echo -e "OAuth token:  ${RED}not found${NC} (run 'claude' to authenticate)"
    fi
}

case "${1:-}" in
    --install)
        install_launchagent
        ;;
    --uninstall)
        uninstall_launchagent
        ;;
    --stop)
        stop_app
        ;;
    --status)
        status
        ;;
    --help|-h)
        echo "Usage: ./start.sh [option]"
        echo ""
        echo "Options:"
        echo "  (none)        Start in background"
        echo "  --install     Install as Login Item (auto-start + auto-restart)"
        echo "  --uninstall   Remove Login Item and stop"
        echo "  --stop        Stop running instance"
        echo "  --status      Show current status"
        echo "  --help        Show this help"
        ;;
    *)
        start_app
        ;;
esac
