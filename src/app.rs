//! State machine TUI: screen, menu, input, background task.

use std::io::BufRead;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use crate::core;

// Model

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    IndexPath,
    InitPath,
    SearchQuery,
    DefSymbol,
}

impl PromptKind {
    pub fn title(&self) -> &'static str {
        match self {
            PromptKind::IndexPath => "Index project",
            PromptKind::InitPath => "Init project for AI agent",
            PromptKind::SearchQuery => "Search code",
            PromptKind::DefSymbol => "Go-to-definition",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            PromptKind::IndexPath => "Paste project folder path:",
            PromptKind::InitPath => "Paste project folder path:",
            PromptKind::SearchQuery => "Query pencarian:",
            PromptKind::DefSymbol => "Nama fungsi/simbol:",
        }
    }
    pub fn hint(&self) -> &'static str {
        match self {
            PromptKind::IndexPath => "e.g. ~/Development/erpv2   (Enter: run, Esc: cancel)",
            PromptKind::InitPath => "e.g. ~/Development/erpv2   (Enter: run, Esc: cancel)",
            PromptKind::SearchQuery => "e.g. calculate_total function   (Enter: search, Esc: cancel)",
            PromptKind::DefSymbol => "e.g. calculate_total   (Enter: search, Esc: cancel)",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Install,
    ToggleServer,
    Index,
    Init,
    Search,
    Def,
    Gain,
    Feedback,
    Autostart,
    Quit,
}

/// Aksi yang mengubah sesuatu di luar TUI WAJIB lewat layar konfirmasi
/// dulu (prinsip keamanan: tidak ada aksi destruktif diam-diam).
/// Setiap varian wajib menjelaskan "yang akan dilakukan" di summary_lines().
#[derive(Clone)]
pub enum PendingAction {
    Install,
    StartServer,
    StopServer(Option<u32>),
    Autoindex(String),
    InitProject(String),
    Autostart(bool),
    Feedback,
}

