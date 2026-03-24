use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget};

use crate::app::Dialog;
use crate::worktree::format_age;
use crate::worktree::WorktreeInfo;

pub struct SidebarState {
    pub selected: usize,
    pub worktrees: Vec<WorktreeInfo>,
    pub show_plus: bool,
}

pub fn render_sidebar(
    state: &SidebarState,
    dialog: &Dialog,
    area: Rect,
    buf: &mut Buffer,
    focused: bool,
    drag_handle_active: bool,
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

    // Highlight the right border when hovering/dragging to show it's resizable
    if drag_handle_active {
        let right_col = area.right().saturating_sub(1);
        let handle_style = Style::default().fg(Color::Yellow);
        for y in area.y..area.bottom() {
            let cell = &mut buf[(right_col, y)];
            cell.set_style(handle_style);
        }
        // Draw a grip indicator in the middle of the border
        let mid_y = area.y + area.height / 2;
        if mid_y > area.y && mid_y < area.bottom().saturating_sub(1) {
            buf[(right_col, mid_y.saturating_sub(1))].set_symbol("╟");
            buf[(right_col, mid_y)].set_symbol("↔");
            buf[(right_col, mid_y + 1)].set_symbol("╢");
        }
    }

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

    // [+] new worktree button
    let plus_style = if state.selected == state.worktrees.len() {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    items.push(ListItem::new(Line::from(Span::styled(
        "  [+] new worktree",
        plus_style,
    ))));

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray));

    StatefulWidget::render(list, inner, buf, &mut list_state);

    // Render dialog overlay at bottom of sidebar
    match dialog {
        Dialog::CreateInput { input, archived, selected_archived } => {
            let has_archived = !archived.is_empty();
            let dialog_height: u16 = if has_archived { 5 } else { 3 };
            let dialog_area = Rect {
                x: area.x + 1,
                y: area.bottom().saturating_sub(dialog_height + 1),
                width: area.width.saturating_sub(2),
                height: dialog_height,
            };
            for y in dialog_area.y..dialog_area.bottom() {
                for x in dialog_area.x..dialog_area.right() {
                    buf[(x, y)].set_char(' ').set_style(Style::default().bg(Color::DarkGray));
                }
            }
            let mut row = dialog_area.y;

            let title_line = Line::from(Span::styled(
                " New worktree",
                Style::default().fg(Color::Cyan).bg(Color::DarkGray).add_modifier(Modifier::BOLD),
            ));
            buf.set_line(dialog_area.x, row, &title_line, dialog_area.width);
            row += 1;

            let input_style = if selected_archived.is_some() {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Cyan).bg(Color::DarkGray)
            };
            let prompt = Line::from(vec![
                Span::styled(" Branch: ", Style::default().fg(Color::White).bg(Color::DarkGray)),
                Span::styled(format!("{}_", input), input_style),
            ]);
            buf.set_line(dialog_area.x, row, &prompt, dialog_area.width);
            row += 1;

            if has_archived {
                let archived_label = format!(
                    " Tab: restore ({} archived)",
                    archived.len()
                );
                let archived_line = Line::from(Span::styled(
                    archived_label,
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                ));
                buf.set_line(dialog_area.x, row, &archived_line, dialog_area.width);
                row += 1;

                // Show current archived selection if cycling
                if let Some(idx) = selected_archived {
                    let preview = format!(" → {}", archived[*idx]);
                    let preview_line = Line::from(Span::styled(
                        preview,
                        Style::default().fg(Color::Yellow).bg(Color::DarkGray).add_modifier(Modifier::BOLD),
                    ));
                    buf.set_line(dialog_area.x, row, &preview_line, dialog_area.width);
                    row += 1;
                } else {
                    row += 1;
                }
            }

            let _ = row; // suppress unused warning
            let hint_y = dialog_area.bottom().saturating_sub(1);
            let hint = Line::from(Span::styled(
                " Enter confirm · Esc cancel",
                Style::default().fg(Color::Gray).bg(Color::DarkGray),
            ));
            buf.set_line(dialog_area.x, hint_y, &hint, dialog_area.width);
        }
        Dialog::ArchiveConfirm(_idx, name) => {
            let dialog_area = Rect {
                x: area.x + 1,
                y: area.bottom().saturating_sub(4),
                width: area.width.saturating_sub(2),
                height: 3,
            };
            for y in dialog_area.y..dialog_area.bottom() {
                for x in dialog_area.x..dialog_area.right() {
                    buf[(x, y)].set_char(' ').set_style(Style::default().bg(Color::DarkGray));
                }
            }
            let title = Line::from(Span::styled(
                " Archive worktree",
                Style::default().fg(Color::Yellow).bg(Color::DarkGray).add_modifier(Modifier::BOLD),
            ));
            buf.set_line(dialog_area.x, dialog_area.y, &title, dialog_area.width);

            let prompt = Line::from(Span::styled(
                format!(" Remove {}? (y/n)", name),
                Style::default().fg(Color::White).bg(Color::DarkGray),
            ));
            buf.set_line(dialog_area.x, dialog_area.y + 1, &prompt, dialog_area.width);

            let hint = Line::from(Span::styled(
                " Branch kept · restore with n",
                Style::default().fg(Color::Gray).bg(Color::DarkGray),
            ));
            buf.set_line(dialog_area.x, dialog_area.y + 2, &hint, dialog_area.width);
        }
        Dialog::None => {}
    }
}
