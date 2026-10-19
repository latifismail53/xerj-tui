# xerj-tui

> **Disclaimer:** xerj-tui is an **unofficial** community tool, not affiliated with or endorsed by [xerj-org](https://github.com/xerj-org/xerj). XERJ is licensed under [Apache-2.0](https://github.com/xerj-org/xerj/blob/main/LICENSE).

A terminal menu for using [XERJ](https://github.com/xerj-org/xerj) (a local search engine for AI coding agents) without memorizing commands.

```
 xerj-tui, a friendly companion for XERJ
────────────────────────────────────────────────────────────────────────────────
┌ XERJ Status ─────────────────────────────────────────────────────────────────┐
│ version v1.0.0-rc.96      status ○ STOPPED                                   │
│  api     http://localhost:9200                                               │
│  console http://localhost:9200/_xerj-console/                                │
│  api key unknown                                                             │
│  data    -                                                                   │
│ corpora -                                                                    │
└──────────────────────────────────────────────────────────────────────────────┘
┌ Menu ────────────────────────────────────────────────────────────────────────┐
│  1  Install XERJ   [v1.0.0-rc.96 installed]                                  │
│› 2  Start server                                                             │
│  3  Index project   (needs XERJ + running server)                            │
│  4  Init project for AI agent                                                │
│  5  Search code   (needs XERJ + running server)                              │
│  6  Go-to-definition   (needs XERJ + running server)                         │
│  7  Statistics (xerj gain)   (needs XERJ + running server)                   │
│  8  Auto-start at login   (macOS only)                                       │
│  0  Send field report to the XERJ community                                  │
│  9  Quit                                                                     │
└──────────────────────────────────────────────────────────────────────────────┘
────────────────────────────────────────────────────────────────────────────────
           ↑↓ select · Enter run · 0-9 shortcuts · r refresh · q quit
```

## Install

No Rust needed. One line:

```bash
curl -fsSL https://raw.githubusercontent.com/latifismail53/xerj-tui/main/install.sh | sh
```

This downloads the prebuilt binary for your platform (macOS Intel, macOS Apple Silicon, or Linux x86_64) into `~/.local/bin`. Make sure `~/.local/bin` is in your `PATH`.

Or build it yourself (requires [Rust](https://sh.rustup.rs)):

```bash
cargo build --release
./target/release/xerj-tui   # single ~3 MB binary, no runtime dependencies
```

Requires macOS or Linux, and a terminal at least 80x24.

## Uninstall

```bash
curl -fsSL https://raw.githubusercontent.com/latifismail53/xerj-tui/main/uninstall.sh | sh
```

This stops the server (if the TUI started it) and removes the `xerj-tui` binary plus `~/.xerj-tui/`. XERJ itself and your data in `~/xerj-data` are left untouched.

## Usage

1. **Install XERJ** (menu 1) — runs the official installer.
2. **Start server** (menu 2) — serves at `http://localhost:9200`.
3. **Index project** (menu 3) — paste the folder path, press Enter.
4. **Init project** (menu 4) — writes `.mcp.json` + skill files into the project folder so Claude Code/Cursor can use XERJ.

## Features

- Automatic detection: if `xerj` is already installed, the install menu is disabled and the version is shown.
- Status card: version, server/console URLs, API key location, data dir, and indexed corpora in one view.
- One-click server start/stop, plus start-on-login (macOS).
- Code search, go-to-definition, and usage statistics from the menu.
- Every mutating action asks for confirmation first.
- Optional: send a field report to the XERJ community.

## Code layout

```
src/
├── main.rs   # entry point
├── core.rs   # logic: detection, parsing, server management, xerj commands
├── app.rs    # state machine + confirmation screens
└── ui.rs     # ratatui rendering
```

## Security

- Never modifies the XERJ binary, data, or keys. No delete/reset menu exists.
- TUI-owned files live only in `~/.xerj-tui/` (server PID, state, logs).
- The server runs with `--insecure` (no TLS/auth), bound to `127.0.0.1` only, as recommended by XERJ for single-user laptop use.
- Default data dir is `~/xerj-data`, always outside indexed folders.

## FAQ

**Is my data sent anywhere?**
No. XERJ runs 100% locally.

**The server won't start?**
Make sure port 9200 is free. Check the log at `~/.xerj-tui/xerj-server.log`.

**How do I update XERJ?**
Re-run the Install menu.

## License

MIT — see [LICENSE](LICENSE). This repo is not a fork of XERJ and does not bundle the XERJ binary; installation always uses the official installer.
