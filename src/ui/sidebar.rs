use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget};

use crate::worktree::format_age;
use crate::worktree::WorktreeInfo;

pub struct SidebarState {
    pub selected: usize,
    pub worktrees: Vec<WorktreeInfo>,
    pub show_plus: bool,
}

pub fn render_sidebar(
    state: &SidebarState,
    area: Rect,
    buf: &mut Buffer,
    focused: bool,
) {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" arbor ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    block.render(area, buf);

    let mut items: Vec<ListItem> = Vec::new();

    for (i, wt) in state.worktrees.iter().enumerate() {
        let is_selected = i == state.selected;

        let prefix = if wt.is_main { "🌳 " } else if is_selected { "▼ " } else { "▶ " };
        let name_style = if is_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let name_line = Line::from(vec![
            Span::raw(prefix),
            Span::styled(&wt.branch, name_style),
        ]);

        let status_line = if let Some(ref status) = wt.status {
            let dot = if status.is_dirty {
                Span::styled("● ", Style::default().fg(Color::Yellow))
            } else {
                Span::styled("● ", Style::default().fg(Color::Green))
            };
            let label = if status.is_dirty { "dirty" } else { "clean" };
            let age = format_age(status.last_commit_age_secs);
            Line::from(vec![
                Span::raw("  "),
                dot,
                Span::styled(format!("{} · {}", label, age), Style::default().fg(Color::DarkGray)),
            ])
        } else {
            Line::from(Span::styled("  ? unknown", Style::default().fg(Color::DarkGray)))
        };

        items.push(ListItem::new(vec![name_line, status_line]));
    }

    // [+] new button
    items.push(ListItem::new(Line::from(Span::styled(
        "  [+] new",
        Style::default().fg(Color::DarkGray),
    ))));

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray));

    StatefulWidget::render(list, inner, buf, &mut list_state);

    // Footer with keybinding hints
    let footer_y = area.bottom().saturating_sub(1);
    if footer_y > area.y {
        let hints = " n d ? help ";
        let hints_span = Span::styled(hints, Style::default().fg(Color::DarkGray));
        buf.set_line(area.x + 1, footer_y, &Line::from(hints_span), area.width - 2);
    }
}
