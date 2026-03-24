use anyhow::Result;
use crossterm::event::{self, Event, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::DefaultTerminal;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::keys::{self, Action, Focus};
use crate::persistence::{ArborConfig, WorkflowStatus};
use crate::pty::PtySession;
use crate::ui;
use crate::ui::ControlPanelState;
use crate::ui::TerminalWidget;
use crate::worktree::WorktreeManager;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogField {
    Branch,
    Name,
}

#[derive(Debug)]
pub enum Dialog {
    None,
    CreateInput {
        input: String,
        short_name: String,
        active_field: DialogField,
        archived: Vec<String>,     // branches that can be restored
        selected_archived: Option<usize>,
    },
    ArchiveConfirm(usize, String), // index, worktree name
}

struct DragState {
    worktree_idx: usize,
    dragging: bool,
}

pub struct App {
    worktree_mgr: WorktreeManager,
    pty_sessions: HashMap<PathBuf, PtySession>,
    active_worktree: Option<PathBuf>,
    pub focus: Focus,
    pub sidebar_state: ControlPanelState,
    pub dialog: Dialog,
    sidebar_width: u16,
    spinner_frame: u8,
    should_quit: bool,
    config: ArborConfig,
    repo_root: PathBuf,
    drag_state: Option<DragState>,
}

impl App {
    pub fn new(repo_path: &std::path::Path) -> Result<Self> {
        let worktree_mgr = WorktreeManager::open(repo_path)?;
        let repo_root = worktree_mgr.repo_root().to_path_buf();
        let config = ArborConfig::load(&repo_root);

        let mut worktrees = worktree_mgr.list()?;
        for wt in &mut worktrees {
            if wt.is_main {
                wt.workflow_status = WorkflowStatus::InProgress;
            } else if let Some(wt_config) = config.worktrees.get(&wt.name) {
                wt.workflow_status = wt_config.status;
                wt.short_name = wt_config.short_name.clone();
            }
        }

        let sidebar_state = ControlPanelState {
            selected: 0,
            worktrees,
            show_plus: true,
            row_to_flat_idx: Vec::new(),
            group_regions: Vec::new(),
        };

        let mut app = Self {
            worktree_mgr,
            pty_sessions: HashMap::new(),
            active_worktree: None,
            focus: Focus::Terminal,
            sidebar_state,
            dialog: Dialog::None,
            sidebar_width: 30,
            spinner_frame: 0,
            should_quit: false,
            config,
            repo_root,
            drag_state: None,
        };
        app.sidebar_width = app.calculate_panel_width();
        Ok(app)
    }

    pub fn panel_width(&self) -> u16 {
        self.sidebar_width
    }

    fn calculate_panel_width(&self) -> u16 {
        let max_name_len = self.sidebar_state.worktrees.iter()
            .map(|wt| {
                let display = wt.short_name.as_deref().unwrap_or(&wt.branch);
                display.len()
            })
            .max()
            .unwrap_or(0);
        // Padding: 2 (border) + 2 (indent) + 2 (icon + space) + 2 (right padding) = 8
        let width = (max_name_len + 8) as u16;
        width.clamp(20, 60)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let size = terminal.size()?;
        self.ensure_pty_for_selected(size.height, size.width)?;

        while !self.should_quit {
            terminal.draw(|frame| {
                // Top-level: main area + status bar
                let outer = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(frame.area());

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(self.sidebar_width),
                        Constraint::Min(1),
                    ])
                    .split(outer[0]);

                let pty_last_outputs: std::collections::HashMap<std::path::PathBuf, u64> = self.pty_sessions.iter()
                    .map(|(k, v)| (k.clone(), v.last_output_millis()))
                    .collect();

                ui::render_control_panel(
                    &mut self.sidebar_state,
                    &self.dialog,
                    chunks[0],
                    frame.buffer_mut(),
                    self.focus == Focus::Sidebar,
                    self.spinner_frame,
                    &pty_last_outputs,
                );

                // Split right panel into header + terminal
                let right_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Min(1)])
                    .split(chunks[1]);

                // Render header
                if self.sidebar_state.selected < self.sidebar_state.worktrees.len() {
                    let wt = &self.sidebar_state.worktrees[self.sidebar_state.selected];
                    let header = Line::from(vec![
                        Span::styled(
                            format!(" {} ", wt.path.display()),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("⎇ {} ", wt.branch),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]);
                    frame.render_widget(header, right_chunks[0]);
                }

                // Render terminal in remaining space (dimmed when sidebar focused)
                if let Some(ref key) = self.active_worktree {
                    if let Some(pty) = self.pty_sessions.get(key) {
                        let term_widget = TerminalWidget::new(pty.screen())
                            .dimmed(self.focus == Focus::Sidebar);
                        frame.render_widget(term_widget, right_chunks[1]);
                    }
                }

                // Render status bar
                let status_line = self.build_status_line(outer[1].width);
                frame.render_widget(status_line, outer[1]);
            })?;

            self.spinner_frame = self.spinner_frame.wrapping_add(1);

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        // Dialogs consume raw key events first
                        if self.handle_dialog_key(key)? {
                            continue;
                        }
                        let action = keys::handle_key(key, &self.focus);
                        self.handle_action(action)?;
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse(mouse)?;
                    }
                    Event::Resize(cols, rows) => {
                        if let Some(ref key) = self.active_worktree {
                            if let Some(pty) = self.pty_sessions.get(key) {
                                let terminal_cols = cols.saturating_sub(self.sidebar_width);
                                // Subtract 2 for status bar and header
                                let terminal_rows = rows.saturating_sub(2);
                                pty.resize(terminal_rows, terminal_cols)?;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn build_status_line(&self, width: u16) -> Line<'static> {
        let bg = Color::DarkGray;
        let fg = Color::White;
        let key_style = Style::default().fg(Color::Cyan).bg(bg).add_modifier(Modifier::BOLD);
        let label_style = Style::default().fg(fg).bg(bg);
        let sep_style = Style::default().fg(Color::Gray).bg(bg);

        let sep = Span::styled(" │ ", sep_style);

        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled(" ", label_style));

        match self.focus {
            Focus::Terminal => {
                spans.push(Span::styled("Shift+←", key_style));
                spans.push(Span::styled(" sidebar", label_style));
            }
            Focus::Sidebar => {
                spans.push(Span::styled("j/k", key_style));
                spans.push(Span::styled(" navigate", label_style));
                spans.push(sep.clone());
                spans.push(Span::styled("Enter", key_style));
                spans.push(Span::styled(" select", label_style));
                spans.push(sep.clone());
                spans.push(Span::styled("s", key_style));
                spans.push(Span::styled(" status", label_style));
                spans.push(sep.clone());
                spans.push(Span::styled("n", key_style));
                spans.push(Span::styled(" new", label_style));
                spans.push(sep.clone());
                spans.push(Span::styled("a", key_style));
                spans.push(Span::styled(" archive", label_style));
                spans.push(sep.clone());
                spans.push(Span::styled("Shift+→", key_style));
                spans.push(Span::styled(" terminal", label_style));
                spans.push(sep.clone());
                spans.push(Span::styled("q", key_style));
                spans.push(Span::styled(" quit", label_style));
            }
        }

        // Pad the rest of the status bar
        let used: usize = spans.iter().map(|s| s.content.len()).sum();
        let remaining = (width as usize).saturating_sub(used);
        if remaining > 0 {
            spans.push(Span::styled(" ".repeat(remaining), Style::default().bg(bg)));
        }

        Line::from(spans)
    }

    pub fn handle_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::ToggleFocus => {
                self.focus = match self.focus {
                    Focus::Sidebar => Focus::Terminal,
                    Focus::Terminal => Focus::Sidebar,
                };
                if self.focus == Focus::Sidebar {
                    self.sidebar_state.worktrees = self.worktree_mgr.list()?;
                }
            }
            Action::FocusSidebar => {
                self.sidebar_state.worktrees = self.worktree_mgr.list()?;
                self.focus = Focus::Sidebar;
            }
            Action::FocusTerminal => self.focus = Focus::Terminal,
            Action::SidebarUp => {
                if self.sidebar_state.selected > 0 {
                    self.sidebar_state.selected -= 1;
                }
            }
            Action::SidebarDown => {
                let max = self.sidebar_state.worktrees.len();
                if self.sidebar_state.selected < max {
                    self.sidebar_state.selected += 1;
                }
            }
            Action::SidebarSelect => {
                if self.sidebar_state.selected < self.sidebar_state.worktrees.len() {
                    let size = crossterm::terminal::size()?;
                    self.ensure_pty_for_selected(size.1, size.0)?;
                    self.focus = Focus::Terminal;
                } else {
                    self.handle_action(Action::SidebarCreate)?;
                }
            }
            Action::SidebarCreate => {
                let archived = self.worktree_mgr.archived_branches().unwrap_or_default();
                self.dialog = Dialog::CreateInput {
                    input: String::new(),
                    short_name: String::new(),
                    active_field: DialogField::Branch,
                    archived,
                    selected_archived: None,
                };
            }
            Action::SidebarArchive => {
                let idx = self.sidebar_state.selected;
                if idx < self.sidebar_state.worktrees.len() {
                    let wt = &self.sidebar_state.worktrees[idx];
                    if !wt.is_main {
                        let name = wt.name.clone();
                        self.dialog = Dialog::ArchiveConfirm(idx, name);
                    }
                }
            }
            Action::StatusCycle => {
                let idx = self.sidebar_state.selected;
                if idx < self.sidebar_state.worktrees.len() {
                    let wt = &mut self.sidebar_state.worktrees[idx];
                    if !wt.is_main {
                        wt.workflow_status = wt.workflow_status.next();
                        let entry = self.config.worktrees
                            .entry(wt.name.clone())
                            .or_default();
                        entry.status = wt.workflow_status;
                        let _ = self.config.save(&self.repo_root);
                    }
                }
            }
            Action::TerminalInput(key) => {
                if let Some(ref active) = self.active_worktree {
                    if let Some(ref mut pty) = self.pty_sessions.get_mut(active) {
                        let bytes = key_to_bytes(key);
                        if !bytes.is_empty() {
                            pty.write(&bytes)?;
                        }
                    }
                }
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
        Ok(())
    }

    /// Handle raw key events for active dialogs. Returns true if the dialog consumed the event.
    pub fn handle_dialog_key(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        use crossterm::event::KeyCode;

        match &mut self.dialog {
            Dialog::CreateInput { ref mut input, ref mut short_name, ref mut active_field, ref archived, ref mut selected_archived } => {
                match key.code {
                    KeyCode::Enter => {
                        // Use selected archived branch or typed input
                        let branch = if let Some(idx) = selected_archived {
                            archived[*idx].clone()
                        } else if !input.is_empty() {
                            input.clone()
                        } else {
                            return Ok(true);
                        };
                        let sn = if short_name.is_empty() { None } else { Some(short_name.clone()) };
                        self.worktree_mgr.create(&branch)?;
                        self.sidebar_state.worktrees = self.worktree_mgr.list()?;
                        // Persist short_name
                        let entry = self.config.worktrees.entry(branch.clone()).or_default();
                        if let Some(ref name) = sn {
                            entry.short_name = Some(name.clone());
                        }
                        let _ = self.config.save(&self.repo_root);
                        // Select the newly created worktree
                        if let Some(idx) = self.sidebar_state.worktrees.iter()
                            .position(|w| w.branch == branch)
                        {
                            self.sidebar_state.selected = idx;
                        }
                        self.dialog = Dialog::None;
                        let size = crossterm::terminal::size()?;
                        self.ensure_pty_for_selected(size.1, size.0)?;
                        self.focus = Focus::Terminal;
                    }
                    KeyCode::Down => { *active_field = DialogField::Name; }
                    KeyCode::Up => { *active_field = DialogField::Branch; }
                    KeyCode::Tab if !archived.is_empty() && *active_field == DialogField::Branch => {
                        // Cycle through archived branches
                        *selected_archived = Some(match selected_archived {
                            Some(idx) => (*idx + 1) % archived.len(),
                            None => 0,
                        });
                        *input = archived[selected_archived.unwrap()].clone();
                    }
                    KeyCode::BackTab if !archived.is_empty() && *active_field == DialogField::Branch => {
                        // Cycle backwards through archived branches
                        *selected_archived = Some(match selected_archived {
                            Some(0) | None => archived.len() - 1,
                            Some(idx) => *idx - 1,
                        });
                        *input = archived[selected_archived.unwrap()].clone();
                    }
                    KeyCode::Esc => self.dialog = Dialog::None,
                    KeyCode::Char(c) => {
                        *selected_archived = None;
                        match active_field {
                            DialogField::Branch => input.push(c),
                            DialogField::Name => {
                                if short_name.len() < 20 { short_name.push(c); }
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        *selected_archived = None;
                        match active_field {
                            DialogField::Branch => { input.pop(); }
                            DialogField::Name => { short_name.pop(); }
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            Dialog::ArchiveConfirm(_idx, ref name) => {
                let name = name.clone();
                match key.code {
                    KeyCode::Char('y') => {
                        // Remove PTY session for this worktree (dropping it kills the child)
                        let wt = &self.sidebar_state.worktrees[self.sidebar_state.selected];
                        let key = wt.path.clone();
                        self.pty_sessions.remove(&key);
                        if self.active_worktree.as_ref() == Some(&key) {
                            self.active_worktree = None;
                        }

                        self.worktree_mgr.delete(&name, false)?;
                        self.sidebar_state.worktrees = self.worktree_mgr.list()?;
                        self.sidebar_state.selected = 0;
                        self.dialog = Dialog::None;
                        let size = crossterm::terminal::size()?;
                        self.ensure_pty_for_selected(size.1, size.0)?;
                    }
                    KeyCode::Char('n') | KeyCode::Esc => self.dialog = Dialog::None,
                    _ => {}
                }
                Ok(true)
            }
            Dialog::None => Ok(false),
        }
    }

    pub fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Moved => {}
            MouseEventKind::Down(_) => {
                if mouse.column < self.sidebar_width {
                    if self.focus != Focus::Sidebar {
                        self.sidebar_state.worktrees = self.worktree_mgr.list()?;
                        self.focus = Focus::Sidebar;
                    }
                    // Look up which worktree was clicked via row_to_flat_idx
                    let row = mouse.row as usize;
                    let clicked_idx = if row < self.sidebar_state.row_to_flat_idx.len() {
                        self.sidebar_state.row_to_flat_idx[row]
                    } else {
                        None
                    };
                    if let Some(idx) = clicked_idx {
                        self.sidebar_state.selected = idx;
                        self.focus = Focus::Sidebar;
                        // Start drag if not main worktree
                        if idx < self.sidebar_state.worktrees.len()
                            && !self.sidebar_state.worktrees[idx].is_main
                        {
                            self.drag_state = Some(DragState {
                                worktree_idx: idx,
                                dragging: false,
                            });
                        }
                    }
                } else {
                    self.focus = Focus::Terminal;
                }
            }
            MouseEventKind::Drag(_) => {
                if let Some(ref mut ds) = self.drag_state {
                    ds.dragging = true;
                }
            }
            MouseEventKind::Up(_) => {
                if let Some(ds) = self.drag_state.take() {
                    if ds.dragging {
                        // Find the target group based on mouse row
                        let row = mouse.row;
                        let target_status = self.sidebar_state.group_regions.iter()
                            .find(|(_status, start, end)| row >= *start && row < *end)
                            .map(|(status, _, _)| *status);
                        if let Some(new_status) = target_status {
                            let idx = ds.worktree_idx;
                            if idx < self.sidebar_state.worktrees.len() {
                                let wt = &mut self.sidebar_state.worktrees[idx];
                                if !wt.is_main && wt.workflow_status != new_status {
                                    wt.workflow_status = new_status;
                                    let entry = self.config.worktrees
                                        .entry(wt.name.clone())
                                        .or_default();
                                    entry.status = new_status;
                                    let _ = self.config.save(&self.repo_root);
                                }
                            }
                        }
                    }
                    // If not dragging, it was a click — already handled on Down
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn ensure_pty_for_selected(&mut self, rows: u16, cols: u16) -> Result<()> {
        let wt = &self.sidebar_state.worktrees[self.sidebar_state.selected];
        let key = wt.path.clone();
        if !self.pty_sessions.contains_key(&key) {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            let terminal_cols = cols.saturating_sub(self.sidebar_width);
            // Subtract 2 for status bar and header
            let terminal_rows = rows.saturating_sub(2);
            let session = PtySession::spawn(&shell, &[], terminal_rows, terminal_cols, &wt.path)?;
            self.pty_sessions.insert(key.clone(), session);
        }
        self.active_worktree = Some(key);
        Ok(())
    }
}

fn key_to_bytes(key: crossterm::event::KeyEvent) -> Vec<u8> {
    use crossterm::event::{KeyCode, KeyModifiers};
    let mut bytes = Vec::new();

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            let ctrl = (c as u8).wrapping_sub(b'a').wrapping_add(1);
            bytes.push(ctrl);
            return bytes;
        }
    }

    match key.code {
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(s.as_bytes());
        }
        KeyCode::Enter => bytes.push(b'\r'),
        KeyCode::Backspace => bytes.push(0x7f),
        KeyCode::Tab => bytes.push(b'\t'),
        KeyCode::Esc => bytes.push(0x1b),
        KeyCode::Up => bytes.extend_from_slice(b"\x1b[A"),
        KeyCode::Down => bytes.extend_from_slice(b"\x1b[B"),
        KeyCode::Right => bytes.extend_from_slice(b"\x1b[C"),
        KeyCode::Left => bytes.extend_from_slice(b"\x1b[D"),
        KeyCode::Home => bytes.extend_from_slice(b"\x1b[H"),
        KeyCode::End => bytes.extend_from_slice(b"\x1b[F"),
        KeyCode::Delete => bytes.extend_from_slice(b"\x1b[3~"),
        _ => {}
    }
    bytes
}
