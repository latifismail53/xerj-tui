//! Contoh: render layar utama TUI ke teks (untuk screenshot ASCII / verifikasi).

use ratatui::{backend::TestBackend, Terminal};

use xerj_tui::{app::App, ui};

fn main() {
    let mut app = App::new();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| ui::render(f, &mut app))
        .expect("render failed");
    let buf = terminal.backend().buffer();
    for y in 0..24 {
        let mut line = String::new();
        for x in 0..80 {
            line.push_str(buf[(x, y)].symbol());
        }
        println!("{}", line.trim_end());
    }
}
