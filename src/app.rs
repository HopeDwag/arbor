use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::DefaultTerminal;
use std::path::PathBuf;
use std::time::Duration;

use crate::keys::{self, Action, Focus};
use crate::pty::PtySession;
use crate::ui;
use crate::ui::SidebarState;
use crate::ui::TerminalWidget;
use crate::worktree::WorktreeManager;
use crate::zellij::ZellijManager;

#[derive(Debug)]
pub enum Dialog {
    None,
    CreateInput(String),           // branch name being typed
    DeleteConfirm(usize, String),  // index, worktree name
}

pub struct App {
    worktree_mgr: WorktreeManager,
    zellij_mgr: ZellijManager,
    pty_session: Option<PtySession>,
    pub focus: Focus,
    pub sidebar_state: SidebarState,
    pub dialog: Dialog,
    sidebar_width: u16,
    should_quit: bool,
}

impl App {
    pub fn new(repo_path: &PathBuf) -> Result<Self> {
        let worktree_mgr = WorktreeManager::open(repo_path)?;
        let zellij_mgr = ZellijManager::new()?;
        let worktrees = worktree_mgr.list()?;

        let sidebar_state = SidebarState {
            selected: 0,
            worktrees,
            show_plus: true,
        };

        Ok(Self {
            worktree_mgr,
            zellij_mgr,
            pty_session: None,
            focus: Focus::Terminal,
            sidebar_state,
            dialog: Dialog::None,
            sidebar_width: 30,
            should_quit: false,
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        // Launch zellij for the first (main) worktree
        let size = terminal.size()?;
        self.launch_zellij_for_selected(size.height, size.width)?;

        while !self.should_quit {
            terminal.draw(|frame| {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(self.sidebar_width),
                        Constraint::Min(1),
                    ])
                    .split(frame.area());

                ui::render_sidebar(
                    &self.sidebar_state,
                    &self.dialog,
                    chunks[0],
                    frame.buffer_mut(),
                    self.focus == Focus::Sidebar,
                );

                if let Some(ref pty) = self.pty_session {
                    let term_widget = TerminalWidget::new(pty.screen());
                    frame.render_widget(term_widget, chunks[1]);
                }
            })?;

            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    // Dialogs consume raw key events first
                    if self.handle_dialog_key(key)? {
                        continue;
                    }
                    let action = keys::handle_key(key, &self.focus);
                    self.handle_action(action)?;
                }
            }
        }

        Ok(())
    }

    fn handle_action(&mut self, action: Action) -> Result<()> {
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
                    self.launch_zellij_for_selected(size.1, size.0)?;
                    self.focus = Focus::Terminal;
                } else {
                    self.handle_action(Action::SidebarCreate)?;
                }
            }
            Action::SidebarCreate => {
                self.dialog = Dialog::CreateInput(String::new());
            }
            Action::SidebarDelete => {
                let idx = self.sidebar_state.selected;
                if idx < self.sidebar_state.worktrees.len() {
                    let wt = &self.sidebar_state.worktrees[idx];
                    if !wt.is_main {
                        let name = wt.name.clone();
                        self.dialog = Dialog::DeleteConfirm(idx, name);
                    }
                }
            }
            Action::SidebarResizeLeft => {
                if self.sidebar_width > 20 {
                    self.sidebar_width -= 2;
                }
            }
            Action::SidebarResizeRight => {
                if self.sidebar_width < 60 {
                    self.sidebar_width += 2;
                }
            }
            Action::TerminalInput(key) => {
                if let Some(ref mut pty) = self.pty_session {
                    let bytes = key_to_bytes(key);
                    if !bytes.is_empty() {
                        pty.write(&bytes)?;
                    }
                }
            }
            Action::Quit => self.should_quit = true,
            _ => {}
        }
        Ok(())
    }

    /// Handle raw key events for active dialogs. Returns true if the dialog consumed the event.
    fn handle_dialog_key(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        use crossterm::event::KeyCode;

        match &mut self.dialog {
            Dialog::CreateInput(ref mut input) => {
                match key.code {
                    KeyCode::Enter => {
                        if !input.is_empty() {
                            let branch = input.clone();
                            self.worktree_mgr.create(&branch)?;
                            self.sidebar_state.worktrees = self.worktree_mgr.list()?;
                            self.sidebar_state.selected = self.sidebar_state.worktrees.len() - 1;
                            self.dialog = Dialog::None;
                            let size = crossterm::terminal::size()?;
                            self.launch_zellij_for_selected(size.1, size.0)?;
                            self.focus = Focus::Terminal;
                        }
                    }
                    KeyCode::Esc => self.dialog = Dialog::None,
                    KeyCode::Char(c) => input.push(c),
                    KeyCode::Backspace => { input.pop(); }
                    _ => {}
                }
                Ok(true)
            }
            Dialog::DeleteConfirm(_idx, ref name) => {
                let name = name.clone();
                match key.code {
                    KeyCode::Char('y') => {
                        let session_name = crate::zellij::sanitize_session_name(&name);
                        let _ = self.zellij_mgr.kill_session(&session_name);
                        self.worktree_mgr.delete(&name, false)?;
                        self.sidebar_state.worktrees = self.worktree_mgr.list()?;
                        self.sidebar_state.selected = 0;
                        self.dialog = Dialog::None;
                        let size = crossterm::terminal::size()?;
                        self.launch_zellij_for_selected(size.1, size.0)?;
                    }
                    KeyCode::Char('n') | KeyCode::Esc => self.dialog = Dialog::None,
                    _ => {}
                }
                Ok(true)
            }
            Dialog::None => Ok(false),
        }
    }

    fn launch_zellij_for_selected(&mut self, rows: u16, cols: u16) -> Result<()> {
        let wt = &self.sidebar_state.worktrees[self.sidebar_state.selected];
        let session_name = self.zellij_mgr.create_session(&wt.branch, &wt.path)?;

        let args = if self.zellij_mgr.session_exists(&session_name) {
            self.zellij_mgr.zellij_attach_args(&session_name)
        } else {
            self.zellij_mgr.zellij_launch_args(&session_name)
        };

        let terminal_cols = cols.saturating_sub(self.sidebar_width);
        self.pty_session = Some(PtySession::spawn("zellij", &args, rows, terminal_cols)?);
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