impl PendingAction {
    pub fn title(&self) -> &'static str {
        match self {
            PendingAction::Install => "Install XERJ",
            PendingAction::StartServer => "Start server",
            PendingAction::StopServer(_) => "Stop server",
            PendingAction::Autoindex(_) => "Index project",
            PendingAction::InitProject(_) => "Init project for AI agent",
            PendingAction::Autostart(true) => "Aktifkan auto-start",
            PendingAction::Autostart(false) => "Disable auto-start",
            PendingAction::Feedback => "Send field report to the XERJ community",
        }
    }

    /// Ringkasan jujur "yang akan dilakukan", ditampilkan sebelum eksekusi.
    pub fn summary_lines(&self) -> Vec<String> {
        match self {
            PendingAction::Install => vec![
                "What will happen:",
                "",
                "· Runs the OFFICIAL XERJ installer:",
                "    curl -fsSL https://xerj.org/get | sh",
                "",
                "· Downloads a single `xerj` binary to ~/.local/bin.",
                "· Does not touch any project files.",
                "· Does not patch or modify the binary or anything else.",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            PendingAction::StartServer => vec![
                "What will happen:",
                "",
                "· Runs: xerj --insecure --data-dir ~/xerj-data",
                "  as a background process.",
                "",
                "· --insecure mode = NO TLS and NO API key.",
                "  Safe only for a single user on their own laptop",
                "  (server binds 127.0.0.1 only, per XERJ's security model).",
                "",
                "· PID is recorded in ~/.xerj-tui/server.pid (TUI-owned file).",
                "· XERJ's internal files are not touched.",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            PendingAction::StopServer(pid) => vec![
                "What will happen:",
                "",
                &format!(
                    "· Stops the XERJ server process{}.",
                    pid.map(|p| format!(" (PID {})", p)).unwrap_or_default()
                ),
                "· Index and data in ~/xerj-data REMAIN INTACT.",
                "  Only the process is stopped.",
                "· No XERJ files are deleted.",
            ]
            .into_iter()
            .map(|s: &str| s.to_string())
            .collect(),
            PendingAction::Autoindex(path) => vec![
                "What will happen:",
                "",
                &format!("· Runs: xerj autoindex {}", path),
                "· Project files are only READ. Nothing is modified.",
                "· Index is written to ~/xerj-data (TUI-owned data dir).",
                "· Existing XERJ data/indexes are NOT deleted.",
            ]
            .into_iter()
            .map(|s: &str| s.to_string())
            .collect(),
            PendingAction::InitProject(path) => vec![
                "What will happen:",
                "",
                &format!("· Runs: xerj init in {}", path),
                "· Will write/merge: .mcp.json and AGENTS.md",
                "  in that project folder.",
                "· An existing .mcp.json is backed up as",
                "  .mcp.json.bak (official xerj behavior, verified).",
                "· No other files are touched.",
            ]
            .into_iter()
            .map(|s: &str| s.to_string())
            .collect(),
            PendingAction::Autostart(true) => vec![
                "What will happen:",
                "",
                "· Writes ~/Library/LaunchAgents/org.xerj.tui.plist",
                "  (TUI-owned file).",
                "· Runs: launchctl load. The XERJ server will",
                "  start automatically at each login (macOS).",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            PendingAction::Autostart(false) => vec![
                "What will happen:",
                "",
                "· Runs: launchctl unload.",
                "· Deletes the TUI-owned plist file.",
                "· The server will no longer start automatically.",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            // Honest: this is a community request from XERJ (community entry ticket),
            // NOT a legal obligation. The --dry-run mode sends nothing.
            PendingAction::Feedback => vec![
                "What will happen:",
                "",
                "· Runs: xerj feedback --dry-run",
                "  (drafts the report on screen, does NOT send it).",
                "",
                "· This is a community request from XERJ (community entry ticket),",
                "  NOT a legal obligation. Feel free to skip it.",
                "",
                "· To actually send it, run manually in a terminal:",
                "    xerj feedback --open-pr --verdict \"...\" --used-for \"...\"",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }
}

pub struct MenuItem {
    pub key: char,
    pub label: String,
    pub enabled: bool,
    pub action: Action,
}

pub enum Screen {
    Menu,
    Prompt {
        kind: PromptKind,
        input: String,
        cursor: usize,
    },
    /// Konfirmasi sebelum aksi yang mengubah sesuatu di luar TUI.
    Confirm {
        action: PendingAction,
    },
    /// Perintah berjalan; output di-streaming baris per baris.
    Busy {
        title: String,
        lines: Vec<String>,
    },
    /// Output selesai, bisa di-scroll.
    Output {
        title: String,
        lines: Vec<String>,
        scroll: usize,
    },
    /// Pesan singkat (sukses/gagal).
    Message {
        title: String,
        lines: Vec<String>,
    },
}

pub enum TaskEvent {
    Line(String),
    Done(Result<String, String>),
}

pub struct App {
    pub screen: Screen,
    pub selected: usize,
    pub menu_offset: usize,
    pub items: Vec<MenuItem>,
    pub card: core::StatusCard,
    pub menu_status: core::MenuStatus,
    task_rx: Option<Receiver<TaskEvent>>,
    pending_title: String,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = App {
            screen: Screen::Menu,
            selected: 0,
            menu_offset: 0,
            items: Vec::new(),
            card: core::status_card(),
            menu_status: core::menu_status(),
            task_rx: None,
            pending_title: String::new(),
        };
        app.refresh();
        app
    }

    /// Baca ulang status (install, server, kartu). Dipanggil tiap kembali ke menu.
    pub fn refresh(&mut self) {
        // Adopsi: server sehat tapi tanpa catatan PID (dijalankan di luar
        // TUI) -> coba temukan PID-nya agar menu Stop tetap berfungsi.
        let st0 = core::server_status();
        if st0.running() && !st0.pid.map(core::pid_alive).unwrap_or(false) {
            let _ = core::ensure_pid_recorded(&core::pid_file(), core::HEALTH_URL);
        }
        self.card = core::status_card();
        self.menu_status = core::menu_status();
        self.items = build_menu(&self.menu_status);
        if self.selected >= self.items.len() {
            self.selected = 0;
        }
        // Pastikan seleksi tidak mendarat di item disabled.
        if !self.items.is_empty() && !self.items[self.selected].enabled {
            self.move_selection(1);
        }
    }

    fn move_selection(&mut self, dir: i32) {
        let n = self.items.len();
        if n == 0 {
            return;
        }
        let mut i = self.selected;
        for _ in 0..n {
            i = (i as i32 + dir).rem_euclid(n as i32) as usize;
            if self.items[i].enabled {
                self.selected = i;
                return;
            }
        }
    }

    /// true = minta keluar aplikasi.
    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> bool {
        match &mut self.screen {
            Screen::Menu => self.handle_menu_key(code, mods),
            Screen::Prompt { kind, input, cursor } => {
                let kind = *kind;
                match prompt_key(input, cursor, code, mods) {
                    PromptResult::Cancel => {
                        self.screen = Screen::Menu;
                    }
                    PromptResult::Submit => {
                        let value = input.clone();
                        self.submit_prompt(kind, &value);
                    }
                    PromptResult::Keep => {}
                }
                false
            }
            Screen::Busy { .. } => false, // tunggu selesai; jangan diinterupsi
            Screen::Confirm { action } => {
                let action = action.clone();
                match code {
                    KeyCode::Enter => {
                        self.execute_confirmed(action);
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.screen = Screen::Menu;
                        self.refresh();
                    }
                    _ => {}
                }
                false
            }
            Screen::Output { scroll, lines, .. } => {
                let max = lines.len().saturating_sub(1);
                match code {
                    KeyCode::Up => *scroll = scroll.saturating_sub(1),
                    KeyCode::Down => *scroll = (*scroll + 1).min(max),
                    KeyCode::PageUp => *scroll = scroll.saturating_sub(10),
                    KeyCode::PageDown => *scroll = (*scroll + 10).min(max),
                    KeyCode::Home => *scroll = 0,
                    KeyCode::End => *scroll = max,
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                        self.screen = Screen::Menu;
                        self.refresh();
                    }
                    _ => {}
                }
                false
            }
            Screen::Message { .. } => {
                self.screen = Screen::Menu;
                self.refresh();
                false
            }
        }
    }

    fn handle_menu_key(&mut self, code: KeyCode, _mods: KeyModifiers) -> bool {
        match code {
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::Enter => {
                if self.items[self.selected].enabled {
                    return self.activate(self.items[self.selected].action);
                }
            }
            KeyCode::Char('q') => return true,
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if let Some(item) = self.items.iter().find(|i| i.key == c) {
                    if item.enabled {
                        return self.activate(item.action);
                    }
                }
            }
            KeyCode::Char('r') => self.refresh(),
            _ => {}
        }
        false
    }

    /// true = keluar.
    /// Aksi yang mengubah sesuatu di luar TUI selalu lewat layar
    /// konfirmasi dulu (PendingAction). Aksi read-only jalan langsung.
    fn activate(&mut self, action: Action) -> bool {
        match action {
            Action::Quit => return true,
            Action::Install => self.confirm(PendingAction::Install),
            Action::ToggleServer => {
                if self.menu_status.server_running {
                    self.confirm(PendingAction::StopServer(self.menu_status.pid));
                } else {
                    self.confirm(PendingAction::StartServer);
                }
            }
            Action::Index => self.open_prompt(PromptKind::IndexPath),
            Action::Init => self.open_prompt(PromptKind::InitPath),
            Action::Search => self.open_prompt(PromptKind::SearchQuery),
            Action::Def => self.open_prompt(PromptKind::DefSymbol),
            Action::Gain => self.start_background("Statistik (xerj gain)", TaskKind::Gain),
            Action::Feedback => self.confirm(PendingAction::Feedback),
            Action::Autostart => {
                self.confirm(PendingAction::Autostart(!self.menu_status.autostart));
            }
        }
        false
    }

    fn confirm(&mut self, action: PendingAction) {
        self.screen = Screen::Confirm { action };
    }

    /// Eksekusi aksi yang sudah dikonfirmasi user.
    fn execute_confirmed(&mut self, action: PendingAction) {
        match action {
            PendingAction::Install => self.start_background("Install XERJ", TaskKind::Install),
            PendingAction::StartServer => {
                self.start_background("Start server", TaskKind::StartServer)
            }
            PendingAction::StopServer(_) => {
                match core::stop_server() {
                    Ok(msg) => self.show_message("Stop server", &msg),
                    Err(e) => self.show_message("Failed", &e),
                }
                self.refresh();
            }
            PendingAction::Autoindex(path) => {
                self.start_background("Index project", TaskKind::Autoindex(path))
            }
            PendingAction::InitProject(path) => {
                self.start_background("Init project", TaskKind::Init(path))
            }
            PendingAction::Autostart(enable) => {
                match core::set_autostart(enable) {
                    Ok(msg) => self.show_message("Auto-start", &msg),
                    Err(e) => self.show_message("Auto-start", &e),
                }
                self.refresh();
            }
            PendingAction::Feedback => {
                self.start_background("Field report (draft)", TaskKind::Feedback)
            }
        }
    }

    fn open_prompt(&mut self, kind: PromptKind) {
        self.screen = Screen::Prompt {
            kind,
            input: String::new(),
            cursor: 0,
        };
    }

    fn submit_prompt(&mut self, kind: PromptKind, value: &str) {
        match kind {
            // Index & init mengubah sesuatu -> wajib konfirmasi dulu.
            PromptKind::IndexPath => {
                self.confirm(PendingAction::Autoindex(value.to_string()));
            }
            PromptKind::InitPath => {
                self.confirm(PendingAction::InitProject(value.to_string()));
            }
            // Search & def read-only -> langsung jalan.
            PromptKind::SearchQuery => {
                let v = value.to_string();
                self.start_background("Search code", TaskKind::Search(v));
            }
            PromptKind::DefSymbol => {
                let v = value.to_string();
                self.start_background("Go-to-definition", TaskKind::Def(v));
            }
        }
    }

    fn show_message(&mut self, title: &str, text: &str) {
        self.screen = Screen::Message {
            title: title.into(),
            lines: text.lines().map(|l| l.to_string()).collect(),
        };
    }

    /// Jalankan task di thread background, streaming output ke layar Busy.
    fn start_background(&mut self, title: &str, kind: TaskKind) {
        let (tx, rx) = mpsc::channel();
        self.task_rx = Some(rx);
        self.pending_title = title.to_string();
        self.screen = Screen::Busy {
            title: title.to_string(),
            lines: vec!["menyiapkan...".into()],
        };
        std::thread::spawn(move || {
            run_task(kind, &tx);
        });
    }

    /// Poll hasil dari background thread. Dipanggil tiap tick.
    pub fn poll_task(&mut self) {
        let mut done: Option<Result<String, String>> = None;
        if let Some(rx) = &self.task_rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    TaskEvent::Line(line) => {
                        if let Screen::Busy { lines, .. } = &mut self.screen {
                            // buang baris "menyiapkan..." placeholder
                            if lines.len() == 1 && lines[0] == "menyiapkan..." {
                                lines.clear();
                            }
                            lines.push(line);
                            if lines.len() > 500 {
                                lines.drain(..lines.len() - 500);
                            }
                        }
                    }
                    TaskEvent::Done(res) => {
                        done = Some(res);
                        break;
                    }
                }
            }
        }
        if let Some(res) = done {
            self.task_rx = None;
            let title = std::mem::take(&mut self.pending_title);
            match res {
                Ok(out) => {
                    self.screen = Screen::Output {
                        title,
                        lines: out.lines().map(|l| l.to_string()).collect(),
                        scroll: 0,
                    };
                }
                Err(e) => {
                    self.screen = Screen::Message {
                        title: "Failed".into(),
                        lines: e.lines().map(|l| l.to_string()).collect(),
                    };
                }
            }
        }
    }
}

