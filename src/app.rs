use anyhow::Result;
use crossterm::event::{self, Event, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::DefaultTerminal;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::github::SharedGitHubCache;
use crate::keys::{self, Action, Focus};
use crate::persistence::{ArborConfig, WorkflowStatus};
use crate::pty::PtySession;
use crate::ui;
use crate::ui::ControlPanelState;
use crate::worktree::{WorktreeInfo, WorktreeManager};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogField {
    Repo,
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
        repo_root: PathBuf,        // which repo to create in
        repo_names: Vec<(String, PathBuf)>, // (display_name, root_path) pairs; empty in single-repo mode
        selected_repo: usize,               // index into repo_names
    },
    ArchiveConfirm(usize, String, String), // index, worktree name, display name
}

struct DragState {
    worktree_idx: usize,
    dragging: bool,
}

pub struct App {
    managers: HashMap<PathBuf, WorktreeManager>,
    pty_sessions: HashMap<PathBuf, PtySession>,
    active_worktree: Option<PathBuf>,
    pub focus: Focus,
    pub sidebar_state: ControlPanelState,
    pub dialog: Dialog,
    sidebar_width: u16,
    spinner_frame: u8,
    should_quit: bool,
    configs: HashMap<PathBuf, ArborConfig>,
    scan_root: PathBuf,
    multi_repo: bool,
    drag_state: Option<DragState>,
    github_caches: HashMap<PathBuf, SharedGitHubCache>,
    scroll_offset: usize,
}

impl App {
    pub fn new(repo_path: &std::path::Path) -> Result<Self> {
        let (managers, multi_repo, scan_root) = if git2::Repository::discover(repo_path).is_ok() {
            // Single-repo mode
            let mgr = WorktreeManager::open(repo_path)?;
            let root = mgr.repo_root().to_path_buf();
            let mut map = HashMap::new();
            map.insert(root.clone(), mgr);
            (map, false, root)
        } else {
            // Multi-repo mode
            let discovered = crate::discovery::discover_repos(repo_path)?;
            let mut map = HashMap::new();
            for repo in discovered {
                match WorktreeManager::open(&repo.path) {
                    Ok(mgr) => {
                        let root = mgr.repo_root().to_path_buf();
                        map.insert(root, mgr);
                    }
                    Err(e) => {
                        eprintln!("arbor: skipping {}: {}", repo.name, e);
                    }
                }
            }
            if map.is_empty() {
                anyhow::bail!("No valid git repositories found");
            }
            let canonical = repo_path.canonicalize().unwrap_or_else(|_| repo_path.to_path_buf());
            (map, true, canonical)
        };

        // Build per-repo configs and github caches
        let mut configs = HashMap::new();
        let mut github_caches = HashMap::new();
        for root in managers.keys() {
            configs.insert(root.clone(), ArborConfig::load(root));
            github_caches.insert(root.clone(), SharedGitHubCache::new(root));
        }

        let mut app = Self {
            managers,
            pty_sessions: HashMap::new(),
            active_worktree: None,
            focus: Focus::Terminal,
            sidebar_state: ControlPanelState {
                selected: 0,
                worktrees: Vec::new(),
                show_plus: true,
                row_to_flat_idx: Vec::new(),
                group_regions: Vec::new(),
            },
            dialog: Dialog::None,
            sidebar_width: 30,
            spinner_frame: 0,
            should_quit: false,
            configs,
            scan_root,
            multi_repo,
            drag_state: None,
            github_caches,
            scroll_offset: 0,
        };
        app.sidebar_state.worktrees = app.build_worktree_list()?;
        app.sidebar_width = app.calculate_panel_width();
        Ok(app)
    }

    fn build_worktree_list(&self) -> Result<Vec<WorktreeInfo>> {
        let mut all = Vec::new();
        for (root, mgr) in &self.managers {
            let mut worktrees = match mgr.list() {
                Ok(wts) => wts,
                Err(e) => {
                    if self.multi_repo {
                        eprintln!("arbor: skipping {}: {}", root.display(), e);
                        continue;
                    }
                    return Err(e);
                }
            };
            let config = self.configs.get(root);

            // Tag with repo_name in multi-repo mode
            if self.multi_repo {
                let repo_name = root
                    .strip_prefix(&self.scan_root)
                    .unwrap_or(root)
                    .to_string_lossy()
                    .replace('\\', "/");
                for wt in &mut worktrees {
                    wt.repo_name = Some(repo_name.clone());
                }
            }

            // Apply config and PR auto-status
            for wt in &mut worktrees {
                if wt.is_main {
                    wt.workflow_status = WorkflowStatus::InProgress;
                } else if let Some(cfg) = config {
                    if let Some(wt_config) = cfg.worktrees.get(&wt.branch) {
                        wt.workflow_status = wt_config.status;
                        wt.short_name = wt_config.short_name.clone();
                    }
                }
                Self::apply_pr_auto_status(&self.github_caches, wt);
            }

            all.extend(worktrees);
        }
        Ok(all)
    }

