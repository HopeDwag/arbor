use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use vt100_ctt::Parser;
use std::sync::{Arc, Mutex};

use crate::ui::theme::THEME;

/// Render the PTY screen into a ratatui buffer and return the cursor position
/// plus the clamped scrollback offset.
/// Returns (cursor_row, cursor_col, clamped_scroll_offset).
pub fn render_terminal(
    parser: &Arc<Mutex<Parser>>,
    area: Rect,
    buf: &mut Buffer,
    dimmed: bool,
    scroll_offset: usize,
) -> (u16, u16, usize) {
    let mut parser = parser.lock().unwrap();
    let screen = parser.screen_mut();
    screen.set_scrollback(scroll_offset);
    let clamped = screen.scrollback();

    let rows = area.height.min(screen.size().0);
    let cols = area.width.min(screen.size().1);

    for row in 0..rows {
        let buf_y = area.y + row;
        for col in 0..cols {
            let buf_x = area.x + col;
            if let Some(cell) = screen.cell(row, col) {
                // Skip wide character continuation cells
                if cell.is_wide_continuation() {
                    continue;
                }

                let fg = convert_vt100_color(cell.fgcolor());
                let bg = convert_vt100_color(cell.bgcolor());

                let mut style = Style::reset().fg(fg).bg(bg);

                // Map all text attributes
                if cell.bold() {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if cell.italic() {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if cell.underline() {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                if cell.inverse() {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                if cell.dim() {
                    style = style.add_modifier(Modifier::DIM);
                }

                if dimmed {
                    style = dim_style(style);
                }

                if cell.has_contents() {
                    buf[(buf_x, buf_y)].set_symbol(cell.contents()).set_style(style);
                } else {
                    buf[(buf_x, buf_y)].set_char(' ').set_style(style);
                }
            }
        }
    }

    // Show scroll indicator when scrolled up
    if clamped > 0 && cols > 0 && rows > 0 {
        let label = format!(" [+{}] ", clamped);
        let label_len = label.len().min(cols as usize);
        let start_x = area.x + cols - label_len as u16;
        let indicator_style = Style::reset().fg(THEME.bg0).bg(THEME.yellow).add_modifier(Modifier::BOLD);
        for (i, ch) in label.chars().take(label_len).enumerate() {
            buf[(start_x + i as u16, area.y)].set_char(ch).set_style(indicator_style);
        }
    }

    let cursor = screen.cursor_position();
    (cursor.0, cursor.1, clamped)
}

fn dim_style(style: Style) -> Style {
    let fg = style.fg.map(dim_color).unwrap_or(THEME.grey0);
    let bg = style.bg.unwrap_or(Color::Reset);
    // Strip modifiers when dimmed for a muted look
    Style::reset().fg(fg).bg(bg).add_modifier(Modifier::DIM)
}

fn dim_color(color: Color) -> Color {
    match color {
        Color::Reset => Color::DarkGray,
        Color::Rgb(r, g, b) => Color::Rgb(r / 2, g / 2, b / 2),
        Color::White => Color::Gray,
        Color::Gray => Color::DarkGray,
        _ => color,
    }
}

fn convert_vt100_color(color: vt100_ctt::Color) -> Color {
    match color {
        vt100_ctt::Color::Default => Color::Reset,
        vt100_ctt::Color::Idx(i) => match i {
            0 => THEME.bg3,
            1 | 9 => THEME.red,
            2 | 10 => THEME.green,
            3 | 11 => THEME.yellow,
            4 | 12 => THEME.blue,
            5 | 13 => THEME.purple,
            6 | 14 => THEME.aqua,
            7 | 15 => THEME.fg,
            8 => THEME.bg4,
            _ => Color::Indexed(i),
        },
        vt100_ctt::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
