//! core.rs -- logika murni xerj-tui (tanpa ratatui/crossterm).
//!
//! Semua deteksi, parsing, dan manajemen server XERJ ada di sini sehingga
//! bisa di-unit-test. Format output yang di-parse didasarkan pada observasi
//! nyata XERJ v1.0.0-rc.96 (bukan tebakan):
//!   - `xerj --version`            -> "xerj v1.0.0-rc.96"
//!   - startup server mencetak ringkasan:
//!       Native REST  127.0.0.1:8080 [plain]
//!       ES-compat    127.0.0.1:9200 [plain]
//!       gRPC         127.0.0.1:8081 [h2c]
//!       Data dir     <data_dir>
//!       Xerj Console UI  http://localhost:9200/_xerj-console/
//!   - tanpa --insecure: API key ditulis ke <data_dir>/admin.key (mode 600),
//!     health check tanpa key -> HTTP 401.
//!   - `xerj autoindex` diakhiri baris "xerj-done ok=true exit=0 ..."
//!   - daftar korpus: index `ax-*` (dataset docs di index `autoindex-catalog`
//!     dengan doc_kind="dataset": slug, index_name, record_count, file_count).

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Konstanta
// ---------------------------------------------------------------------------

/// Health endpoint ES-compat (observasi: selalu 127.0.0.1:9200 [plain]).
pub const HEALTH_URL: &str = "http://localhost:9200/_cluster/health";
/// Base URL server untuk kartu status.
pub const SERVER_URL: &str = "http://localhost:9200";
/// URL console/dashboard web XERJ (observasi: installer + startup mencetak ini).
pub const CONSOLE_URL: &str = "http://localhost:9200/_xerj-console/";
/// Installer resmi XERJ.
pub const INSTALL_CMD: &str = "curl -fsSL https://xerj.org/get | sh";
/// Label LaunchAgent macOS.
pub const LAUNCH_AGENT_LABEL: &str = "org.xerj.tui";

// ---------------------------------------------------------------------------
// Path
// ---------------------------------------------------------------------------

pub fn home_dir() -> PathBuf {
    env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."))
}

/// ~/.xerj-tui -- semua state TUI (pid, state.json, log).
pub fn app_dir() -> PathBuf {
    home_dir().join(".xerj-tui")
}

pub fn pid_file() -> PathBuf {
    app_dir().join("server.pid")
}

pub fn state_file() -> PathBuf {
    app_dir().join("state.json")
}

pub fn server_log() -> PathBuf {
    app_dir().join("xerj-server.log")
}

/// Default data-dir (PENTING: harus di luar folder yang di-index).
pub fn default_data_dir() -> PathBuf {
    home_dir().join("xerj-data")
}

pub fn launch_agent_path() -> PathBuf {
    home_dir().join("Library/LaunchAgents/org.xerj.tui.plist")
}

// ---------------------------------------------------------------------------
// Deteksi install & versi
// ---------------------------------------------------------------------------

/// Cari `xerj` di dalam PATH (diberikan eksplisit agar bisa di-test).
pub fn find_xerj_in(path_env: &str) -> Option<PathBuf> {
    for dir in env::split_paths(path_env) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let cand = dir.join("xerj");
        if is_executable(&cand) {
            return Some(cand);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.is_file()
        && p.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

pub fn find_xerj() -> Option<PathBuf> {
    if let Ok(p) = env::var("PATH") {
        if let Some(f) = find_xerj_in(&p) {
            return Some(f);
        }
    }
    // Fallback: lokasi default installer resmi -- installer sendiri
    // memperingatkan bahwa ~/.local/bin sering belum ada di PATH.
    for cand in [
        home_dir().join(".local/bin/xerj"),
        PathBuf::from("/usr/local/bin/xerj"),
    ] {
        if is_executable(&cand) {
            return Some(cand);
        }
    }
    None
}

/// Parse output `xerj --version`.
/// Observasi nyata: "xerj v1.0.0-rc.96" -> Some("1.0.0-rc.96").
pub fn parse_version(output: &str) -> Option<String> {
    let line = output.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    let rest = line.strip_prefix("xerj").map(str::trim).unwrap_or(line);
    let rest = rest.strip_prefix('v').unwrap_or(rest);
    let ver = rest.split_whitespace().next()?;
    if ver.is_empty() {
        None
    } else {
        Some(ver.to_string())
    }
}

pub fn xerj_version() -> Option<String> {
    let x = find_xerj()?;
    let out = Command::new(&x).arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let text = if text.trim().is_empty() {
        String::from_utf8_lossy(&out.stderr).into_owned()
    } else {
        text.into_owned()
    };
    parse_version(&text)
}

// ---------------------------------------------------------------------------
// HTTP helper (ureq, std-only selain itu)
// ---------------------------------------------------------------------------

fn http_get(url: &str, api_key: Option<&str>) -> Result<(u16, String), String> {
    let mut req = ureq::get(url).timeout(Duration::from_secs(4));
    if let Some(k) = api_key {
        req = req.set("Authorization", &format!("ApiKey {}", k));
    }
    match req.call() {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.into_string().unwrap_or_default();
            Ok((status, body))
        }
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            Ok((code, body))
        }
        Err(e) => Err(format!("tidak bisa menghubungi {}: {}", url, e)),
    }
}

