//! Smoke tests for UI rendering with ratatui's TestBackend (no terminal).

use ratatui::{backend::TestBackend, Terminal};

use xerj_tui::app::{App, PendingAction, PromptKind, Screen};
use xerj_tui::ui;

fn render_to_string(app: &mut App, w: u16, h: u16) -> String {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| ui::render(f, app))
        .expect("render failed");
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
    assert!(out.contains("xerj-tui"), "header missing");
    assert!(out.contains("XERJ Status"), "status card missing");
    assert!(out.contains("Install XERJ"), "install menu missing");
    assert!(out.contains("Quit"), "quit menu missing");
    assert!(out.contains("version"), "version row missing");
    assert!(out.contains("console"), "console row missing");
}

#[test]
fn prompt_renders_popup() {
    let mut app = App::new();
    app.screen = Screen::Prompt {
        kind: PromptKind::SearchQuery,
        input: "login function".into(),
        cursor: 14,
    };
    let out = render_to_string(&mut app, 80, 24);
    assert!(out.contains("Search code"), "popup title missing");
    assert!(out.contains("login function"), "input not shown");
}

#[test]
fn output_renders_scrollable() {
    let mut app = App::new();
    app.screen = Screen::Output {
        title: "Search code".into(),
        lines: vec!["line one".into(), "line two".into()],
        scroll: 0,
    };
    let out = render_to_string(&mut app, 80, 24);
    assert!(out.contains("line one"));
    assert!(out.contains("line two"));
}

#[test]
fn confirm_renders_summary() {
    let mut app = App::new();
    app.screen = Screen::Confirm {
        action: PendingAction::InitProject("/tmp/proj".into()),
    };
    let out = render_to_string(&mut app, 80, 24);
    assert!(out.contains("Confirm"), "confirm title missing");
    assert!(out.contains("What will happen"), "summary missing");
    assert!(out.contains(".mcp.json.bak"), "backup info missing");
    assert!(out.contains("/tmp/proj"), "project path missing");
}

#[test]
fn confirm_start_server_explains_insecure() {
    let mut app = App::new();
    app.screen = Screen::Confirm {
        action: PendingAction::StartServer,
    };
    let out = render_to_string(&mut app, 80, 24);
    assert!(out.contains("--insecure"), "--insecure explanation missing");
    assert!(out.contains("127.0.0.1"), "bind info missing");
}

#[test]
fn renders_on_small_terminal_without_panic() {
    let mut app = App::new();
    // 80x24 is the minimum supported; must not panic.
    let _ = render_to_string(&mut app, 80, 24);
    // Smaller terminal: still must not panic.
    let _ = render_to_string(&mut app, 60, 20);
}
