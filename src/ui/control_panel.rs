use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget};

use crate::app::{Dialog, DialogField};
use crate::github::PrState;
use crate::persistence::WorkflowStatus;
use crate::ui::theme::THEME;
use crate::worktree::{WorktreeInfo, format_age};

pub struct ControlPanelState {
    pub selected: usize,
    pub worktrees: Vec<WorktreeInfo>,
    pub row_to_flat_idx: Vec<Option<usize>>,
    pub group_regions: Vec<(WorkflowStatus, u16, u16)>, // (status, start_row, end_row)
}

#[allow(clippy::too_many_arguments)]
pub fn render_control_panel(
    state: &mut ControlPanelState,
    dialog: &Dialog,
    area: Rect,
    buf: &mut Buffer,
    focused: bool,
    spinner_frame: u8,
    pty_last_outputs: &std::collections::HashMap<std::path::PathBuf, u64>,
    filter: &Option<String>,
) {
    let border_style = if focused {
        Style::default().fg(THEME.aqua)
    } else {
        Style::default().fg(THEME.bg3)
    };

    let block = Block::default()
        .title(" \u{1F332} arbor ")
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

    let mut list_area = list_area;
    if let Some(ref filter_text) = filter {
        let filter_line = Line::from(vec![
            Span::styled(" \u{2315} ", Style::default().fg(THEME.aqua)),
            Span::styled(format!("{}_", filter_text), Style::default().fg(THEME.aqua)),
        ]);
        buf.set_line(list_area.x, list_area.y, &filter_line, list_area.width);
        list_area = Rect {
            y: list_area.y + 1,
            height: list_area.height.saturating_sub(1),
            ..list_area
        };
    }

    let now_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let groups: &[(WorkflowStatus, &str)] = &[
        (WorkflowStatus::InProgress, "\u{1F33F} IN PROGRESS"),
        (WorkflowStatus::InReview, "\u{1F343} IN REVIEW"),
        (WorkflowStatus::Queued, "\u{1F331} QUEUED"),
        (WorkflowStatus::Backlog, "\u{1F342} BACKLOG"),
    ];

    // Clear layout tracking
    state.row_to_flat_idx.clear();
    state.group_regions.clear();

    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_to_visual: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    // visual_row tracks the row offset within the inner area (0-based)
    let mut visual_row: usize = 0;

    for (status, label) in groups {
        let group_wts: Vec<(usize, &WorktreeInfo)> = state.worktrees.iter()
            .enumerate()
            .filter(|(_, wt)| wt.workflow_status == *status)
            .filter(|(_, wt)| {
                if let Some(ref f) = filter {
                    wt.branch.to_lowercase().contains(&f.to_lowercase())
                } else {
                    true
                }
            })
            .collect();

        if group_wts.is_empty() {
            continue;
        }

        // Group header - record start_row as absolute screen row
        let group_start_row = list_area.y + visual_row as u16;
        let count = group_wts.len();
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!(" {}", label),
                Style::default().fg(THEME.grey0).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", count),
                Style::default().fg(THEME.grey1),
            ),
        ])));
        // Ensure row_to_flat_idx is long enough, map header row to None
        let abs_row = (list_area.y + visual_row as u16) as usize;
        if state.row_to_flat_idx.len() <= abs_row {
            state.row_to_flat_idx.resize(abs_row + 1, None);
        }
        visual_row += 1;

        for (flat_idx, wt) in &group_wts {
            let is_selected = *flat_idx == state.selected;
            let position_in_group = group_wts.iter().position(|(idx, _)| *idx == *flat_idx).unwrap();
            let is_last = position_in_group == group_wts.len() - 1;

            // Activity icon
            let icon = if let Some(&last_output) = pty_last_outputs.get(&wt.path) {
                if last_output > 0 && now_millis.saturating_sub(last_output) < 500 {
                    let frames = ['\u{280B}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283C}', '\u{2834}', '\u{2826}', '\u{2827}', '\u{2807}', '\u{280F}'];
                    let frame_char = frames[(spinner_frame % 10) as usize];
                    Span::styled(format!("{} ", frame_char), Style::default().fg(THEME.aqua))
                } else {
                    Span::styled("! ", Style::default().fg(THEME.yellow))
                }
            } else {
                match wt.workflow_status {
                    WorkflowStatus::Backlog => Span::styled("\u{25B6} ", Style::default().fg(THEME.bg4)),
                    WorkflowStatus::Queued => Span::styled("! ", Style::default().fg(THEME.yellow)),
                    WorkflowStatus::InReview => Span::styled("\u{e728} ", Style::default().fg(THEME.aqua)),
                    WorkflowStatus::InProgress => Span::styled("\u{00B7} ", Style::default().fg(THEME.grey0)),
                }
            };

            let display_name = if let Some(ref repo) = wt.repo_name {
                let name = wt.short_name.as_deref().unwrap_or(&wt.branch);
                format!("{}/{}", repo, name)
            } else {
                wt.short_name.as_deref().unwrap_or(&wt.branch).to_string()
            };
            let name_style = if is_selected && focused {
                Style::default().fg(THEME.aqua).add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(THEME.grey0).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(THEME.fg)
            };

            // Tree gutter characters
            let trunk_color = if is_selected && focused { THEME.green } else { THEME.bg2 };
            let fork = if is_last { "\u{2514}\u{2500}" } else { "\u{251C}\u{2500}" };
            let leaf_color = if is_selected && focused {
                THEME.green
            } else {
                match wt.workflow_status {
                    WorkflowStatus::InReview => THEME.blue,
                    WorkflowStatus::Queued => THEME.yellow,
                    _ => THEME.bg3,
                }
            };

            // Build row 1: tree gutter + icon + name + tags
            let mut line1_spans = vec![
                Span::styled(fork, Style::default().fg(trunk_color)),
                Span::styled("\u{25CF}", Style::default().fg(leaf_color)),
                icon,
                Span::styled(display_name, name_style),
            ];

            // Dirty tag
            if wt.is_dirty {
                line1_spans.push(Span::styled(" M", Style::default().fg(THEME.yellow)));
            }

            // PR tag
            if let Some((pr_num, ref pr_state)) = wt.pr {
                let (pr_color, pr_suffix) = match pr_state {
                    PrState::Open => (THEME.green, ""),
                    PrState::Draft => (THEME.yellow, " Draft"),
                    PrState::Merged => (THEME.purple, " Merged"),
                    PrState::Closed => (THEME.red, " Closed"),
                };
                line1_spans.push(Span::styled(
                    format!(" #{}{}", pr_num, pr_suffix),
                    Style::default().fg(pr_color),
                ));
            }

            let line1 = Line::from(line1_spans);

            // Build row 2: commit message + stats
            let trunk_cont = if is_last { "   " } else { "\u{2502}  " };
            let commit_msg = wt.commit_message.as_deref().unwrap_or("");
            let truncated_msg: String = if commit_msg.len() > 36 {
                format!("{}…", &commit_msg[..35])
            } else {
                commit_msg.to_string()
            };

            let mut line2_spans = vec![
                Span::styled(trunk_cont, Style::default().fg(trunk_color)),
                Span::styled(
                    truncated_msg,
                    Style::default().fg(THEME.grey0),
                ),
            ];

            if wt.ahead > 0 {
                line2_spans.push(Span::styled(
                    format!(" \u{2191}{}", wt.ahead),
                    Style::default().fg(THEME.aqua),
                ));
            }
            if wt.behind > 0 {
                line2_spans.push(Span::styled(
                    format!(" \u{2193}{}", wt.behind),
                    Style::default().fg(THEME.yellow),
                ));
            }
            if wt.last_commit_age_secs < u64::MAX {
                line2_spans.push(Span::styled(
                    format!(" {}", format_age(wt.last_commit_age_secs)),
                    Style::default().fg(THEME.grey0),
                ));
            }

            let line2 = Line::from(line2_spans);

            flat_to_visual.insert(*flat_idx, items.len());
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
    if let Some(&visual_idx) = flat_to_visual.get(&state.selected) {
        list_state.select(Some(visual_idx));
    }

    let highlight = if focused {
        Style::default().bg(THEME.bg3)
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
                    buf[(x, y)].set_char(' ').set_style(Style::default().bg(THEME.bg3));
                }
            }
            let mut row = dialog_area.y;

            let title_line = Line::from(Span::styled(
                " New worktree",
                Style::default().fg(THEME.aqua).bg(THEME.bg3).add_modifier(Modifier::BOLD),
            ));
            buf.set_line(dialog_area.x, row, &title_line, dialog_area.width);
            row += 1;

            // Repo row (multi-repo mode only)
            if has_repo {
                let repo_style = if *active_field == DialogField::Repo {
                    Style::default().fg(THEME.aqua).bg(THEME.bg3)
                } else {
                    Style::default().fg(THEME.fg).bg(THEME.bg3)
                };
                let repo_display = repo_names.get(*selected_repo)
                    .map(|(name, _)| name.as_str())
                    .unwrap_or("");
                let repo_prompt = Line::from(vec![
                    Span::styled(" Repo:   ", Style::default().fg(THEME.fg).bg(THEME.bg3)),
                    Span::styled(format!("‹ {} ›", repo_display), repo_style),
                ]);
                buf.set_line(dialog_area.x, row, &repo_prompt, dialog_area.width);
                row += 1;
            }

            let input_style = if selected_archived.is_some() {
                Style::default().fg(THEME.yellow).bg(THEME.bg3)
            } else if *active_field == DialogField::Branch {
                Style::default().fg(THEME.aqua).bg(THEME.bg3)
            } else {
                Style::default().fg(THEME.fg).bg(THEME.bg3)
            };
            let prompt = Line::from(vec![
                Span::styled(" Branch: ", Style::default().fg(THEME.fg).bg(THEME.bg3)),
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
                    Style::default().fg(THEME.yellow).bg(THEME.bg3),
                ));
                buf.set_line(dialog_area.x, row, &archived_line, dialog_area.width);
                row += 1;

                // Show current archived selection if cycling
                if let Some(idx) = selected_archived {
                    let preview = format!(" \u{2192} {}", archived[*idx]);
                    let preview_line = Line::from(Span::styled(
                        preview,
                        Style::default().fg(THEME.yellow).bg(THEME.bg3).add_modifier(Modifier::BOLD),
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
                Style::default().fg(THEME.grey1).bg(THEME.bg3),
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
                    buf[(x, y)].set_char(' ').set_style(Style::default().bg(THEME.bg3));
                }
            }
            let title = Line::from(Span::styled(
                " Archive worktree",
                Style::default().fg(THEME.yellow).bg(THEME.bg3).add_modifier(Modifier::BOLD),
            ));
            buf.set_line(dialog_area.x, dialog_area.y, &title, dialog_area.width);

            let prompt = Line::from(Span::styled(
                format!(" Remove {}? (y/n)", name),
                Style::default().fg(THEME.fg).bg(THEME.bg3),
            ));
            buf.set_line(dialog_area.x, dialog_area.y + 1, &prompt, dialog_area.width);

            let hint = Line::from(Span::styled(
                " Branch kept \u{00B7} restore with n",
                Style::default().fg(THEME.grey1).bg(THEME.bg3),
            ));
            buf.set_line(dialog_area.x, dialog_area.y + 2, &hint, dialog_area.width);
        }
        Dialog::None => {}
    }

    // Fixed footer bar
    let wt_count = state.worktrees.len();
    let new_style = if focused {
        Style::default().fg(THEME.green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(THEME.grey0)
    };
    let hint_style = Style::default().fg(THEME.grey0);
    let count_str = format!("\u{1F332} {} worktrees", wt_count);
    let footer_line = Line::from(vec![
        Span::styled(" [+]New", new_style),
        Span::styled("  Archive", hint_style),
    ]);
    buf.set_line(footer_area.x, footer_area.y, &footer_line, footer_area.width);

    let count_span = Span::styled(&count_str, hint_style);
    let count_x = footer_area.right().saturating_sub(count_str.len() as u16 + 1);
    buf.set_line(count_x, footer_area.y, &Line::from(count_span), footer_area.width);
}