fn http_post_json(url: &str, body: &str, api_key: Option<&str>) -> Result<(u16, String), String> {
    let mut req = ureq::post(url)
        .timeout(Duration::from_secs(10))
        .set("Content-Type", "application/json");
    if let Some(k) = api_key {
        req = req.set("Authorization", &format!("ApiKey {}", k));
    }
    match req.send_string(body) {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.into_string().unwrap_or_default();
            Ok((status, text))
        }
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            Ok((code, text))
        }
        Err(e) => Err(format!("tidak bisa menghubungi {}: {}", url, e)),
    }
}

/// GET dengan fallback API key: kalau 401 dan data_dir punya admin.key,
/// baca key-nya lalu coba lagi. Mengembalikan (status, body, key_terpakai).
fn http_get_smart(url: &str, data_dir: Option<&Path>) -> Result<(u16, String), String> {
    let (code, body) = http_get(url, None)?;
    if code != 401 {
        return Ok((code, body));
    }
    if let Some(dir) = data_dir {
        if let Some(key) = read_admin_key(dir) {
            let (code2, body2) = http_get(url, Some(&key))?;
            return Ok((code2, body2));
        }
    }
    Ok((code, body))
}

fn http_post_smart(url: &str, body: &str, data_dir: Option<&Path>) -> Result<(u16, String), String> {
    let (code, resp) = http_post_json(url, body, None)?;
    if code != 401 {
        return Ok((code, resp));
    }
    if let Some(dir) = data_dir {
        if let Some(key) = read_admin_key(dir) {
            return http_post_json(url, body, Some(&key));
        }
    }
    Ok((code, resp))
}

// ---------------------------------------------------------------------------
// PID & status server
// ---------------------------------------------------------------------------

pub fn read_pid(p: &Path) -> Option<u32> {
    fs::read_to_string(p).ok()?.trim().parse().ok()
}

pub fn pid_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        Path::new(&format!("/proc/{}", pid)).exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

pub struct ServerStatus {
    pub pid: Option<u32>,
    pub pid_alive: bool,
    pub healthy: bool,
}

impl ServerStatus {
    pub fn running(&self) -> bool {
        self.healthy
    }
}

pub fn server_status_with(pid_file: &Path, health_url: &str) -> ServerStatus {
    let pid = read_pid(pid_file);
    let alive = pid.map(pid_alive).unwrap_or(false);
    let healthy = http_get(health_url, None)
        .map(|(c, _)| c == 200)
        .unwrap_or(false);
    ServerStatus {
        pid,
        pid_alive: alive,
        healthy,
    }
}

/// Versi "pintar": kalau health check dijawab 401 (server pakai auth),
/// coba lagi dengan admin.key dari data_dir yang tercatat di state.
pub fn server_status_smart(
    pid_file: &Path,
    health_url: &str,
    data_dir: Option<&Path>,
) -> ServerStatus {
    let pid = read_pid(pid_file);
    let alive = pid.map(pid_alive).unwrap_or(false);
    let healthy = http_get_smart(health_url, data_dir)
        .map(|(c, _)| c == 200)
        .unwrap_or(false);
    ServerStatus {
        pid,
        pid_alive: alive,
        healthy,
    }
}

pub fn server_status() -> ServerStatus {
    let state = read_state(&state_file());
    server_status_smart(&pid_file(), HEALTH_URL, state.data_dir.as_deref())
}

