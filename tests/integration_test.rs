//! Integration test dengan fake `xerj` shim.
//!
//! Satu test sekuensial (bukan paralel) yang menjalankan alur penuh:
//! deteksi -> versi -> start server -> status -> autoindex -> init ->
//! search -> def -> gain -> stop. Health check memakai HTTP server
//! palsu dari stdlib (tanpa crate tambahan).

use std::io::{Read, Write};
use std::path::PathBuf;

use xerj_tui::core;

fn base_dir() -> PathBuf {
    std::env::temp_dir().join(format!("xerj-tui-it-{}", std::process::id()))
}

/// HTTP server palsu dengan "gate": menjawab 503 sampai gate dibuka,
/// lalu 200 {"status":"green"}. Mensimulasikan server yang sedang boot.
fn start_gated_health() -> (String, std::sync::Arc<std::sync::atomic::AtomicBool>) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let up = Arc::new(AtomicBool::new(false));
    let up_srv = up.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut s) = stream {
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf); // abaikan isi request
                let (code, reason, body) = if up_srv.load(Ordering::SeqCst) {
                    (200, "OK", r#"{"status":"green"}"#)
                } else {
                    (503, "Service Unavailable", r#"{"status":"red"}"#)
                };
                let resp = format!(
                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    code,
                    reason,
                    body.len(),
                    body
                );
                let _ = s.write_all(resp.as_bytes());
            }
        }
    });
    // Buka gate setelah 1 detik: mensimulasikan server selesai boot.
    let up_timer = up.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        up_timer.store(true, Ordering::SeqCst);
    });
    (
        format!("http://127.0.0.1:{}/_cluster/health", port),
        up,
    )
}

#[test]
fn full_flow_with_shim() {
    let base = base_dir();
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("bin")).unwrap();

    // 1. Pasang shim sebagai `xerj`.
    let shim_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/shim/xerj");
    let shim_dst = base.join("bin/xerj");
    std::fs::copy(&shim_src, &shim_dst).expect("copy shim");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&shim_dst, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let new_path = format!(
        "{}:{}",
        base.join("bin").display(),
        "/usr/bin:/bin"
    );
    std::env::set_var("PATH", &new_path);

    // 2. Deteksi + versi (format "xerj v9.9.9-test" seperti aslinya).
    let found = core::find_xerj().expect("shim harus ketemu di PATH");
    assert!(found.ends_with("bin/xerj"));
    assert_eq!(core::xerj_version().as_deref(), Some("9.9.9-test"));

    // 3. Start server (shim --insecure -> sleep, PID hidup).
    //    Health gate masih tertutup (503) -> status awal: tidak jalan.
    let (health_url, _gate) = start_gated_health();
    let data_dir = base.join("data");
    let pid_file = base.join("server.pid");
    let state_file = base.join("state.json");
    let log_file = base.join("server.log");
    let st0 = core::server_status_with(&pid_file, &health_url);
    assert!(!st0.healthy, "gate masih tertutup, harusnya belum healthy");

    let pid = core::start_server_with(&data_dir, &pid_file, &state_file, &log_file, &health_url)
        .expect("start_server_with");
    assert!(pid > 0);
    assert!(core::pid_alive(pid));
    assert!(pid_file.exists());
    // state.json tercatat
    let state_text = std::fs::read_to_string(&state_file).unwrap();
    assert!(state_text.contains("data_dir"));
    assert!(state_text.contains(&data_dir.to_string_lossy().into_owned()));

    // 4. Status: healthy. Start kedua harus mengembalikan PID yang sama.
    let st = core::server_status_with(&pid_file, &health_url);
    assert!(st.healthy);
    assert!(st.running());
    assert_eq!(st.pid, Some(pid));
    let pid2 =
        core::start_server_with(&data_dir, &pid_file, &state_file, &log_file, &health_url)
            .expect("start kedua");
    assert_eq!(pid2, pid, "server sudah jalan: jangan start dua kali");

    // 5. Autoindex project.
    let proj = base.join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(proj.join("a.php"), "<?php // halo").unwrap();
    let out = core::autoindex(proj.to_str().unwrap(), &data_dir).expect("autoindex");
    assert!(out.contains("xerj-done"));

    // 6. Init project -> .mcp.json tertulis.
    let msg = core::init_project(proj.to_str().unwrap()).expect("init_project");
    assert!(msg.contains("init OK") || msg.contains(".mcp.json"));
    assert!(proj.join(".mcp.json").exists());

    // 7. Search / def / gain.
    let out = core::search_code("fungsi login").expect("search");
    assert!(out.contains("hasil search"));
    let out = core::def_symbol("hitung_total").expect("def");
    assert!(out.contains("hitung_total"));
    let out = core::gain_stats().expect("gain");
    assert!(out.contains("gain:"));

    // 7b. Feedback dry-run (tidak mengirim apa-apa).
    let out = core::feedback_dry_run().expect("feedback");
    assert!(out.contains("dry-run"));

    // 8. Stop server.
    let msg = core::stop_server_with(&pid_file).expect("stop");
    assert!(msg.contains("berhenti"));
    assert!(!pid_file.exists());
    assert!(!core::pid_alive(pid));

    let _ = std::fs::remove_dir_all(&base);
}
