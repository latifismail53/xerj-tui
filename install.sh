#!/bin/sh
# Installer for xerj-tui. No Rust needed.
# Usage: curl -fsSL https://raw.githubusercontent.com/latifismail53/xerj-tui/main/install.sh | sh
set -e

REPO="latifismail53/xerj-tui"
BIN_DIR="${XERJ_TUI_BIN:-$HOME/.local/bin}"

OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS/$ARCH" in
  Darwin/x86_64) TARGET="x86_64-apple-darwin" ;;
  Darwin/arm64)  TARGET="aarch64-apple-darwin" ;;
  Linux/x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
  *)
    echo "xerj-tui: unsupported platform: $OS $ARCH" >&2
    exit 1
    ;;
esac

URL="https://github.com/$REPO/releases/latest/download/xerj-tui-$TARGET"

echo "Downloading xerj-tui ($TARGET)..."
mkdir -p "$BIN_DIR"
curl -fsSL "$URL" -o "$BIN_DIR/xerj-tui"
chmod +x "$BIN_DIR/xerj-tui"

echo ""
echo "Installed to $BIN_DIR/xerj-tui"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    echo "Note: $BIN_DIR is not in your PATH. Add this to your shell profile:"
    echo "  export PATH=\"$BIN_DIR:\$PATH\""
    ;;
esac
echo "Run it with: xerj-tui"
