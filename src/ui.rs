//! Rendering ratatui. Tema: gelap, rapi, satu aksen (cyan).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, PendingAction, PromptKind, Screen};
use crate::core::{ApiKeyInfo, StatusCard};

// Palet (prinsip antislop-ui):
//   - Netral: teks terang (White/Gray) di atas background gelap terminal.
//   - DIM (DarkGray): border dan label sekunder, bukan dekorasi warna.
//   - AKSEN cyan: HANYA untuk elemen yang sedang fokus/dipilih
//     (marker menu terpilih, border popup modal). Satu momen aksen.
//   - Warna semantik HANYA untuk state nyata:
//       hijau = OK/jalan, merah = error, kuning = peringatan.
const ACCENT: Color = Color::Cyan;
const OK: Color = Color::Green;
const ERR: Color = Color::Red;
const WARN: Color = Color::Yellow;
const DIM: Color = Color::DarkGray;
const TEXT: Color = Color::Gray;
const TITLE: Color = Color::White;

/// Border netral untuk panel yang tidak fokus.
fn border_neutral() -> Style {
    Style::default().fg(DIM)
}

/// Border aksen: hanya untuk popup modal yang sedang fokus.
fn border_focus() -> Style {
    Style::default().fg(ACCENT)
}

/// Aksen: hanya untuk marker item menu yang sedang dipilih
/// (satu momen aksen di layar menu).
fn accent() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

fn dim() -> Style {
    Style::default().fg(DIM)
}

/// Judul panel: netral terang, tanpa warna aksen.
fn title_style() -> Style {
    Style::default().fg(TITLE)
}

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // header
            Constraint::Min(0),    // konten
            Constraint::Length(2), // footer
        ])
        .split(area);

    render_header(f, chunks[0]);

    // Jenis screen disalin dulu agar tidak menahan borrow saat render_*
    // butuh &mut App (untuk scroll offset menu).
    enum Kind {
        Menu,
        Prompt,
        Confirm,
        Busy,
        Output,
        Message,
    }
    let kind = match &app.screen {
        Screen::Menu => Kind::Menu,
        Screen::Prompt { .. } => Kind::Prompt,
        Screen::Confirm { .. } => Kind::Confirm,
        Screen::Busy { .. } => Kind::Busy,
        Screen::Output { .. } => Kind::Output,
        Screen::Message { .. } => Kind::Message,
    };
    match kind {
        Kind::Menu => render_menu_screen(f, chunks[1], app),
        Kind::Prompt => {
            render_menu_screen(f, chunks[1], app);
            if let Screen::Prompt { kind, input, cursor } = &app.screen {
                render_prompt_popup(f, area, kind, input, *cursor);
            }
        }
        Kind::Confirm => {
            render_menu_screen(f, chunks[1], app);
            if let Screen::Confirm { action } = &app.screen {
                render_confirm_popup(f, area, action);
            }
        }
        Kind::Busy => {
            if let Screen::Busy { title, lines } = &app.screen {
                let (t, l) = (title.clone(), lines.clone());
                render_busy(f, chunks[1], &t, &l);
            }
        }
        Kind::Output => {
            if let Screen::Output {
                title,
                lines,
                scroll,
            } = &app.screen
            {
                let (t, l, s) = (title.clone(), lines.clone(), *scroll);
                render_scrollable(f, chunks[1], &t, &l, s);
            }
        }
        Kind::Message => {
            if let Screen::Message { title, lines } = &app.screen {
                let (t, l) = (title.clone(), lines.clone());
                render_message(f, chunks[1], &t, &l);
            }
        }
    }
    render_footer(f, chunks[2], app);
}

fn render_header(f: &mut Frame, area: Rect) {
    let title = Line::from(vec![
        Span::styled(" xerj-tui", title_style()),
        Span::styled(", a friendly companion for XERJ", dim()),
    ]);
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(DIM))
        .title(title)
        .title_alignment(Alignment::Left);
    f.render_widget(block, area);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let hints = match &app.screen {
        Screen::Menu => "↑↓ select · Enter run · 0-9 shortcuts · r refresh · q quit",
        Screen::Prompt { .. } => "type, then Enter · Esc cancel",
        Screen::Confirm { .. } => "Enter = continue · Esc = cancel",
        Screen::Busy { .. } => "command running, please wait...",
        Screen::Output { .. } => "↑↓/PgUp/PgDn scroll · q back",
        Screen::Message { .. } => "press any key to go back",
    };
    let p = Paragraph::new(Line::from(Span::styled(hints, dim())))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(DIM)),
        )
        .alignment(Alignment::Center);
    f.render_widget(p, area);
}

// Layar menu: kartu status + daftar menu

