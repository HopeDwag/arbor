use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget};

use crate::app::{Dialog, DialogField};
use crate::persistence::WorkflowStatus;
use crate::worktree::WorktreeInfo;

pub struct ControlPanelState {
    pub selected: usize,
    pub worktrees: Vec<WorktreeInfo>,
    pub show_plus: bool,
}

pub fn render_control_panel(
    state: &ControlPanelState,
    dialog: &Dialog,
    area: Rect,
    buf: &mut Buffer,
    focused: bool,
    spinner_frame: u8,
    pty_last_outputs: &std::collections::HashMap<std::path::PathBuf, u64>,
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

    let now_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let groups: &[(WorkflowStatus, &str)] = &[
        (WorkflowStatus::InProgress, "IN PROGRESS"),
        (WorkflowStatus::Queued, "QUEUED"),
        (WorkflowStatus::Done, "DONE"),
    ];

    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_to_visual: Vec<usize> = Vec::new();

    for (status, label) in groups {
        let group_wts: Vec<(usize, &WorktreeInfo)> = state.worktrees.iter()
            .enumerate()
            .filter(|(_, wt)| wt.workflow_status == *status)
            .collect();

        if group_wts.is_empty() {
            continue;
        }

        // Group header
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" {}", label),
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
        ))));

        for (flat_idx, wt) in &group_wts {
            let is_selected = *flat_idx == state.selected;

            // Activity icon
            let icon = if let Some(&last_output) = pty_last_outputs.get(&wt.path) {
                if last_output > 0 && now_millis.saturating_sub(last_output) < 500 {
                    let frames = ['\u{280B}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283C}', '\u{2834}', '\u{2826}', '\u{2827}', '\u{2807}', '\u{280F}'];
                    let frame_char = frames[(spinner_frame % 10) as usize];
                    Span::styled(format!("{} ", frame_char), Style::default().fg(Color::Cyan))
                } else {
                    Span::styled("! ", Style::default().fg(Color::Yellow))
                }
            } else {
                match wt.workflow_status {
                    WorkflowStatus::Queued => Span::styled("\u{25B6} ", Style::default().fg(Color::DarkGray)),
                    WorkflowStatus::Done => Span::styled("\u{2713} ", Style::default().fg(Color::Green)),
                    WorkflowStatus::InProgress => Span::styled("\u{00B7} ", Style::default().fg(Color::DarkGray)),
                }
            };

            let display_name = wt.short_name.as_deref().unwrap_or(&wt.branch);
            let name_style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let name_line = Line::from(vec![
                Span::raw("  "),
                icon,
                Span::styled(display_name, name_style),
            ]);

            flat_to_visual.push(items.len());
            items.push(ListItem::new(name_line));
        }

        items.push(ListItem::new(Line::from("")));
    }

    // [+] new worktree button
    let plus_style = if state.selected == state.worktrees.len() {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let plus_visual_idx = items.len();
    items.push(ListItem::new(Line::from(Span::styled(
        "  [+] new worktree",
        plus_style,
    ))));

    let mut list_state = ListState::default();
    if state.selected < state.worktrees.len() {
        if let Some(&visual_idx) = flat_to_visual.get(state.selected) {
            list_state.select(Some(visual_idx));
        }
    } else {
        list_state.select(Some(plus_visual_idx));
    }

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray));

    StatefulWidget::render(list, inner, buf, &mut list_state);

    // Render dialog overlay at bottom of sidebar
    match dialog {
        Dialog::CreateInput { input, short_name, active_field, archived, selected_archived } => {
            let has_archived = !archived.is_empty();
            let dialog_height: u16 = if has_archived { 6 } else { 4 };
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
            } else if *active_field == DialogField::Branch {
                Style::default().fg(Color::Cyan).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            };
            let prompt = Line::from(vec![
                Span::styled(" Branch: ", Style::default().fg(Color::White).bg(Color::DarkGray)),
                Span::styled(format!("{}_", input), input_style),
            ]);
            buf.set_line(dialog_area.x, row, &prompt, dialog_area.width);
            row += 1;

            let name_style = if *active_field == DialogField::Name {
                Style::default().fg(Color::Cyan).bg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            };
            let name_prompt = Line::from(vec![
                Span::styled(" Name:   ", Style::default().fg(Color::White).bg(Color::DarkGray)),
                Span::styled(format!("{}_", short_name), name_style),
            ]);
            buf.set_line(dialog_area.x, row, &name_prompt, dialog_area.width);
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
                    let preview = format!(" \u{2192} {}", archived[*idx]);
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
                " Enter confirm \u{00B7} Esc cancel",
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
                " Branch kept \u{00B7} restore with n",
                Style::default().fg(Color::Gray).bg(Color::DarkGray),
            ));
            buf.set_line(dialog_area.x, dialog_area.y + 2, &hint, dialog_area.width);
        }
        Dialog::None => {}
    }
}