/// Coba temukan PID server xerj yang sedang jalan (untuk kasus server
/// dijalankan di luar TUI sehingga tidak ada catatan PID).
fn adopt_xerj_server() -> Option<u32> {
    let me = std::process::id();
    // Pola [x]erj: trik bracket agar tidak match dirinya sendiri.
    for pattern in ["[x]erj --insecure --data-dir", "[x]erj --data-dir"] {
        let out = Command::new("pgrep").args(["-f", pattern]).output().ok()?;
        if !out.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if let Ok(pid) = line.trim().parse::<u32>() {
                if pid != me && pid_alive(pid) {
                    return Some(pid);
                }
            }
        }
    }
    None
}

/// Kalau server sehat tapi pid_file kosong/basi, coba adopsi via pgrep
/// lalu catat. Mengembalikan PID yang valid, atau None.
/// Dipakai eksplisit dari App::refresh (bukan dari fungsi status murni).
pub fn ensure_pid_recorded(pid_file: &Path, health_url: &str) -> Option<u32> {
    let st = server_status_with(pid_file, health_url);
    if !st.healthy {
        return None;
    }
    if let Some(pid) = st.pid {
        if pid_alive(pid) {
            return Some(pid);
        }
    }
    if let Some(pid) = adopt_xerj_server() {
        let _ = fs::write(pid_file, pid.to_string());
        return Some(pid);
    }
    None
}

// ---------------------------------------------------------------------------
// State (state.json) -- ingat data_dir & mode server yang kita jalankan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ServerState {
    pub data_dir: Option<PathBuf>,
    pub pid: Option<u32>,
    pub insecure: bool,
}

pub fn read_state(path: &Path) -> ServerState {
    let mut st = ServerState::default();
    let Ok(text) = fs::read_to_string(path) else {
        return st;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return st;
    };
    if let Some(d) = v.get("data_dir").and_then(|x| x.as_str()) {
        st.data_dir = Some(PathBuf::from(d));
    }
    if let Some(p) = v.get("pid").and_then(|x| x.as_u64()) {
        st.pid = Some(p as u32);
    }
    if let Some(i) = v.get("insecure").and_then(|x| x.as_bool()) {
        st.insecure = i;
    }
    st
}

fn write_state(path: &Path, data_dir: &Path, pid: u32, insecure: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("gagal membuat folder: {}", e))?;
    }
    let json = format!(
        "{{\"data_dir\":{},\"pid\":{},\"insecure\":{}}}",
        serde_json::to_string(&data_dir.to_string_lossy()).unwrap_or("\"\"".into()),
        pid,
        insecure
    );
    fs::write(path, json).map_err(|e| format!("gagal menulis state: {}", e))?;
    Ok(())
}

