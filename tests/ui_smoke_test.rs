//! Smoke test rendering UI dengan ratatui TestBackend (tanpa terminal).

use ratatui::{backend::TestBackend, Terminal};

use xerj_tui::app::{App, PendingAction, PromptKind, Screen};
use xerj_tui::ui;

fn render_to_string(app: &mut App, w: u16, h: u16) -> String {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| ui::render(f, app))
        .expect("render gagal");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol().to_string())
        .collect()
}

#[test]
fn menu_renders_status_card_and_items() {
    let mut app = App::new();
    let out = render_to_string(&mut app, 80, 24);
    assert!(out.contains("xerj-tui"), "header hilang");
    assert!(out.contains("Status XERJ"), "kartu status hilang");
    assert!(out.contains("Install XERJ"), "menu install hilang");
    assert!(out.contains("Keluar"), "menu keluar hilang");
    assert!(out.contains("versi"), "baris versi hilang");
    assert!(out.contains("console"), "baris console hilang");
}

#[test]
fn prompt_renders_popup() {
    let mut app = App::new();
    app.screen = Screen::Prompt {
        kind: PromptKind::SearchQuery,
        input: "fungsi login".into(),
        cursor: 12,
    };
    let out = render_to_string(&mut app, 80, 24);
    assert!(out.contains("Cari kode"), "judul popup hilang");
    assert!(out.contains("fungsi login"), "input tidak tampil");
}

#[test]
fn output_renders_scrollable() {
    let mut app = App::new();
    app.screen = Screen::Output {
        title: "Cari kode".into(),
        lines: vec!["baris satu".into(), "baris dua".into()],
        scroll: 0,
    };
    let out = render_to_string(&mut app, 80, 24);
    assert!(out.contains("baris satu"));
    assert!(out.contains("baris dua"));
}

#[test]
fn confirm_renders_summary() {
    let mut app = App::new();
    app.screen = Screen::Confirm {
        action: PendingAction::InitProject("/tmp/proyek".into()),
    };
    let out = render_to_string(&mut app, 80, 24);
    assert!(out.contains("Konfirmasi"), "judul konfirmasi hilang");
    assert!(out.contains("Yang akan dilakukan"), "ringkasan hilang");
    assert!(out.contains(".mcp.json.bak"), "info backup hilang");
    assert!(out.contains("/tmp/proyek"), "path project hilang");
}

#[test]
fn confirm_start_server_explains_insecure() {
    let mut app = App::new();
    app.screen = Screen::Confirm {
        action: PendingAction::StartServer,
    };
    let out = render_to_string(&mut app, 80, 24);
    assert!(out.contains("--insecure"), "penjelasan --insecure hilang");
    assert!(out.contains("127.0.0.1"), "info bind hilang");
}

#[test]
fn renders_on_small_terminal_without_panic() {
    let mut app = App::new();
    // 80x24 minimum yang didukung; jangan panic.
    let _ = render_to_string(&mut app, 80, 24);
    // Terminal lebih kecil: tetap tidak boleh panic.
    let _ = render_to_string(&mut app, 60, 20);
}