fn render_menu_screen(f: &mut Frame, area: Rect, app: &mut App) {
    // Kartu status kompak (8 baris) + menu scrollable mengisi sisa.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(0)])
        .split(area);
    render_status_card(f, chunks[0], &app.card);
    render_menu_list(f, chunks[1], app);
}

fn kv<'a>(k: &'a str, v: String, vstyle: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!(" {:<9}", k), dim()),
        Span::styled(v, vstyle),
    ])
}

/// Kartu status: 6 baris konten, satu kolom, padat tapi terbaca.
/// Merapikan output startup xerj yang berantakan menjadi info penting saja.
fn render_status_card(f: &mut Frame, area: Rect, card: &StatusCard) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_neutral())
        .title(Span::styled(" XERJ Status ", title_style()))
        .title_alignment(Alignment::Left);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let width = inner.width as usize;

    let mut rows: Vec<Line> = Vec::new();

    // Baris 1: versi + status server.
    let ver = card
        .version
        .as_deref()
        .map(|v| format!("v{}", v))
        .unwrap_or_else(|| "not installed".into());
    let mut l1 = vec![
        Span::styled(" version ", dim()),
        Span::styled(ver, Style::default().fg(TEXT)),
        Span::styled("      status ", dim()),
    ];
    if card.running {
        l1.push(Span::styled(
            "● RUNNING",
            Style::default().fg(OK).add_modifier(Modifier::BOLD),
        ));
        if let Some(pid) = card.pid {
            l1.push(Span::styled(format!(" (PID {})", pid), dim()));
        }
    } else {
        l1.push(Span::styled(
            "○ STOPPED",
            Style::default().fg(DIM).add_modifier(Modifier::BOLD),
        ));
    }
    rows.push(Line::from(l1));

    // Baris 2-3: URL API + console.
    rows.push(kv(" api", card.server_url.clone(), Style::default().fg(TEXT)));
    rows.push(kv(
        " console",
        card.console_url.clone(),
        Style::default().fg(TEXT),
    ));

    // Baris 4: API key.
    let (key_txt, key_style) = match &card.api_key {
        ApiKeyInfo::DisabledInsecure => (
            "nonaktif (--insecure)".to_string(),
            Style::default().fg(WARN),
        ),
        ApiKeyInfo::Present(p) => (
            format!("ada ({})", shorten(&p.display().to_string(), 40)),
            Style::default().fg(OK),
        ),
        ApiKeyInfo::Missing(p) => (
            format!("none ({})", shorten(&p.display().to_string(), 36)),
            Style::default().fg(WARN),
        ),
        ApiKeyInfo::Unknown => ("unknown".to_string(), dim()),
    };
    rows.push(kv(" api key", key_txt, key_style));

    // Baris 5: data dir.
    rows.push(kv(
        " data",
        card.data_dir
            .as_ref()
            .map(|p| shorten(&p.display().to_string(), 52))
            .unwrap_or_else(|| "-".into()),
        Style::default().fg(TEXT),
    ));

    // Baris 6: korpus terindex.
    let korpus = if !card.running {
        "-".to_string()
    } else if card.corpora.is_empty() {
        "No projects indexed yet. Use 'Index project' to start.".to_string()
    } else {
        let parts: Vec<String> = card
            .corpora
            .iter()
            .map(|c| match (c.records, c.files) {
                (Some(r), Some(fl)) => format!("{} ({} recs, {} files)", c.index, r, fl),
                (Some(r), None) => format!("{} ({} recs)", c.index, r),
                _ => c.index.clone(),
            })
            .collect();
        parts.join(", ")
    };
    let mut korpus_line = vec![
        Span::styled(" corpora ", dim()),
        Span::styled(
            shorten(&korpus, width.saturating_sub(12)),
            Style::default().fg(TEXT),
        ),
    ];
    if let Some(note) = &card.note {
        korpus_line.push(Span::styled(format!("  ! {}", note), dim()));
    }
    rows.push(Line::from(korpus_line));

    f.render_widget(Paragraph::new(rows), inner);
}

/// Potong string kepanjangan dengan ellipsis.
fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn render_menu_list(f: &mut Frame, area: Rect, app: &mut App) {
    // Scroll mengikuti seleksi agar muat di terminal pendek (80x24).
    let h = area.height as usize;
    let n = app.items.len();
    let mut off = app.menu_offset.min(n.saturating_sub(1));
    if app.selected < off {
        off = app.selected;
    } else if h > 0 && app.selected >= off + h {
        off = app.selected + 1 - h;
    }
    app.menu_offset = off;

    let items: Vec<ListItem> = app
        .items
        .iter()
        .enumerate()
        .skip(off)
        .take(h.max(1))
        .map(|(i, item)| {
            let marker = if i == app.selected { "› " } else { "  " };
            let mut spans = vec![
                Span::styled(
                    marker,
                    if i == app.selected {
                        accent()
                    } else {
                        dim()
                    },
                ),
                Span::styled(
                    format!("{}  {}", item.key, item.label),
                    if !item.enabled {
                        dim()
                    } else if i == app.selected {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(TEXT)
                    },
                ),
            ];
            if !item.enabled && item.label.contains("terinstall") {
                spans.push(Span::styled("  ✓", Style::default().fg(OK)));
            }
            let mut li = ListItem::new(Line::from(spans));
            if i == app.selected && item.enabled {
                li = li.style(Style::default().bg(Color::Rgb(20, 40, 50)));
            }
            li
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM))
            .title(Span::styled(" Menu ", title_style())),
    );
    f.render_widget(list, area);
}