// Background task

#[derive(Clone)]
enum TaskKind {
    Install,
    StartServer,
    Autoindex(String),
    Init(String),
    Search(String),
    Def(String),
    Gain,
    Feedback,
}

fn run_task(kind: TaskKind, tx: &mpsc::Sender<TaskEvent>) {
    let send = |s: String| {
        let _ = tx.send(TaskEvent::Line(s));
    };
    let done_ok = |s: String| {
        let _ = tx.send(TaskEvent::Done(Ok(s)));
    };
    let done_err = |s: String| {
        let _ = tx.send(TaskEvent::Done(Err(s)));
    };

    match kind {
        TaskKind::Install => {
            // Installer resmi via sh; streaming output apa adanya.
            let cmd = core::install_cmd();
            match std::process::Command::new("sh")
                .args(["-c", &cmd])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    let mut out = child.stdout.take();
                    let mut err = child.stderr.take();
                    let tx2 = tx.clone();
                    let t = std::thread::spawn(move || {
                        let Some(out) = out.take() else { return };
                        let mut buf = std::io::BufReader::new(out);
                        let mut line = String::new();
                        while let Ok(n) = buf.read_line(&mut line) {
                            if n == 0 {
                                break;
                            }
                            let _ = tx2.send(TaskEvent::Line(line.trim_end().to_string()));
                            line.clear();
                        }
                    });
                    let Some(err) = err.take() else {
                        let _ = t.join();
                        done_err("Failed to read installer output.".to_string());
                        return;
                    };
                    let mut ebuf = std::io::BufReader::new(err);
                    let mut line = String::new();
                    while let Ok(n) = ebuf.read_line(&mut line) {
                        if n == 0 {
                            break;
                        }
                        send(line.trim_end().to_string());
                        line.clear();
                    }
                    let _ = t.join();
                    match child.wait() {
                        Ok(st) if st.success() => {
                            let v = core::xerj_version().unwrap_or_else(|| "?".into());
                            done_ok(format!(
                                "Install complete!\n\nXERJ v{} is installed.\n\nNext step: 'Start server' from the menu.",
                                v
                            ));
                        }
                        Ok(st) => done_err(format!(
                            "Installer failed (code {}).\nCheck your internet connection and try again.",
                            st.code().unwrap_or(-1)
                        )),
                        Err(e) => done_err(format!("Failed while waiting for the installer: {}", e)),
                    }
                }
                Err(e) => done_err(format!("Failed to run the installer: {}", e)),
            }
        }
        TaskKind::StartServer => {
            send("running xerj --insecure --data-dir ~/xerj-data ...".into());
            match core::start_server(&core::default_data_dir()) {
                Ok(pid) => done_ok(format!(
                    "XERJ server running (PID {}).\n\nAPI     : {}\nConsole : {}\n\nNote: --insecure mode means no TLS and no API key.\nOnly for use on your own laptop.",
                    pid,
                    core::SERVER_URL,
                    core::CONSOLE_URL
                )),
                Err(e) => done_err(e),
            }
        }
        TaskKind::Autoindex(path) => {
            // Streaming via spawn agar user bisa melihat progres.
            let args = vec!["autoindex".to_string(), path.clone()];
            stream_xerj(&args, None, tx, |out| {
                // Validasi data-dir dilakukan di core; di sini cukup teruskan.
                let _ = out;
                Ok("Indexing complete.".to_string())
            });
        }
        TaskKind::Init(path) => {
            let cwd = match core::validate_folder(&path) {
                Ok(p) => p,
                Err(e) => {
                    done_err(e);
                    return;
                }
            };
            stream_xerj(&["init".to_string()], Some(&cwd), tx, |_| {
                Ok("Init complete.\n\n.mcp.json + skill files have been written to the project folder.\nOpen that project with your AI agent, and it can use XERJ.".to_string())
            });
        }
        TaskKind::Search(q) => {
            if q.trim().is_empty() {
                done_err("Query is empty.".into());
                return;
            }
            match core::search_code(&q) {
                Ok(out) => done_ok(out),
                Err(e) => done_err(e),
            }
        }
        TaskKind::Def(s) => {
            if s.trim().is_empty() {
                done_err("Symbol name is empty.".into());
                return;
            }
            match core::def_symbol(&s) {
                Ok(out) => done_ok(out),
                Err(e) => done_err(e),
            }
        }
        TaskKind::Gain => match core::gain_stats() {
            Ok(out) => done_ok(out),
            Err(e) => done_err(e),
        },
        // --dry-run: non-interaktif, aman untuk background task.
        TaskKind::Feedback => match core::feedback_dry_run() {
            Ok(out) => done_ok(out),
            Err(e) => done_err(e),
        },
    }
}

