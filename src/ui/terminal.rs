use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
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
                let contents = cell.contents();
                let fg = convert_vt100_color(cell.fgcolor());
                let bg = convert_vt100_color(cell.bgcolor());
                let style = if dimmed {
                    Style::default().fg(dim_color(fg)).bg(bg)
                } else {
                    Style::default().fg(fg).bg(bg)
                };

                if contents.is_empty() {
                    buf[(buf_x, buf_y)].set_char(' ').set_style(style);
                } else {
                    buf[(buf_x, buf_y)].set_symbol(contents).set_style(style);
                }
            }
        }
    }

    screen.cursor_position()
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
