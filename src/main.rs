//! main.rs -- entry point xerj-tui.

use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use xerj_tui::app::App;
use xerj_tui::ui;

fn main() {
    if let Err(e) = run() {
        eprintln!("xerj-tui gagal: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    enable_raw_mode().map_err(|e| format!("gagal masuk mode terminal: {}", e))?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| format!("gagal buka layar: {}", e))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| format!("gagal init terminal: {}", e))?;

    let mut app = App::new();
    let result = event_loop(&mut terminal, &mut app);

    disable_raw_mode().map_err(|e| format!("gagal keluar mode terminal: {}", e))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| format!("gagal tutup layar: {}", e))?;
    terminal.show_cursor().map_err(|e| format!("gagal tampilkan kursor: {}", e))?;

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), String> {
    loop {
        terminal
            .draw(|f| ui::render(f, app))
            .map_err(|e| format!("gagal menggambar: {}", e))?;

        if event::poll(Duration::from_millis(120)).map_err(|e| format!("event: {}", e))? {
            if let Event::Key(key) = event::read().map_err(|e| format!("event: {}", e))? {
                // Abaikan key-release / repeat agar tidak dobel.
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if app.handle_key(key.code, key.modifiers) {
                    return Ok(());
                }
            }
        }
        app.poll_task();
    }
}