/// Helper streaming: validasi data-dir untuk autoindex, lalu stream output.
fn stream_xerj(
    args: &[String],
    cwd: Option<&PathBuf>,
    tx: &mpsc::Sender<TaskEvent>,
    finish: impl FnOnce(String) -> Result<String, String>,
) {
    // Guard khusus autoindex: data-dir tidak boleh di dalam folder target.
    if args.first().map(|s| s.as_str()) == Some("autoindex") {
        if let Some(folder) = args.get(1) {
            match core::validate_folder(folder) {
                Err(e) => {
                    let _ = tx.send(TaskEvent::Done(Err(e)));
                    return;
                }
                Ok(f) => {
                    if core::data_dir_inside(&f, &core::default_data_dir()) {
                        let _ = tx.send(TaskEvent::Done(Err("Cancelled for safety: the XERJ data folder is INSIDE the folder you want to index.\nMove the data dir elsewhere first (default: ~/xerj-data).".to_string())));
                        return;
                    }
                }
            }
        }
    }
    let proc = match core::spawn_xerj(args, cwd.map(|p| p.as_path())) {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(TaskEvent::Done(Err(e)));
            return;
        }
    };
    let mut child = proc.child;
    let mut reader = proc.stdout;
    let mut collected = String::new();
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let t = line.trim_end().to_string();
                // Saring baris log ANSI yang berantakan? Tidak, tampilkan apa adanya,
                // tapi buang escape sequence kasar agar TUI tidak rusak.
                let clean = strip_ansi(&t);
                collected.push_str(&clean);
                collected.push('\n');
                let _ = tx.send(TaskEvent::Line(clean));
            }
            Err(_) => break,
        }
    }
    let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
    if code != 0 {
        let _ = tx.send(TaskEvent::Done(Err(format!(
            "Command failed (code {}).\n\nMake sure the XERJ server is running ('Start server' menu).\n\n{}",
            code,
            collected.trim()
        ))));
        return;
    }
    match finish(collected.trim().to_string()) {
        Ok(msg) => {
            let body = if msg == "Indexing complete." && !collected.trim().is_empty() {
                collected.trim().to_string()
            } else {
                msg
            };
            let _ = tx.send(TaskEvent::Done(Ok(body)));
        }
        Err(e) => {
            let _ = tx.send(TaskEvent::Done(Err(e)));
        }
    }
}