/// Lokasi API key. Observasi: tanpa --insecure, server menulis
/// <data_dir>/admin.key (mode 600) saat start pertama.
pub fn read_admin_key(data_dir: &Path) -> Option<String> {
    let text = fs::read_to_string(data_dir.join("admin.key")).ok()?;
    let key = text.trim().to_string();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

// ---------------------------------------------------------------------------
// Start / stop server
// ---------------------------------------------------------------------------

pub fn start_server_with(
    data_dir: &Path,
    pid_file: &Path,
    state_file: &Path,
    log_path: &Path,
    health_url: &str,
) -> Result<u32, String> {
    let xerj = find_xerj().ok_or_else(|| {
        "XERJ belum terinstall. Pilih menu 'Install XERJ' dulu ya.".to_string()
    })?;

    let st = server_status_with(pid_file, health_url);
    if st.healthy {
        if let Some(pid) = ensure_pid_recorded(pid_file, health_url) {
            return Ok(pid);
        }
        return Err("Server XERJ sudah jalan, tapi PID-nya tidak bisa dipastikan \
                    (kemungkinan dijalankan di luar TUI).\nHentikan manual lalu start dari TUI ya."
            .into());
    }

    fs::create_dir_all(data_dir).map_err(|e| format!("gagal membuat data-dir: {}", e))?;
    if let Some(parent) = pid_file.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("gagal membuat folder: {}", e))?;
    }
    let _ = fs::remove_file(pid_file); // bersihkan PID basi
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| format!("gagal membuka log: {}", e))?;
    let log2 = log.try_clone().map_err(|e| format!("gagal duplikat log: {}", e))?;

    let mut cmd = Command::new(&xerj);
    cmd.args(["--insecure", "--data-dir"])
        .arg(data_dir)
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(log2);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0); // lepas dari grup proses terminal
    }
    let mut child = cmd.spawn().map_err(|e| format!("gagal menjalankan xerj: {}", e))?;
    let pid = child.id();
    // Thread reaper: wait() agar tidak jadi zombie saat server berhenti.
    // Server tetap jalan mandiri (tidak di-kill saat TUI keluar).
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    fs::write(pid_file, pid.to_string()).map_err(|e| format!("gagal mencatat PID: {}", e))?;
    write_state(state_file, data_dir, pid, true)?;

    let deadline = Instant::now() + Duration::from_secs(25);
    while Instant::now() < deadline {
        if !pid_alive(pid) {
            let _ = fs::remove_file(pid_file);
            return Err(format!(
                "Server XERJ langsung mati setelah dijalankan.\nIntip log-nya di: {}",
                log_path.display()
            ));
        }
        if http_get(health_url, None)
            .map(|(c, _)| c == 200)
            .unwrap_or(false)
        {
            return Ok(pid);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err("Server XERJ tidak merespons health check dalam 25 detik.\nCoba 'Stop server' lalu 'Start server' lagi ya.".into())
}

pub fn start_server(data_dir: &Path) -> Result<u32, String> {
    start_server_with(
        data_dir,
        &pid_file(),
        &state_file(),
        &server_log(),
        HEALTH_URL,
    )
}

pub fn stop_server_with(pid_file: &Path) -> Result<String, String> {
    let pid = match read_pid(pid_file).filter(|p| pid_alive(*p)) {
        Some(p) => p,
        None => {
            // Tidak ada catatan valid: coba adopsi (server dari luar TUI?).
            match adopt_xerj_server() {
                Some(p) => p,
                None => {
                    let _ = fs::remove_file(pid_file);
                    return Ok("Tidak ada server XERJ yang berjalan.".into());
                }
            }
        }
    };
    // SIGTERM dulu, beri waktu, lalu SIGKILL kalau bandel.
    let _ = Command::new("kill").arg(pid.to_string()).output();
    let deadline = Instant::now() + Duration::from_secs(5);
    while pid_alive(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(250));
    }
    if pid_alive(pid) {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
    }
    // Tunggu benar-benar hilang (reaper thread butuh sesaat untuk wait()).
    let deadline2 = Instant::now() + Duration::from_secs(3);
    while pid_alive(pid) && Instant::now() < deadline2 {
        std::thread::sleep(Duration::from_millis(100));
    }
    let _ = fs::remove_file(pid_file);
    if pid_alive(pid) {
        return Err(format!(
            "Server (PID {}) bandel, tidak mau berhenti. Coba 'kill -9 {}' manual ya.",
            pid, pid
        ));
    }
    Ok(format!("Server berhenti (PID {}). Sampai jumpa lagi!", pid))
}

pub fn stop_server() -> Result<String, String> {
    stop_server_with(&pid_file())
}

// ---------------------------------------------------------------------------
// Menjalankan perintah xerj
// ---------------------------------------------------------------------------

/// Jalankan `xerj <args>` dengan timeout. Kembalikan (exit_code, output).
pub fn run_xerj(args: &[&str], cwd: Option<&Path>, timeout: Duration) -> Result<(i32, String), String> {
    let xerj = find_xerj().ok_or_else(|| {
        "XERJ belum terinstall. Pilih menu 'Install XERJ' dulu ya.".to_string()
    })?;
    let mut cmd = Command::new(&xerj);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let mut child = cmd.spawn().map_err(|e| format!("gagal menjalankan xerj: {}", e))?;

    let mut out_h = child.stdout.take();
    let mut err_h = child.stderr.take();
    let t_out = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(h) = out_h.as_mut() {
            let _ = h.read_to_string(&mut s);
        }
        s
    });
    let t_err = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(h) = err_h.as_mut() {
            let _ = h.read_to_string(&mut s);
        }
        s
    });

    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| format!("gagal menunggu xerj: {}", e))? {
            Some(st) => break st,
            None => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err("Perintahnya kelamaan dan di-timeout. Coba lagi ya.".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    };
    let mut text = t_out.join().unwrap_or_default();
    let err = t_err.join().unwrap_or_default();
    if !err.trim().is_empty() {
        text.push_str("\n");
        text.push_str(err.trim());
    }
    Ok((status.code().unwrap_or(-1), text.trim().to_string()))
}

/// Versi streaming: kembalikan Child agar UI bisa membaca baris per baris.
pub struct XerjProc {
    pub child: std::process::Child,
    pub stdout: std::io::BufReader<std::process::ChildStdout>,
}