    pub fn panel_width(&self) -> u16 {
        self.sidebar_width
    }

    fn calculate_panel_width(&self) -> u16 {
        let max_name_len = self.sidebar_state.worktrees.iter()
            .map(|wt| {
                let display = wt.short_name.as_deref().unwrap_or(&wt.branch);
                let mut len = display.len();
                // Account for repo name prefix in multi-repo mode
                if let Some(ref rn) = wt.repo_name {
                    len += rn.len() + 1; // "repo/branch"
                }
                // Account for ahead/behind indicators (e.g. " ↑3 ↓2")
                if wt.ahead > 0 { len += 3; }
                if wt.behind > 0 { len += 3; }
                // Account for PR badge (e.g. " #123")
                if let Some(cache) = self.github_caches.get(&wt.repo_root) {
                    if cache.get(&wt.branch).is_some() { len += 7; }
                }
                len
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

                // Split right panel into header + info bar + terminal
                let has_pr = self.sidebar_state.selected < self.sidebar_state.worktrees.len()
                    && {
                        let wt = &self.sidebar_state.worktrees[self.sidebar_state.selected];
                        let has_gh = self.github_caches.get(&wt.repo_root)
                            .and_then(|c| c.get(&wt.branch)).is_some();
                        has_gh || wt.ahead > 0 || wt.behind > 0
                    };
                let info_bar_height = if has_pr { 1 } else { 0 };

                let right_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(info_bar_height),
                        Constraint::Min(1),
                    ])
                    .split(chunks[1]);

                // Render header
                if self.sidebar_state.selected < self.sidebar_state.worktrees.len() {
                    let wt = &self.sidebar_state.worktrees[self.sidebar_state.selected];
                    let mut header_spans = vec![
                        Span::styled(
                            format!(" {} ", wt.path.display()),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("⎇ {} ", wt.branch),
                            Style::default().fg(Color::Cyan),
                        ),
                    ];
                    if let Some(ref repo_name) = wt.repo_name {
                        header_spans.push(Span::styled(
                            format!(" [{}]", repo_name),
                            Style::default().fg(Color::Yellow),
                        ));
                    }
                    let header = Line::from(header_spans);
                    frame.render_widget(header, right_chunks[0]);

                    // Render PR/git info bar
                    if has_pr {
                        let mut info_spans: Vec<Span> = vec![Span::raw(" ")];

                        if let Some(pr) = self.github_caches.get(&wt.repo_root).and_then(|c| c.get(&wt.branch)) {
                            let (icon, color) = match pr.state {
                                crate::github::PrState::Open => ("\u{e728}", Color::Green),
                                crate::github::PrState::Draft => ("\u{e728}", Color::Yellow),
                                crate::github::PrState::Merged => ("\u{e727}", Color::Magenta),
                                crate::github::PrState::Closed => ("\u{e728}", Color::Red),
                            };
                            let state_label = match pr.state {
                                crate::github::PrState::Open => "Open",
                                crate::github::PrState::Draft => "Draft",
                                crate::github::PrState::Merged => "Merged",
                                crate::github::PrState::Closed => "Closed",
                            };
                            info_spans.push(Span::styled(format!("{} ", icon), Style::default().fg(color)));
                            info_spans.push(Span::styled(format!("#{} ", pr.number), Style::default().fg(color).add_modifier(Modifier::BOLD)));
                            info_spans.push(Span::styled(format!("{} ", state_label), Style::default().fg(color)));
                            info_spans.push(Span::styled("· ", Style::default().fg(Color::DarkGray)));
                            info_spans.push(Span::styled(pr.url.clone(), Style::default().fg(Color::DarkGray)));
                        }

                        if wt.ahead > 0 || wt.behind > 0 {
                            if info_spans.len() > 1 {
                                info_spans.push(Span::styled("  ", Style::default()));
                            }
                            if wt.ahead > 0 {
                                info_spans.push(Span::styled(format!("↑{}", wt.ahead), Style::default().fg(Color::Cyan)));
                            }
                            if wt.behind > 0 {
                                info_spans.push(Span::styled(format!(" ↓{}", wt.behind), Style::default().fg(Color::Yellow)));
                            }
                        }

                        frame.render_widget(Line::from(info_spans), right_chunks[1]);
                    }
                }

                // Render terminal in remaining space (dimmed when sidebar focused)
                if let Some(ref key) = self.active_worktree {
                    if let Some(pty) = self.pty_sessions.get(key) {
                        let screen_arc = pty.screen();
                        let dimmed = self.focus == Focus::Sidebar;
                        let terminal_area = right_chunks[2];
                        let (cursor_row, cursor_col, clamped) = ui::render_terminal(
                            &screen_arc,
                            terminal_area,
                            frame.buffer_mut(),
                            dimmed,
                            self.scroll_offset,
                        );
                        // Sync our offset to the clamped value so we don't overshoot
                        self.scroll_offset = clamped;

                        if self.focus == Focus::Terminal && self.scroll_offset == 0 {
                            let cursor_x = terminal_area.x + cursor_col;
                            let cursor_y = terminal_area.y + cursor_row;
                            if cursor_x < terminal_area.right() && cursor_y < terminal_area.bottom() {
                                frame.set_cursor_position((cursor_x, cursor_y));
                            }
                        }
                    }
                }

                // Render status bar
                let status_line = self.build_status_line(outer[1].width);
                frame.render_widget(status_line, outer[1]);
            })?;

            self.spinner_frame = self.spinner_frame.wrapping_add(1);

            // Respawn shell if the active PTY's child process has exited
            if let Some(ref key) = self.active_worktree {
                if self.pty_sessions.get(key).is_some_and(|p| p.has_exited()) {
                    self.pty_sessions.remove(key);
                    let size = terminal.size()?;
                    self.ensure_pty_for_selected(size.height, size.width)?;
                }
            }

            if event::poll(Duration::from_millis(16))? {
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
                    Event::Paste(text) => {
                        if !self.handle_dialog_paste(&text)
                            && self.focus == Focus::Terminal
                        {
                            self.scroll_offset = 0;
                            if let Some(ref key) = self.active_worktree {
                                if let Some(pty) = self.pty_sessions.get_mut(key) {
                                    pty.write(text.as_bytes())?;
                                }
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
            }
            Action::FocusSidebar => {
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
                let repo_root = if self.sidebar_state.selected < self.sidebar_state.worktrees.len() {
                    self.sidebar_state.worktrees[self.sidebar_state.selected].repo_root.clone()
                } else if let Some(root) = self.managers.keys().next() {
                    root.clone()
                } else {
                    return Ok(());
                };
                let archived = self.managers.get(&repo_root)
                    .map(|mgr| mgr.archived_branches().unwrap_or_default())
                    .unwrap_or_default();

                // Build repo_names list for multi-repo mode
                let repo_names: Vec<(String, PathBuf)> = if self.multi_repo {
                    let mut names: Vec<(String, PathBuf)> = self.managers.keys()
                        .map(|root| {
                            let display = root
                                .strip_prefix(&self.scan_root)
                                .unwrap_or(root)
                                .to_string_lossy()
                                .replace('\\', "/");
                            (display, root.clone())
                        })
                        .collect();
                    names.sort_by(|a, b| a.0.cmp(&b.0));
                    names
                } else {
                    Vec::new()
                };
                let selected_repo = repo_names.iter()
                    .position(|(_, path)| *path == repo_root)
                    .unwrap_or(0);

                self.dialog = Dialog::CreateInput {
                    input: String::new(),
                    short_name: String::new(),
                    active_field: DialogField::Branch,
                    archived,
                    selected_archived: None,
                    repo_root,
                    repo_names,
                    selected_repo,
                };
            }
            Action::SidebarArchive => {
                let idx = self.sidebar_state.selected;
                if idx < self.sidebar_state.worktrees.len() {
                    let wt = &self.sidebar_state.worktrees[idx];
                    if !wt.is_main {
                        let name = wt.name.clone();
                        let display_name = if let Some(ref repo) = wt.repo_name {
                            format!("{}/{}", repo, name)
                        } else {
                            name.clone()
                        };
                        self.dialog = Dialog::ArchiveConfirm(idx, name, display_name);
                    }
                }
            }
            Action::StatusCycle => {
                let idx = self.sidebar_state.selected;
                if idx < self.sidebar_state.worktrees.len() {
                    let wt = &mut self.sidebar_state.worktrees[idx];
                    if !wt.is_main {
                        wt.workflow_status = wt.workflow_status.next();
                        let repo_root = wt.repo_root.clone();
                        let branch = wt.branch.clone();
                        let status = wt.workflow_status;
                        if let Some(config) = self.configs.get_mut(&repo_root) {
                            let entry = config.worktrees.entry(branch).or_default();
                            entry.status = status;
                            let _ = config.save(&repo_root);
                        }
                    }
                }
            }
            Action::TerminalInput(key) => {
                self.scroll_offset = 0;
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

        // Handle Repo cycling (Left/Right) separately to avoid borrow conflicts
        if let Dialog::CreateInput { ref active_field, ref repo_names, ref selected_repo, .. } = self.dialog {
            if *active_field == DialogField::Repo && !repo_names.is_empty() {
                let len = repo_names.len();
                let current = *selected_repo;
                let new_idx = match key.code {
                    KeyCode::Left => if current == 0 { len - 1 } else { current - 1 },
                    KeyCode::Right => (current + 1) % len,
                    _ => current,
                };
                if new_idx != current || matches!(key.code, KeyCode::Left | KeyCode::Right) {
                    let new_root = repo_names[new_idx].1.clone();
                    let new_archived = self.managers.get(&new_root)
                        .map(|mgr| mgr.archived_branches().unwrap_or_default())
                        .unwrap_or_default();
                    if let Dialog::CreateInput { ref mut selected_repo, ref mut repo_root, ref mut archived, ref mut selected_archived, .. } = self.dialog {
                        *selected_repo = new_idx;
                        *repo_root = new_root;
                        *archived = new_archived;
                        *selected_archived = None;
                    }
                    if matches!(key.code, KeyCode::Left | KeyCode::Right) {
                        return Ok(true);
                    }
                }
            }
        }

        match &mut self.dialog {
            Dialog::CreateInput { ref mut input, ref mut short_name, ref mut active_field, ref mut archived, ref mut selected_archived, ref mut repo_root, ref repo_names, .. } => {
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
                        let repo_root = repo_root.clone();
                        if let Some(mgr) = self.managers.get(&repo_root) {
                            if mgr.create(&branch).is_err() {
                                self.dialog = Dialog::None;
                                return Ok(true);
                            }
                        } else {
                            self.dialog = Dialog::None;
                            return Ok(true);
                        }
                        self.sidebar_state.worktrees = self.build_worktree_list()?;
                        if let Some(cache) = self.github_caches.get(&repo_root) {
                            cache.force_refresh(&repo_root);
                        }
                        // Persist short_name before applying config
                        if let Some(config) = self.configs.get_mut(&repo_root) {
                            let entry = config.worktrees.entry(branch.clone()).or_default();
                            if let Some(ref name) = sn {
                                entry.short_name = Some(name.clone());
                            }
                            let _ = config.save(&repo_root);
                        }
                        self.apply_config();
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
                    KeyCode::Down => {
                        *active_field = match active_field {
                            DialogField::Repo => DialogField::Branch,
                            DialogField::Branch => DialogField::Name,
                            DialogField::Name => DialogField::Name,
                        };
                    }
                    KeyCode::Up => {
                        *active_field = match active_field {
                            DialogField::Repo => DialogField::Repo,
                            DialogField::Branch => {
                                if !repo_names.is_empty() { DialogField::Repo } else { DialogField::Branch }
                            }
                            DialogField::Name => DialogField::Branch,
                        };
                    }
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
                            DialogField::Repo => {}
                            DialogField::Branch => input.push(c),
                            DialogField::Name => {
                                if short_name.len() < 20 { short_name.push(c); }
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        *selected_archived = None;
                        match active_field {
                            DialogField::Repo => {}
                            DialogField::Branch => { input.pop(); }
                            DialogField::Name => { short_name.pop(); }
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            Dialog::ArchiveConfirm(_idx, ref name, _) => {
                let name = name.clone();
                match key.code {
                    KeyCode::Char('y') => {
                        // Remove PTY session for this worktree (dropping it kills the child)
                        let wt = &self.sidebar_state.worktrees[self.sidebar_state.selected];
                        let key = wt.path.clone();
                        let repo_root = wt.repo_root.clone();
                        self.pty_sessions.remove(&key);
                        if self.active_worktree.as_ref() == Some(&key) {
                            self.active_worktree = None;
                        }

                        if let Some(mgr) = self.managers.get(&repo_root) {
                            mgr.delete(&name, false)?;
                        }
                        self.sidebar_state.worktrees = self.build_worktree_list()?;
                        if let Some(cache) = self.github_caches.get(&repo_root) {
                            cache.force_refresh(&repo_root);
                        }
                        self.apply_config();
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

    /// Handle paste events for active dialogs. Returns true if the dialog consumed the paste.
    pub fn handle_dialog_paste(&mut self, text: &str) -> bool {
        match &mut self.dialog {
            Dialog::CreateInput { ref mut input, ref mut short_name, ref active_field, ref mut selected_archived, .. } => {
                // Strip newlines — branch names can't contain them
                let clean: String = text.chars().filter(|c| *c != '\n' && *c != '\r').collect();
                *selected_archived = None;
                match active_field {
                    DialogField::Repo => {} // read-only
                    DialogField::Branch => input.push_str(&clean),
                    DialogField::Name => {
                        let remaining = 20usize.saturating_sub(short_name.len());
                        let truncated: String = clean.chars().take(remaining).collect();
                        short_name.push_str(&truncated);
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::Moved => {}
            MouseEventKind::Down(_) => {
                if mouse.column < self.sidebar_width {
                    self.focus = Focus::Sidebar;
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
                                    let repo_root = wt.repo_root.clone();
                                    let branch = wt.branch.clone();
                                    if let Some(config) = self.configs.get_mut(&repo_root) {
                                        let entry = config.worktrees.entry(branch).or_default();
                                        entry.status = new_status;
                                        let _ = config.save(&repo_root);
                                    }
                                }
                            }
                        }
                    }
                    // If not dragging, it was a click — already handled on Down
                }
            }
            MouseEventKind::ScrollUp => {
                if self.focus == Focus::Terminal {
                    self.scroll_offset = self.scroll_offset.saturating_add(3);
                }
            }
            MouseEventKind::ScrollDown => {
                if self.focus == Focus::Terminal {
                    self.scroll_offset = self.scroll_offset.saturating_sub(3);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn apply_config(&mut self) {
        for wt in &mut self.sidebar_state.worktrees {
            if wt.is_main {
                wt.workflow_status = WorkflowStatus::InProgress;
            } else if let Some(config) = self.configs.get(&wt.repo_root) {
                if let Some(wt_config) = config.worktrees.get(&wt.branch) {
                    wt.workflow_status = wt_config.status;
                    wt.short_name = wt_config.short_name.clone();
                }
            }
            Self::apply_pr_auto_status(&self.github_caches, wt);
        }
        self.sidebar_width = self.calculate_panel_width();
    }

    /// Override workflow status based on PR state (open -> InReview, merged -> Done).
    fn apply_pr_auto_status(
        github_caches: &HashMap<PathBuf, SharedGitHubCache>,
        wt: &mut WorktreeInfo,
    ) {
        if !wt.is_main {
            if let Some(cache) = github_caches.get(&wt.repo_root) {
                if let Some(pr) = cache.get(&wt.branch) {
                    match pr.state {
                        crate::github::PrState::Open => wt.workflow_status = WorkflowStatus::InReview,
                        crate::github::PrState::Merged => wt.workflow_status = WorkflowStatus::Done,
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn ensure_pty_for_selected(&mut self, rows: u16, cols: u16) -> Result<()> {
        if self.sidebar_state.worktrees.is_empty()
            || self.sidebar_state.selected >= self.sidebar_state.worktrees.len()
        {
            return Ok(());
        }

        // Lazily compute status and ahead/behind for the selected worktree
        {
            let wt = &mut self.sidebar_state.worktrees[self.sidebar_state.selected];
            if wt.status.is_none() {
                wt.status = crate::worktree::check_status(&wt.path).ok();
                let (ahead, behind) = crate::worktree::ahead_behind(&wt.path);
                wt.ahead = ahead;
                wt.behind = behind;
            }
        }

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