/// Buang ANSI escape sequence agar tidak merusak TUI.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI ... <final byte @-~>
            if chars.peek() == Some(&'[') {
                chars.next();
                for c2 in chars.by_ref() {
                    if ('@'..='~').contains(&c2) {
                        break;
                    }
                }
                continue;
            }
            continue;
        }
        out.push(c);
    }
    out
}

// Menu

fn build_menu(st: &core::MenuStatus) -> Vec<MenuItem> {
    let mut items = Vec::new();
    if st.installed {
        items.push(MenuItem {
            key: '1',
            label: format!(
                "Install XERJ   [v{} installed]",
                st.version.as_deref().unwrap_or("?")
            ),
            enabled: false,
            action: Action::Install,
        });
    } else {
        items.push(MenuItem {
            key: '1',
            label: "Install XERJ".into(),
            enabled: true,
            action: Action::Install,
        });
    }
    if !st.installed {
        items.push(MenuItem {
            key: '2',
            label: "Start server   (install XERJ first)".into(),
            enabled: false,
            action: Action::ToggleServer,
        });
    } else if st.server_running {
        items.push(MenuItem {
            key: '2',
            label: format!(
                "Stop server   (PID {})",
                st.pid.map(|p| p.to_string()).unwrap_or_else(|| "?".into())
            ),
            enabled: true,
            action: Action::ToggleServer,
        });
    } else {
        items.push(MenuItem {
            key: '2',
            label: "Start server".into(),
            enabled: true,
            action: Action::ToggleServer,
        });
    }
    let need_server = st.installed && st.server_running;
    let tag = |ok: bool| {
        if ok {
            String::new()
        } else {
            "   (needs XERJ + running server)".to_string()
        }
    };
    items.push(MenuItem {
        key: '3',
        label: format!("Index project{}", tag(need_server)),
        enabled: need_server,
        action: Action::Index,
    });
    items.push(MenuItem {
        key: '4',
        label: format!(
            "Init project for AI agent{}",
            if st.installed {
                String::new()
            } else {
                "   (needs XERJ)".to_string()
            }
        ),
        enabled: st.installed,
        action: Action::Init,
    });
    items.push(MenuItem {
        key: '5',
        label: format!("Search code{}", tag(need_server)),
        enabled: need_server,
        action: Action::Search,
    });
    items.push(MenuItem {
        key: '6',
        label: format!("Go-to-definition{}", tag(need_server)),
        enabled: need_server,
        action: Action::Def,
    });
    items.push(MenuItem {
        key: '7',
        label: format!("Statistics (xerj gain){}", tag(need_server)),
        enabled: need_server,
        action: Action::Gain,
    });
    if st.is_macos {
        let label = if st.autostart {
            "Auto-start at login   [ON]"
        } else {
            "Auto-start at login   [OFF]"
        };
        items.push(MenuItem {
            key: '8',
            label: label.into(),
            enabled: true,
            action: Action::Autostart,
        });
    } else {
        items.push(MenuItem {
            key: '8',
            label: "Auto-start at login   (macOS only)".into(),
            enabled: false,
            action: Action::Autostart,
        });
    }
    items.push(MenuItem {
        key: '0',
        label: format!(
            "Send field report to the XERJ community{}",
            if st.installed {
                String::new()
            } else {
                "   (needs XERJ)".to_string()
            }
        ),
        enabled: st.installed,
        action: Action::Feedback,
    });
    items.push(MenuItem {
        key: '9',
        label: "Quit".into(),
        enabled: true,
        action: Action::Quit,
    });
    items
}

