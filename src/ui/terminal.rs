use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;
use vt100_ctt::Parser;
use std::sync::{Arc, Mutex};

pub struct TerminalWidget {
    parser: Arc<Mutex<Parser>>,
}

impl TerminalWidget {
    pub fn new(parser: Arc<Mutex<Parser>>) -> Self {
        Self { parser }
    }
}

impl Widget for TerminalWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();

        for row in 0..area.height.min(screen.size().0) {
            for col in 0..area.width.min(screen.size().1) {
                let cell = screen.cell(row, col);
                if let Some(cell) = cell {
                    let contents = cell.contents();
                    let fg = convert_vt100_color(cell.fgcolor());
                    let bg = convert_vt100_color(cell.bgcolor());
                    let style = Style::default().fg(fg).bg(bg);

                    let buf_x = area.x + col;
                    let buf_y = area.y + row;
                    if buf_x < area.right() && buf_y < area.bottom() {
                        if contents.is_empty() {
                            buf[(buf_x, buf_y)].set_char(' ').set_style(style);
                        } else {
                            buf[(buf_x, buf_y)]
                                .set_symbol(contents)
                                .set_style(style);
                        }
                    }
                }
            }
        }
    }
}

fn convert_vt100_color(color: vt100_ctt::Color) -> Color {
    match color {
        vt100_ctt::Color::Default => Color::Reset,
        vt100_ctt::Color::Idx(i) => Color::Indexed(i),
        vt100_ctt::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
