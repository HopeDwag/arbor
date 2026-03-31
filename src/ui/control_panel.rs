use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget};

use crate::app::{Dialog, DialogField};
use crate::github::PrState;
use crate::persistence::WorkflowStatus;
use crate::worktree::{WorktreeInfo, format_age};

pub struct ControlPanelState {
    pub selected: usize,
    pub worktrees: Vec<WorktreeInfo>,
    pub row_to_flat_idx: Vec<Option<usize>>,
    pub group_regions: Vec<(WorkflowStatus, u16, u16)>, // (status, start_row, end_row)
}

pub fn render_control_panel(
    state: &mut ControlPanelState,
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
    let footer_height = 1u16;
    let list_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height.saturating_sub(footer_height),
    };
    let footer_area = Rect {
        x: inner.x,
        y: inner.bottom().saturating_sub(footer_height),
        width: inner.width,
        height: footer_height,
    };

    let now_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let groups: &[(WorkflowStatus, &str)] = &[
        (WorkflowStatus::InProgress, "IN PROGRESS"),
        (WorkflowStatus::InReview, "IN REVIEW"),
        (WorkflowStatus::Queued, "QUEUED"),
        (WorkflowStatus::Done, "DONE"),
    ];

    // Clear layout tracking
    state.row_to_flat_idx.clear();
    state.group_regions.clear();

    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_to_visual: Vec<usize> = Vec::new();
    // visual_row tracks the row offset within the inner area (0-based)
    let mut visual_row: usize = 0;

    for (status, label) in groups {
        let group_wts: Vec<(usize, &WorktreeInfo)> = state.worktrees.iter()
            .enumerate()
            .filter(|(_, wt)| wt.workflow_status == *status)
            .collect();

        if group_wts.is_empty() {
            continue;
        }

        // Group header - record start_row as absolute screen row
        let group_start_row = list_area.y + visual_row as u16;
        items.push(ListItem::new(Line::from(Span::styled(
            format!(" {}", label),
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
        ))));
        // Ensure row_to_flat_idx is long enough, map header row to None
        let abs_row = (list_area.y + visual_row as u16) as usize;
        if state.row_to_flat_idx.len() <= abs_row {
            state.row_to_flat_idx.resize(abs_row + 1, None);
        }
        visual_row += 1;

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
                    WorkflowStatus::InReview => Span::styled("\u{e728} ", Style::default().fg(Color::Cyan)),
                    WorkflowStatus::InProgress => Span::styled("\u{00B7} ", Style::default().fg(Color::DarkGray)),
                }
            };

            let display_name = if let Some(ref repo) = wt.repo_name {
                let name = wt.short_name.as_deref().unwrap_or(&wt.branch);
                format!("{}/{}", repo, name)
            } else {
                wt.short_name.as_deref().unwrap_or(&wt.branch).to_string()
            };
            let name_style = if is_selected && focused {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Build row 1: icon + name + tags
            let mut line1_spans = vec![
                Span::raw("  "),
                icon,
                Span::styled(display_name, name_style),
            ];

            // Dirty tag
            if wt.is_dirty {
                line1_spans.push(Span::styled(" M", Style::default().fg(Color::Yellow)));
            }

            // PR tag
            if let Some((pr_num, ref pr_state)) = wt.pr {
                let (pr_color, pr_suffix) = match pr_state {
                    PrState::Open => (Color::Green, ""),
                    PrState::Draft => (Color::Yellow, " Draft"),
                    PrState::Merged => (Color::Magenta, " Merged"),
                    PrState::Closed => (Color::Red, " Closed"),
                };
                line1_spans.push(Span::styled(
                    format!(" #{}{}", pr_num, pr_suffix),
                    Style::default().fg(pr_color),
                ));
            }

            let line1 = Line::from(line1_spans);

            // Build row 2: commit message + stats
            let commit_msg = wt.commit_message.as_deref().unwrap_or("");
            let truncated_msg: String = if commit_msg.len() > 40 {
                format!("{}…", &commit_msg[..39])
            } else {
                commit_msg.to_string()
            };

            let mut line2_spans = vec![
                Span::styled(
                    format!("    {}", truncated_msg),
                    Style::default().fg(Color::DarkGray),
                ),
            ];

            if wt.ahead > 0 {
                line2_spans.push(Span::styled(
                    format!(" \u{2191}{}", wt.ahead),
                    Style::default().fg(Color::Cyan),
                ));
            }
            if wt.behind > 0 {
                line2_spans.push(Span::styled(
                    format!(" \u{2193}{}", wt.behind),
                    Style::default().fg(Color::Yellow),
                ));
            }
            if wt.last_commit_age_secs < u64::MAX {
                line2_spans.push(Span::styled(
                    format!(" {}", format_age(wt.last_commit_age_secs)),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            let line2 = Line::from(line2_spans);

            flat_to_visual.push(items.len());
            items.push(ListItem::new(vec![line1, line2]));
            // Track both rows -> flat_idx mapping
            let abs_row1 = (list_area.y + visual_row as u16) as usize;
            if state.row_to_flat_idx.len() <= abs_row1 {
                state.row_to_flat_idx.resize(abs_row1 + 1, None);
            }
            state.row_to_flat_idx[abs_row1] = Some(*flat_idx);
            visual_row += 1;

            let abs_row2 = (list_area.y + visual_row as u16) as usize;
            if state.row_to_flat_idx.len() <= abs_row2 {
                state.row_to_flat_idx.resize(abs_row2 + 1, None);
            }
            state.row_to_flat_idx[abs_row2] = Some(*flat_idx);
            visual_row += 1;
        }

        // Record group region (start_row .. current row)
        let group_end_row = list_area.y + visual_row as u16;
        state.group_regions.push((*status, group_start_row, group_end_row));

        items.push(ListItem::new(Line::from("")));
        visual_row += 1;
    }

    let mut list_state = ListState::default();
    if let Some(&visual_idx) = flat_to_visual.get(state.selected) {
        list_state.select(Some(visual_idx));
    }

    let highlight = if focused {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };
    let list = List::new(items).highlight_style(highlight);

    StatefulWidget::render(list, list_area, buf, &mut list_state);

    // Render dialog overlay at bottom of sidebar
    match dialog {
        Dialog::CreateInput { input, active_field, archived, selected_archived, repo_names, selected_repo, .. } => {
            let has_archived = !archived.is_empty();
            let has_repo = !repo_names.is_empty();
            let repo_row_height: u16 = if has_repo { 1 } else { 0 };
            let dialog_height: u16 = if has_archived { 5 } else { 3 } + repo_row_height;
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

            // Repo row (multi-repo mode only)
            if has_repo {
                let repo_style = if *active_field == DialogField::Repo {
                    Style::default().fg(Color::Cyan).bg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White).bg(Color::DarkGray)
                };
                let repo_display = repo_names.get(*selected_repo)
                    .map(|(name, _)| name.as_str())
                    .unwrap_or("");
                let repo_prompt = Line::from(vec![
                    Span::styled(" Repo:   ", Style::default().fg(Color::White).bg(Color::DarkGray)),
                    Span::styled(format!("‹ {} ›", repo_display), repo_style),
                ]);
                buf.set_line(dialog_area.x, row, &repo_prompt, dialog_area.width);
                row += 1;
            }

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
        Dialog::ArchiveConfirm(_idx, _name, display_name) => {
            let name = display_name;
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

    // Fixed footer bar
    let wt_count = state.worktrees.len();
    let new_style = if focused {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let hint_style = Style::default().fg(Color::DarkGray);
    let count_str = format!("{} wt", wt_count);
    let footer_line = Line::from(vec![
        Span::styled(" [+]New", new_style),
        Span::styled("  Archive  Status", hint_style),
    ]);
    buf.set_line(footer_area.x, footer_area.y, &footer_line, footer_area.width);

    let count_span = Span::styled(&count_str, hint_style);
    let count_x = footer_area.right().saturating_sub(count_str.len() as u16 + 1);
    buf.set_line(count_x, footer_area.y, &Line::from(count_span), footer_area.width);
}