// Prompt input (line editor sederhana)

use crossterm::event::{KeyCode, KeyModifiers};

enum PromptResult {
    Keep,
    Cancel,
    Submit,
}

fn prompt_key(
    input: &mut String,
    cursor: &mut usize,
    code: KeyCode,
    mods: KeyModifiers,
) -> PromptResult {
    match code {
        KeyCode::Esc => PromptResult::Cancel,
        KeyCode::Enter => PromptResult::Submit,
        KeyCode::Backspace => {
            if *cursor > 0 {
                let mut chars: Vec<char> = input.chars().collect();
                chars.remove(*cursor - 1);
                *input = chars.into_iter().collect();
                *cursor -= 1;
            }
            PromptResult::Keep
        }
        KeyCode::Delete => {
            let mut chars: Vec<char> = input.chars().collect();
            if *cursor < chars.len() {
                chars.remove(*cursor);
                *input = chars.into_iter().collect();
            }
            PromptResult::Keep
        }
        KeyCode::Left => {
            *cursor = cursor.saturating_sub(1);
            PromptResult::Keep
        }
        KeyCode::Right => {
            let len = input.chars().count();
            *cursor = (*cursor + 1).min(len);
            PromptResult::Keep
        }
        KeyCode::Home => {
            *cursor = 0;
            PromptResult::Keep
        }
        KeyCode::End => {
            *cursor = input.chars().count();
            PromptResult::Keep
        }
        KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => PromptResult::Cancel,
        KeyCode::Char(c) => {
            let mut chars: Vec<char> = input.chars().collect();
            chars.insert(*cursor, c);
            *input = chars.into_iter().collect();
            *cursor += 1;
            PromptResult::Keep
        }
        _ => PromptResult::Keep,
    }
}