// Popup, busy, output, message

fn centered_popup(area: Rect, w: u16, h: u16) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

fn render_prompt_popup(f: &mut Frame, area: Rect, kind: &PromptKind, input: &str, cursor: usize) {
    let w = 64.min(area.width.saturating_sub(4));
    let popup = centered_popup(area, w, 9);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_focus())
        .title(Span::styled(format!(" {} ", kind.title()), title_style()));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(inner);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(kind.label(), Style::default().fg(TEXT)))),
        chunks[0],
    );

    // Field input dengan kursor.
    let field_w = chunks[1].width as usize;
    let chars: Vec<char> = input.chars().collect();
    let cursor = cursor.min(chars.len());
    let start = cursor.saturating_sub(field_w.saturating_sub(4));
    let visible: String = chars[start..].iter().take(field_w.saturating_sub(4)).collect();
    let cx = (cursor - start) as u16;
    let field = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(visible, Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM)),
    );
    f.render_widget(field, chunks[1]);
    // Posisikan kursor terminal di dalam field.
    f.set_cursor_position((
        chunks[1].x + 2 + cx.min(field_w as u16 - 4),
        chunks[1].y + 1,
    ));

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(kind.hint(), dim()))),
        chunks[2],
    );
}

/// Popup konfirmasi: menampilkan ringkasan jujur "yang akan dilakukan"
/// sebelum aksi yang mengubah sesuatu di luar TUI dieksekusi.
/// Border aksen karena ini lapisan modal yang sedang fokus.
fn render_confirm_popup(f: &mut Frame, area: Rect, action: &PendingAction) {
    let lines = action.summary_lines();
    let w = 68.min(area.width.saturating_sub(4));
    let h = (lines.len() as u16 + 6).min(area.height.saturating_sub(2)).max(9);
    let popup = centered_popup(area, w, h);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_focus())
        .title(Span::styled(
            format!(" Confirm: {} ", action.title()),
            title_style(),
        ))
        .title_bottom(Line::from(Span::styled(
            " Enter = continue · Esc = cancel ",
            Style::default().fg(ACCENT),
        )));
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    let text: Vec<Line> = lines
        .iter()
        .map(|l| {
            // Netral semua; warna semantik hanya untuk state nyata,
            // bukan untuk dekorasi daftar.
            let style = if l == "What will happen:" {
                Style::default().fg(TITLE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };
            Line::from(Span::styled(l.clone(), style))
        })
        .collect();
    f.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), inner);
}

fn render_busy(f: &mut Frame, area: Rect, title: &str, lines: &[String]) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_neutral())
        .title(Span::styled(format!(" {} ", title), title_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);
    // Tampilkan N baris terakhir.
    let h = inner.height as usize;
    let start = lines.len().saturating_sub(h);
    let text: Vec<Line> = lines[start..]
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(TEXT))))
        .collect();
    f.render_widget(Paragraph::new(text), inner);
}

fn render_scrollable(f: &mut Frame, area: Rect, title: &str, lines: &[String], scroll: usize) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_neutral())
        .title(Span::styled(format!(" {} ", title), title_style()))
        .title_bottom(Line::from(Span::styled(
            format!(" line {}/{} ", scroll + 1, lines.len().max(1)),
            dim(),
        )));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let h = inner.height as usize;
    let text: Vec<Line> = lines
        .iter()
        .skip(scroll)
        .take(h)
        .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(TEXT))))
        .collect();
    f.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), inner);
}

fn render_message(f: &mut Frame, area: Rect, title: &str, lines: &[String]) {
    let w = 66.min(area.width.saturating_sub(4));
    let h = (lines.len() as u16 + 6).min(area.height.saturating_sub(2)).max(7);
    let popup = centered_popup(area, w, h);
    f.render_widget(Clear, popup);
    // Red ONLY for a real error state ("Failed"), not decoration.
    let title_style = if title == "Failed" {
        Style::default().fg(ERR).add_modifier(Modifier::BOLD)
    } else {
        title_style()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_neutral())
        .title(Span::styled(format!(" {} ", title), title_style));
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    let text: Vec<Line> = lines
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(TEXT))))
        .collect();
    f.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), inner);
}