pub fn spawn_xerj(args: &[String], cwd: Option<&Path>) -> Result<XerjProc, String> {
    let xerj = find_xerj().ok_or_else(|| {
        "XERJ belum terinstall. Pilih menu 'Install XERJ' dulu ya.".to_string()
    })?;
    let mut cmd = Command::new(&xerj);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let mut child = cmd.spawn().map_err(|e| format!("gagal menjalankan xerj: {}", e))?;
    let stdout = child.stdout.take().ok_or("gagal mengambil stdout")?;
    Ok(XerjProc {
        child,
        stdout: std::io::BufReader::new(stdout),
    })
}

// ---------------------------------------------------------------------------
// Validasi input
// ---------------------------------------------------------------------------

pub fn validate_folder(path: &str) -> Result<PathBuf, String> {
    let p = shellexpand(path);
    if p.trim().is_empty() {
        return Err("Path-nya kosong. Coba paste lagi ya.".into());
    }
    let p = PathBuf::from(p.trim());
    if !p.exists() {
        return Err(format!("Folder tidak ketemu: {}\nCek lagi path-nya ya.", p.display()));
    }
    if !p.is_dir() {
        return Err(format!("Ini file, bukan folder: {}", p.display()));
    }
    Ok(p.canonicalize().unwrap_or(p))
}

fn shellexpand(path: &str) -> String {
    let t = path.trim();
    if let Some(rest) = t.strip_prefix("~/") {
        return home_dir().join(rest).to_string_lossy().into_owned();
    }
    if t == "~" {
        return home_dir().to_string_lossy().into_owned();
    }
    // $HOME/... sederhana
    if let Some(rest) = t.strip_prefix("$HOME/") {
        return home_dir().join(rest).to_string_lossy().into_owned();
    }
    t.to_string()
}

/// True kalau data-dir ada DI DALAM folder yang mau di-index (bahaya:
/// XERJ akan meng-index databasenya sendiri).
pub fn data_dir_inside(folder: &Path, data_dir: &Path) -> bool {
    let folder = normalize(folder);
    let data = normalize(data_dir);
    data == folder || data.starts_with(format!("{}{}", folder.display(), std::path::MAIN_SEPARATOR))
}

fn normalize(p: &Path) -> PathBuf {
    // canonicalize kalau ada; kalau belum ada, pakai absolut kasar.
    if let Ok(c) = p.canonicalize() {
        return c;
    }
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(p)
    };
    // sederhanakan . dan ..
    let mut out = PathBuf::new();
    for comp in abs.components() {
        use std::path::Component::*;
        match comp {
            CurDir => {}
            ParentDir => {
                out.pop();
            }
            _ => out.push(comp.as_os_str()),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Fitur: autoindex, init, search, def, gain
// ---------------------------------------------------------------------------

pub fn autoindex(folder: &str, data_dir: &Path) -> Result<String, String> {
    let folder = validate_folder(folder)?;
    if data_dir_inside(&folder, data_dir) {
        return Err(format!(
            "Batal demi keamanan: folder data XERJ ({}) ada DI DALAM folder yang mau di-index.\n\n\
             Kalau dilanjut, XERJ malah meng-index databasenya sendiri.\n\
             Pindahkan data-dir keluar dulu (default: ~/xerj-data).",
            data_dir.display()
        ));
    }
    let folder_s = folder.to_string_lossy().into_owned();
    let (code, out) = run_xerj(&["autoindex", &folder_s], None, Duration::from_secs(1800))?;
    if code != 0 {
        return Err(format!(
            "Indexing gagal (kode keluar {}).\n\n{}\n\nPastikan server XERJ jalan (menu 'Start server'), lalu coba lagi.",
            code, out
        ));
    }
    Ok(if out.is_empty() {
        "Indexing selesai.".into()
    } else {
        out
    })
}

pub fn init_project(folder: &str) -> Result<String, String> {
    let folder = validate_folder(folder)?;
    let (code, out) = run_xerj(&["init"], Some(&folder), Duration::from_secs(120))?;
    if code != 0 {
        return Err(format!("Init gagal (kode keluar {}).\n\n{}", code, out));
    }
    let mut msg = if out.is_empty() { "Init selesai.".into() } else { out };
    msg.push_str(
        "\n\nFile .mcp.json + skill sudah ditulis di folder project.\nBuka project itu dengan AI agent kamu, dan dia bisa memakai XERJ.",
    );
    Ok(msg)
}

pub fn search_code(query: &str) -> Result<String, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("Query-nya kosong. Tulis dulu yang mau dicari ya.".into());
    }
    let (code, out) = run_xerj(&["search", q], None, Duration::from_secs(120))?;
    if code != 0 {
        return Err(format!(
            "Pencarian gagal. Pastikan server XERJ jalan (menu 'Start server').\n\n{}",
            out
        ));
    }
    Ok(if out.is_empty() {
        "(tidak ada hasil)".into()
    } else {
        out
    })
}