/// Simulasi tak-blocking untuk spinner.
pub fn _tick_interval() -> Duration {
    Duration::from_millis(120)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(a: &PendingAction) -> String {
        a.summary_lines().join("\n")
    }

    #[test]
    fn confirm_summaries_are_honest() {
        // Install: names the official installer, promises not to touch the project.
        let s = joined(&PendingAction::Install);
        assert!(s.contains("https://xerj.org/get"));
        assert!(s.contains("Does not touch any project files"));

        // Start server: explains --insecure honestly.
        let s = joined(&PendingAction::StartServer);
        assert!(s.contains("--insecure"));
        assert!(s.contains("NO TLS"));
        assert!(s.contains("127.0.0.1"));

        // Stop: asserts data stays intact.
        let s = joined(&PendingAction::StopServer(Some(123)));
        assert!(s.contains("123"));
        assert!(s.contains("REMAIN INTACT"));

        // Autoindex: read-only against the project.
        let s = joined(&PendingAction::Autoindex("/tmp/proj".into()));
        assert!(s.contains("/tmp/proj"));
        assert!(s.contains("only READ"));

        // Init: names written files + .bak backup.
        let s = joined(&PendingAction::InitProject("/tmp/proj".into()));
        assert!(s.contains(".mcp.json"));
        assert!(s.contains("AGENTS.md"));
        assert!(s.contains(".mcp.json.bak"));
        assert!(s.contains("/tmp/proj"));

        // Autostart: names the TUI-owned plist file.
        let s = joined(&PendingAction::Autostart(true));
        assert!(s.contains("org.xerj.tui.plist"));

        // Feedback: honest that this is not a legal obligation.
        let s = joined(&PendingAction::Feedback);
        assert!(s.contains("--dry-run"));
        assert!(s.contains("NOT a legal obligation"));
    }

    #[test]
    fn every_pending_action_has_title_and_summary() {
        let actions = vec![
            PendingAction::Install,
            PendingAction::StartServer,
            PendingAction::StopServer(None),
            PendingAction::Autoindex("x".into()),
            PendingAction::InitProject("x".into()),
            PendingAction::Autostart(true),
            PendingAction::Autostart(false),
            PendingAction::Feedback,
        ];
        for a in &actions {
            assert!(!a.title().is_empty());
            assert!(
                a.summary_lines().len() >= 4,
                "summary {:?} too short",
                a.title()
            );
        }
    }
}
