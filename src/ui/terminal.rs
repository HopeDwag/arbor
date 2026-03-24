use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use vt100_ctt::Parser;
use std::sync::{Arc, Mutex};

/// Render the PTY screen into a ratatui buffer and return the cursor position.
/// Returns (cursor_row, cursor_col) relative to the area origin.
pub fn render_terminal(
    parser: &Arc<Mutex<Parser>>,
    area: Rect,
    buf: &mut Buffer,
    dimmed: bool,
) -> (u16, u16) {
    let parser = parser.lock().unwrap();
    let screen = parser.screen();

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

    screen.cursor_position()
}

fn dim_style(style: Style) -> Style {
    let fg = style.fg.map(dim_color).unwrap_or(Color::DarkGray);
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
        vt100_ctt::Color::Idx(i) => Color::Indexed(i),
        vt100_ctt::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
