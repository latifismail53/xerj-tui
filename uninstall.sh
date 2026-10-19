#!/bin/sh
# Uninstaller for xerj-tui.
# Usage: curl -fsSL https://raw.githubusercontent.com/latifismail53/xerj-tui/main/uninstall.sh | sh
set -e

: "${HOME:?HOME is not set}"
BIN_DIR="${XERJ_TUI_BIN:-$HOME/.local/bin}"
STATE_DIR="$HOME/.xerj-tui"

# 1. Stop the XERJ server if the TUI started it.
PID_FILE="$STATE_DIR/server.pid"
if [ -f "$PID_FILE" ]; then
  PID="$(cat "$PID_FILE" 2>/dev/null || true)"
  case "$PID" in
    ''|*[!0-9]*) ;;
    *)
      if kill -0 "$PID" 2>/dev/null; then
        echo "Stopping XERJ server (PID $PID)..."
        kill "$PID" 2>/dev/null || true
      fi
      ;;
  esac
fi

# 2. Remove the binary.
if [ -f "$BIN_DIR/xerj-tui" ]; then
  rm -f "$BIN_DIR/xerj-tui"
  echo "Removed $BIN_DIR/xerj-tui"
else
  echo "Binary not found at $BIN_DIR/xerj-tui (skipped)"
fi

# 3. Remove TUI-owned state (PID file, state.json, logs).
if [ -d "$STATE_DIR" ]; then
  rm -rf "$STATE_DIR"
  echo "Removed $STATE_DIR"
fi

echo ""
echo "xerj-tui is uninstalled."
echo "Note: XERJ itself and your data in ~/xerj-data were left untouched."