pub fn def_symbol(symbol: &str) -> Result<String, String> {
    let s = symbol.trim();
    if s.is_empty() {
        return Err("Nama simbolnya kosong. Tulis dulu ya, mis. nama_fungsi.".into());
    }
    let (code, out) = run_xerj(&["def", s], None, Duration::from_secs(120))?;
    if code != 0 {
        return Err(format!(
            "Go-to-definition gagal. Pastikan server XERJ jalan.\n\n{}",
            out
        ));
    }
    Ok(if out.is_empty() {
        "(simbol tidak ketemu)".into()
    } else {
        out
    })
}

pub fn gain_stats() -> Result<String, String> {
    let (code, out) = run_xerj(&["gain"], None, Duration::from_secs(60))?;
    if code != 0 {
        return Err(format!("Gagal membaca statistik.\n\n{}", out));
    }
    Ok(if out.is_empty() {
        "(belum ada statistik)".into()
    } else {
        out
    })
}

/// Draf field report komunitas XERJ (mode --dry-run: hanya mencetak,
/// tidak mengirim apa-apa, non-interaktif).
pub fn feedback_dry_run() -> Result<String, String> {
    let (code, out) = run_xerj(&["feedback", "--dry-run"], None, Duration::from_secs(60))?;
    if code != 0 {
        return Err(format!("Gagal membuat draf field report.\n\n{}", out));
    }
    Ok(if out.is_empty() {
        "(tidak ada output)".into()
    } else {
        out
    })
}

// ---------------------------------------------------------------------------
// Install XERJ (installer resmi)
// ---------------------------------------------------------------------------

pub fn install_cmd() -> String {
    env::var("XERJ_TUI_INSTALL_CMD").unwrap_or_else(|_| INSTALL_CMD.to_string())
}

pub fn install_xerj_blocking() -> Result<String, String> {
    if find_in_path("curl").is_none() {
        return Err("Perintah 'curl' tidak ketemu di sistem kamu. Install curl dulu ya.".into());
    }
    let cmd = install_cmd();
    let out = Command::new("sh")
        .args(["-c", &cmd])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("gagal menjalankan installer: {}", e))?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.trim().is_empty() {
        text.push_str("\n");
        text.push_str(err.trim());
    }
    if !out.status.success() {
        return Err(format!(
            "Installer gagal (kode {}).\n\n{}\n\nCek koneksi internet lalu coba lagi.",
            out.status.code().unwrap_or(-1),
            text.trim()
        ));
    }
    Ok(text.trim().to_string())
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    env::var("PATH").ok().and_then(|p| {
        env::split_paths(&p)
            .map(|d| d.join(name))
            .find(|c| c.is_file())
    })
}

// ---------------------------------------------------------------------------
// Kartu Status XERJ -- merapikan output xerj yang berantakan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ApiKeyInfo {
    /// Server jalan dengan --insecure: auth nonaktif (observasi nyata).
    DisabledInsecure,
    /// <data_dir>/admin.key ada.
    Present(PathBuf),
    /// data_dir diketahui tapi admin.key belum ada.
    Missing(PathBuf),
    /// Tidak bisa ditentukan (mis. server dijalankan di luar TUI).
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CorpusInfo {
    pub index: String,
    pub records: Option<u64>,
    pub files: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct StatusCard {
    pub version: Option<String>,
    pub server_url: String,
    pub console_url: String,
    pub api_key: ApiKeyInfo,
    pub data_dir: Option<PathBuf>,
    pub running: bool,
    pub pid: Option<u32>,
    pub corpora: Vec<CorpusInfo>,
    pub note: Option<String>,
}

/// Daftar korpus dari index autoindex-catalog (doc_kind="dataset").
/// Observasi nyata: {slug, index_name, record_count, file_count}.
fn fetch_corpora(data_dir: Option<&Path>) -> Vec<CorpusInfo> {
    let url = "http://localhost:9200/autoindex-catalog/_search";
    let body = r#"{"size":100,"query":{"term":{"doc_kind":"dataset"}}}"#;
    let (code, text) = match http_post_smart(url, body, data_dir) {
        Ok(x) => x,
        Err(_) => return Vec::new(),
    };
    if code != 200 {
        return Vec::new();
    }
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    if let Some(hits) = v
        .get("hits")
        .and_then(|h| h.get("hits"))
        .and_then(|h| h.as_array())
    {
        for h in hits {
            let src = match h.get("_source") {
                Some(s) => s,
                None => continue,
            };
            let index = src
                .get("index_name")
                .and_then(|x| x.as_str())
                .unwrap_or("?")
                .to_string();
            out.push(CorpusInfo {
                index,
                records: src.get("record_count").and_then(|x| x.as_u64()),
                files: src.get("file_count").and_then(|x| x.as_u64()),
            });
        }
    }
    out.sort_by(|a, b| a.index.cmp(&b.index));
    out
}

pub fn status_card() -> StatusCard {
    let version = xerj_version();
    let st = server_status();
    let state = read_state(&state_file());
    let data_dir = state.data_dir.clone();

    let api_key = match (&data_dir, state.insecure) {
        (_, true) => ApiKeyInfo::DisabledInsecure,
        (Some(dir), false) => {
            let key_path = dir.join("admin.key");
            if key_path.exists() {
                ApiKeyInfo::Present(key_path)
            } else if st.running() {
                // Server jalan tapi bukan dari TUI dan tanpa jejak insecure:
                // kemungkinan dijalankan di luar TUI.
                ApiKeyInfo::Unknown
            } else {
                ApiKeyInfo::Missing(key_path)
            }
        }
        (None, false) => ApiKeyInfo::Unknown,
    };

    let note = if st.running() && data_dir.is_none() {
        Some("server tampaknya dijalankan di luar TUI (data-dir tidak diketahui)".into())
    } else {
        None
    };

    let corpora = if st.running() {
        fetch_corpora(data_dir.as_deref())
    } else {
        Vec::new()
    };

    StatusCard {
        version,
        server_url: SERVER_URL.into(),
        console_url: CONSOLE_URL.into(),
        api_key,
        data_dir,
        running: st.running(),
        pid: st.pid,
        corpora,
        note,
    }
}

// ---------------------------------------------------------------------------
// Status ringkas untuk menu
// ---------------------------------------------------------------------------

pub struct MenuStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub server_running: bool,
    pub pid: Option<u32>,
    pub autostart: bool,
    pub is_macos: bool,
}

pub fn menu_status() -> MenuStatus {
    let x = find_xerj();
    let st = server_status();
    MenuStatus {
        installed: x.is_some(),
        version: xerj_version(),
        server_running: st.running(),
        pid: st.pid,
        autostart: autostart_enabled(),
        is_macos: cfg!(target_os = "macos"),
    }
}

// ---------------------------------------------------------------------------
// Auto-start saat login (macOS LaunchAgent)
// ---------------------------------------------------------------------------

pub fn build_plist(xerj_path: &Path, data_dir: &Path, log_path: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{xerj}</string>
        <string>--insecure</string>
        <string>--data-dir</string>
        <string>{data}</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
    <key>StandardOutPath</key><string>{log}</string>
    <key>StandardErrorPath</key><string>{log}</string>
</dict>
</plist>
"#,
        label = LAUNCH_AGENT_LABEL,
        xerj = xerj_path.display(),
        data = data_dir.display(),
        log = log_path.display(),
    )
}

pub fn autostart_enabled() -> bool {
    #[cfg(target_os = "macos")]
    {
        launch_agent_path().exists()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub fn set_autostart(enable: bool) -> Result<String, String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = enable;
        return Err("Auto-start saat ini hanya didukung di macOS.\nDi Linux: belum didukung -- jalankan server manual dari menu 'Start server' ya.".into());
    }
    #[cfg(target_os = "macos")]
    {
        let path = launch_agent_path();
        if enable {
            let xerj = find_xerj().ok_or_else(|| {
                "Install XERJ dulu sebelum mengaktifkan auto-start.".to_string()
            })?;
            let data_dir = default_data_dir();
            std::fs::create_dir_all(app_dir()).map_err(|e| format!("gagal membuat folder: {}", e))?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("gagal membuat folder: {}", e))?;
            }
            std::fs::write(&path, build_plist(&xerj, &data_dir, &server_log()))
                .map_err(|e| format!("gagal menulis plist: {}", e))?;
            let rc = Command::new("launchctl")
                .args(["load", "-w"])
                .arg(&path)
                .output()
                .map(|o| o.status.code().unwrap_or(-1))
                .unwrap_or(-1);
            if rc != 0 {
                return Err("File auto-start sudah ditulis, tapi 'launchctl load' gagal.\nCoba restart Mac kamu -- biasanya langsung aktif setelah itu.".into());
            }
            Ok(format!(
                "Auto-start AKTIF. Server XERJ akan jalan sendiri tiap kamu login.\nFile: {}",
                path.display()
            ))
        } else {
            let _ = Command::new("launchctl").args(["unload"]).arg(&path).output();
            let _ = std::fs::remove_file(&path);
            Ok("Auto-start MATI. Server tidak akan jalan otomatis lagi.".into())
        }
    }
}

// ---------------------------------------------------------------------------
// Unit test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_real_format() {
        // Observasi nyata: `xerj --version` -> "xerj v1.0.0-rc.96"
        assert_eq!(
            parse_version("xerj v1.0.0-rc.96\n"),
            Some("1.0.0-rc.96".to_string())
        );
    }

    #[test]
    fn parse_version_bare() {
        assert_eq!(parse_version("1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(parse_version("v2.0"), Some("2.0".to_string()));
    }

    #[test]
    fn parse_version_garbage() {
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("\n  \n"), None);
    }

    #[test]
    fn data_dir_inside_detects_nesting() {
        let folder = Path::new("/home/u/proj");
        assert!(data_dir_inside(folder, &Path::new("/home/u/proj/xerj-data")));
        assert!(data_dir_inside(folder, &Path::new("/home/u/proj")));
        assert!(!data_dir_inside(folder, &Path::new("/home/u/xerj-data")));
        assert!(!data_dir_inside(folder, &Path::new("/home/u/proj2/xerj-data")));
    }

    #[test]
    fn validate_folder_rejects_missing() {
        let r = validate_folder("/jalan/yang/pasti/tidak/ada/xyz123");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("tidak ketemu"));
    }

    #[test]
    fn validate_folder_rejects_empty() {
        assert!(validate_folder("   ").is_err());
    }

    #[test]
    fn validate_folder_accepts_tmp() {
        let tmp = env::temp_dir();
        assert!(validate_folder(tmp.to_str().unwrap()).is_ok());
    }

    #[test]
    fn validate_folder_rejects_file() {
        let p = env::temp_dir().join("xerj-tui-test-file.txt");
        fs::write(&p, "x").unwrap();
        let r = validate_folder(p.to_str().unwrap());
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("bukan folder"));
        let _ = fs::remove_file(&p);
    }

    #[cfg(unix)]
    #[test]
    fn find_xerj_in_detects_executable() {
        let dir = env::temp_dir().join("xerj-tui-find-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("xerj");
        fs::write(&bin, "#!/bin/sh\necho hi\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        let path_env = dir.to_string_lossy().into_owned();
        assert_eq!(find_xerj_in(&path_env), Some(bin));
        // Tanpa bit executable -> tidak ketemu.
        fs::set_permissions(dir.join("xerj"), fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(find_xerj_in(&path_env), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn plist_has_required_keys() {
        let p = build_plist(
            Path::new("/usr/local/bin/xerj"),
            Path::new("/home/u/xerj-data"),
            Path::new("/home/u/.xerj-tui/xerj-server.log"),
        );
        assert!(p.contains("org.xerj.tui"));
        assert!(p.contains("/usr/local/bin/xerj"));
        assert!(p.contains("--insecure"));
        assert!(p.contains("--data-dir"));
        assert!(p.contains("/home/u/xerj-data"));
        assert!(p.contains("<true/>")); // RunAtLoad + KeepAlive
    }

    #[test]
    fn read_state_missing_file_is_default() {
        let st = read_state(Path::new("/tidak/ada/state.json"));
        assert!(st.data_dir.is_none());
        assert!(!st.insecure);
    }

    #[test]
    fn read_state_parses_json() {
        let p = env::temp_dir().join("xerj-tui-state-test.json");
        fs::write(&p, r#"{"data_dir":"/tmp/dd","pid":1234,"insecure":true}"#).unwrap();
        let st = read_state(&p);
        assert_eq!(st.data_dir, Some(PathBuf::from("/tmp/dd")));
        assert_eq!(st.pid, Some(1234));
        assert!(st.insecure);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn pid_alive_self() {
        assert!(pid_alive(std::process::id()));
    }

    #[test]
    fn pid_alive_bogus() {
        assert!(!pid_alive(999_999_999));
    }
}
