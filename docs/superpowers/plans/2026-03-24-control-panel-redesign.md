# Control Panel Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the sidebar into a status-grouped control panel with drag-and-drop, activity indicators, auto-sizing, short names, and `.arbor.json` persistence.

**Architecture:** A new `persistence` module handles `.arbor.json` serialization. `WorktreeInfo` gains `workflow_status` and `short_name` fields populated from persisted state. `PtySession` exposes a `last_output` atomic timestamp for activity detection. The UI sidebar is renamed to `control_panel` and renders worktrees grouped by status with activity icons. The `App` struct replaces border-drag with worktree-drag and auto-sizing.

**Tech Stack:** Rust, ratatui, crossterm, serde_json (new dependency), git2, portable-pty, vt100-ctt

**Spec:** `docs/superpowers/specs/2026-03-24-control-panel-redesign-design.md`

---

### Task 1: Add serde_json dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add serde and serde_json to Cargo.toml**

Add to `[dependencies]`:
```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: OK, no errors

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add serde and serde_json dependencies"
```

---

### Task 2: Persistence module — ArborConfig load/save

**Files:**
- Create: `src/persistence.rs`
- Create: `tests/persistence.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing tests for persistence**

Create `tests/persistence.rs`:

```rust
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_load_missing_file_returns_defaults() {
    let dir = TempDir::new().unwrap();
    let config = arbor::persistence::ArborConfig::load(dir.path());
    assert!(config.worktrees.is_empty());
}

#[test]
fn test_load_malformed_json_returns_defaults() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".arbor.json");
    std::fs::write(&path, "not valid json {{{").unwrap();
    let config = arbor::persistence::ArborConfig::load(dir.path());
    assert!(config.worktrees.is_empty());
}

#[test]
fn test_save_and_load_roundtrip() {
    let dir = TempDir::new().unwrap();
    let mut config = arbor::persistence::ArborConfig::default();
    config.worktrees.insert(
        "feature-auth".to_string(),
        arbor::persistence::WorktreeConfig {
            status: arbor::persistence::WorkflowStatus::InProgress,
            short_name: Some("auth".to_string()),
        },
    );
    config.save(dir.path()).unwrap();

    let loaded = arbor::persistence::ArborConfig::load(dir.path());
    assert_eq!(loaded.worktrees.len(), 1);
    let wt = &loaded.worktrees["feature-auth"];
    assert_eq!(wt.status, arbor::persistence::WorkflowStatus::InProgress);
    assert_eq!(wt.short_name, Some("auth".to_string()));
}

#[test]
fn test_default_status_is_queued() {
    let config = arbor::persistence::WorktreeConfig::default();
    assert_eq!(config.status, arbor::persistence::WorkflowStatus::Queued);
}

#[test]
fn test_workflow_status_cycle() {
    use arbor::persistence::WorkflowStatus;
    assert_eq!(WorkflowStatus::Queued.next(), WorkflowStatus::InProgress);
    assert_eq!(WorkflowStatus::InProgress.next(), WorkflowStatus::Done);
    assert_eq!(WorkflowStatus::Done.next(), WorkflowStatus::Queued);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test persistence`
Expected: FAIL — `arbor::persistence` module doesn't exist

- [ ] **Step 3: Implement persistence module**

Create `src/persistence.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    #[default]
    Queued,
    InProgress,
    Done,
}

impl WorkflowStatus {
    pub fn next(self) -> Self {
        match self {
            Self::Queued => Self::InProgress,
            Self::InProgress => Self::Done,
            Self::Done => Self::Queued,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    pub status: WorkflowStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            status: WorkflowStatus::Queued,
            short_name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArborConfig {
    pub worktrees: HashMap<String, WorktreeConfig>,
}

impl ArborConfig {
    pub fn load(repo_root: &Path) -> Self {
        let path = repo_root.join(".arbor.json");
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("arbor: warning: malformed .arbor.json: {}", e);
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, repo_root: &Path) -> anyhow::Result<()> {
        let path = repo_root.join(".arbor.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }
}
```

Add to `src/lib.rs`:
```rust
pub mod persistence;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test persistence`
Expected: all 5 tests PASS

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: all existing + new tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/persistence.rs src/lib.rs tests/persistence.rs
git commit -m "feat: add persistence module for .arbor.json config"
```

---

### Task 3: Add workflow_status, short_name, and repo_root accessor to WorktreeInfo

**Files:**
- Modify: `src/worktree/manager.rs:7-13`
- Modify: `src/worktree/mod.rs`
- Modify: `tests/worktree_manager.rs`

> **Note:** This task also adds a `pub fn repo_root(&self) -> &Path` accessor to `WorktreeManager`, needed by Task 7 for persistence (the actual repo root may differ from the path passed to `App::new`).

- [ ] **Step 1: Write failing test**

Add to `tests/worktree_manager.rs`:

```rust
#[test]
fn test_worktree_info_has_workflow_status_and_short_name() {
    let dir = init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    // Main worktree defaults
    assert_eq!(worktrees[0].workflow_status, arbor::persistence::WorkflowStatus::InProgress);
    assert_eq!(worktrees[0].short_name, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_worktree_info_has_workflow_status_and_short_name`
Expected: FAIL — `workflow_status` field doesn't exist

- [ ] **Step 3: Add fields to WorktreeInfo**

In `src/worktree/manager.rs`, update `WorktreeInfo`:

```rust
use crate::persistence::WorkflowStatus;

pub struct WorktreeInfo {
    pub name: String,
    pub branch: String,
    pub path: PathBuf,
    pub is_main: bool,
    pub status: Option<WorktreeStatus>,
    pub workflow_status: WorkflowStatus,
    pub short_name: Option<String>,
}
```

Update both places in `list()` where `WorktreeInfo` is constructed. For the main worktree:
```rust
workflow_status: WorkflowStatus::InProgress,
short_name: None,
```

For additional worktrees:
```rust
workflow_status: WorkflowStatus::Queued,
short_name: None,
```

Add `pub use crate::persistence::WorkflowStatus;` to `src/worktree/mod.rs`.

Add a public accessor to `WorktreeManager`:

```rust
pub fn repo_root(&self) -> &Path {
    &self.repo_root
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/worktree/manager.rs src/worktree/mod.rs tests/worktree_manager.rs
git commit -m "feat: add workflow_status and short_name fields to WorktreeInfo"
```

---

### Task 4: Add last_output timestamp to PtySession

**Files:**
- Modify: `src/pty/session.rs`
- Create: `tests/pty_activity.rs`

- [ ] **Step 1: Write failing test**

Create `tests/pty_activity.rs`:

```rust
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_pty_last_output_millis_starts_at_zero() {
    // We can't easily spawn a PTY in tests without a terminal,
    // but we can test the atomic timestamp type directly
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    let ts = Arc::new(AtomicU64::new(0));
    assert_eq!(ts.load(Ordering::Relaxed), 0);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    ts.store(now, Ordering::Relaxed);
    assert!(ts.load(Ordering::Relaxed) > 0);
}
```

- [ ] **Step 2: Run test to verify it passes** (this is a smoke test for the approach)

Run: `cargo test --test pty_activity`
Expected: PASS

- [ ] **Step 3: Add last_output to PtySession**

In `src/pty/session.rs`, add import at top:

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
```

Add field to `PtySession` struct:

```rust
pub struct PtySession {
    writer: Box<dyn Write + Send>,
    parser: Arc<Mutex<vt100_ctt::Parser>>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    last_output: Arc<AtomicU64>,
}
```

In `spawn()`, create and share the atomic:

```rust
let last_output = Arc::new(AtomicU64::new(0));

// In the reader thread closure, capture last_output_clone:
let last_output_clone = Arc::clone(&last_output);
let parser_clone = Arc::clone(&parser);
std::thread::spawn(move || {
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                last_output_clone.store(now, Ordering::Relaxed);
                let mut p = parser_clone.lock().unwrap();
                p.process(&buf[..n]);
            }
            Err(_) => break,
        }
    }
});
```

Add to the `Ok(Self { ... })` return: `last_output,`

Add a public method:

```rust
pub fn last_output_millis(&self) -> u64 {
    self.last_output.load(Ordering::Relaxed)
}
```

- [ ] **Step 4: Verify it compiles and all tests pass**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/pty/session.rs tests/pty_activity.rs
git commit -m "feat: add last_output atomic timestamp to PtySession"
```

---

### Task 5: Update keys — remove resize, add StatusCycle

**Files:**
- Modify: `src/keys.rs`
- Create: `tests/keys.rs`

- [ ] **Step 1: Write failing test**

Create `tests/keys.rs`:

```rust
use arbor::keys::{handle_key, Action, Focus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn test_s_key_triggers_status_cycle() {
    let action = handle_key(make_key(KeyCode::Char('s')), &Focus::Sidebar);
    assert!(matches!(action, Action::StatusCycle));
}

#[test]
fn test_less_than_no_longer_resizes() {
    let action = handle_key(make_key(KeyCode::Char('<')), &Focus::Sidebar);
    assert!(matches!(action, Action::None));
}

#[test]
fn test_greater_than_no_longer_resizes() {
    let action = handle_key(make_key(KeyCode::Char('>')), &Focus::Sidebar);
    assert!(matches!(action, Action::None));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test keys`
Expected: FAIL — `Action::StatusCycle` doesn't exist

- [ ] **Step 3: Update keys.rs**

In `src/keys.rs`, update the `Action` enum — remove `SidebarResizeLeft` and `SidebarResizeRight`, add `StatusCycle`:

```rust
#[derive(Debug)]
pub enum Action {
    ToggleFocus,
    FocusSidebar,
    FocusTerminal,
    SidebarUp,
    SidebarDown,
    SidebarSelect,
    SidebarCreate,
    SidebarArchive,
    SidebarHelp,
    StatusCycle,
    TerminalInput(KeyEvent),
    Quit,
    None,
}
```

In `handle_key`, update the Sidebar match arm — remove `<`/`>` lines, add `s`:

```rust
Focus::Sidebar => match key.code {
    KeyCode::Up | KeyCode::Char('k') => Action::SidebarUp,
    KeyCode::Down | KeyCode::Char('j') => Action::SidebarDown,
    KeyCode::Enter => Action::SidebarSelect,
    KeyCode::Char('n') => Action::SidebarCreate,
    KeyCode::Char('a') => Action::SidebarArchive,
    KeyCode::Char('s') => Action::StatusCycle,
    KeyCode::Char('?') => Action::SidebarHelp,
    KeyCode::Esc => Action::FocusTerminal,
    KeyCode::Char('q') => Action::Quit,
    _ => Action::None,
},
```

- [ ] **Step 4: Update app.rs to remove resize action handlers**

In `src/app.rs`, remove the `Action::SidebarResizeLeft` and `Action::SidebarResizeRight` match arms from `handle_action`. Add a placeholder for `StatusCycle`:

```rust
Action::StatusCycle => {
    // Will be implemented with persistence wiring in Task 7
}
```

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/keys.rs src/app.rs tests/keys.rs
git commit -m "feat: replace resize keys with status cycle action"
```

---

### Task 6: Rename sidebar to control_panel, add grouped rendering

**Files:**
- Rename: `src/ui/sidebar.rs` → `src/ui/control_panel.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/app.rs` (update imports)

- [ ] **Step 1: Rename the file**

```bash
cd /Users/richardhope/Repositories/Enablis/arbor
git mv src/ui/sidebar.rs src/ui/control_panel.rs
```

- [ ] **Step 2: Update src/ui/mod.rs**

```rust
mod control_panel;
mod terminal;

pub use control_panel::render_control_panel;
pub use control_panel::ControlPanelState;
pub use terminal::TerminalWidget;
```

- [ ] **Step 3: Rename SidebarState to ControlPanelState in control_panel.rs**

In `src/ui/control_panel.rs`, rename `SidebarState` to `ControlPanelState` and `render_sidebar` to `render_control_panel`. Update the `drag_handle_active` parameter name — remove it since border drag is gone. Add `spinner_frame: u8` and `pty_last_outputs: &HashMap<PathBuf, u64>` params.

Replace the entire `render_sidebar` signature with:

```rust
pub fn render_control_panel(
    state: &ControlPanelState,
    dialog: &Dialog,
    area: Rect,
    buf: &mut Buffer,
    focused: bool,
    spinner_frame: u8,
    pty_last_outputs: &std::collections::HashMap<std::path::PathBuf, u64>,
)
```

Rename `SidebarState` to `ControlPanelState`:

```rust
pub struct ControlPanelState {
    pub selected: usize,
    pub worktrees: Vec<WorktreeInfo>,
    pub show_plus: bool,
}
```

Remove the entire `drag_handle_active` border highlighting block (lines 39-54 of the current sidebar.rs).

Replace the worktree rendering loop with grouped rendering. Import `WorkflowStatus`:

```rust
use crate::persistence::WorkflowStatus;
```

Replace the items loop (lines 56-102 of sidebar.rs) with:

```rust
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
let mut flat_to_visual: Vec<usize> = Vec::new(); // maps flat index → visual row in items

for (status, label) in groups {
    let group_wts: Vec<(usize, &WorktreeInfo)> = state.worktrees.iter()
        .enumerate()
        .filter(|(_, wt)| wt.workflow_status == *status)
        .collect();

    if group_wts.is_empty() {
        continue;
    }

    // Group header (not selectable)
    items.push(ListItem::new(Line::from(Span::styled(
        format!(" {}", label),
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
    ))));

    for (flat_idx, wt) in &group_wts {
        let is_selected = *flat_idx == state.selected;

        // Activity icon
        let icon = if let Some(&last_output) = pty_last_outputs.get(&wt.path) {
            if last_output > 0 && now_millis.saturating_sub(last_output) < 500 {
                // Busy — spinner
                let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let frame_char = frames[(spinner_frame % 10) as usize];
                Span::styled(format!("{} ", frame_char), Style::default().fg(Color::Cyan))
            } else {
                // Idle — waiting
                Span::styled("! ", Style::default().fg(Color::Yellow))
            }
        } else {
            // No PTY session
            match wt.workflow_status {
                WorkflowStatus::Queued => Span::styled("▶ ", Style::default().fg(Color::DarkGray)),
                WorkflowStatus::Done => Span::styled("✓ ", Style::default().fg(Color::Green)),
                WorkflowStatus::InProgress => Span::styled("· ", Style::default().fg(Color::DarkGray)),
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

    // Empty line between groups
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
```

Update the `ListState` selection to use the visual row mapping:

```rust
let mut list_state = ListState::default();
if state.selected < state.worktrees.len() {
    if let Some(&visual_idx) = flat_to_visual.get(state.selected) {
        list_state.select(Some(visual_idx));
    }
} else {
    list_state.select(Some(plus_visual_idx));
}
```

- [ ] **Step 4: Update app.rs imports and references**

Replace all occurrences:
- `ui::render_sidebar` → `ui::render_control_panel`
- `ui::SidebarState` → `ui::ControlPanelState`
- `sidebar_state` field type `SidebarState` → `ControlPanelState`

In `App` struct, remove `hover_border` field. In the render call, update to pass new parameters:

```rust
// Build pty_last_outputs map
let pty_last_outputs: std::collections::HashMap<PathBuf, u64> = self.pty_sessions.iter()
    .map(|(k, v)| (k.clone(), v.last_output_millis()))
    .collect();

ui::render_control_panel(
    &self.sidebar_state,
    &self.dialog,
    chunks[0],
    frame.buffer_mut(),
    self.focus == Focus::Sidebar,
    self.spinner_frame,
    &pty_last_outputs,
);
```

Add `spinner_frame: u8` to `App` struct, initialize to `0` in `new()`. Increment after each draw: `self.spinner_frame = self.spinner_frame.wrapping_add(1);` (add this line after the `terminal.draw(...)` call).

- [ ] **Step 5: Update existing app_mouse tests**

The `app_mouse.rs` tests reference `app.focus` which should still work. Verify the `SidebarState` → `ControlPanelState` rename doesn't break them — the field is `sidebar_state` on `App`, which we keep as-is for now (renaming the field is cosmetic and can be done later).

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: rename sidebar to control_panel, add grouped rendering with activity icons"
```

---

### Task 7: Wire up persistence in App — load, save, StatusCycle

**Files:**
- Modify: `src/app.rs`
- Create: `tests/app_status.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/app_status.rs`:

```rust
use std::process::Command;
use tempfile::TempDir;
use arbor::keys::Focus;
use arbor::persistence::WorkflowStatus;

fn init_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "commit", "--allow-empty", "-m", "init"])
        .output()
        .unwrap();
    dir
}

#[test]
fn test_main_worktree_defaults_to_in_progress() {
    let dir = init_test_repo();
    let app = arbor::app::App::new(dir.path()).unwrap();
    let wt = &app.sidebar_state.worktrees[0];
    assert!(wt.is_main);
    assert_eq!(wt.workflow_status, WorkflowStatus::InProgress);
}

#[test]
fn test_status_cycle_changes_status() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    // Create a non-main worktree
    app.sidebar_state.worktrees[0].is_main; // main is at 0

    // We need a second worktree to test cycling
    // For now, manually add workflow_status test via handle_action
    // The worktree_mgr.create will add one
    // But we can test the action directly by setting up state
    app.focus = Focus::Sidebar;
    // Status cycle on main should be no-op
    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();
    assert_eq!(app.sidebar_state.worktrees[0].workflow_status, WorkflowStatus::InProgress);
}

#[test]
fn test_status_cycle_on_non_main() {
    let dir = init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("feature-a").unwrap();

    let mut app = arbor::app::App::new(dir.path()).unwrap();
    // Find the non-main worktree
    let non_main_idx = app.sidebar_state.worktrees.iter()
        .position(|w| !w.is_main)
        .unwrap();
    app.sidebar_state.selected = non_main_idx;
    app.focus = Focus::Sidebar;

    assert_eq!(app.sidebar_state.worktrees[non_main_idx].workflow_status, WorkflowStatus::Queued);

    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();
    assert_eq!(app.sidebar_state.worktrees[non_main_idx].workflow_status, WorkflowStatus::InProgress);

    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();
    assert_eq!(app.sidebar_state.worktrees[non_main_idx].workflow_status, WorkflowStatus::Done);

    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();
    assert_eq!(app.sidebar_state.worktrees[non_main_idx].workflow_status, WorkflowStatus::Queued);
}

#[test]
fn test_status_cycle_persists_to_file() {
    let dir = init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("feature-b").unwrap();

    let mut app = arbor::app::App::new(dir.path()).unwrap();
    let non_main_idx = app.sidebar_state.worktrees.iter()
        .position(|w| !w.is_main)
        .unwrap();
    app.sidebar_state.selected = non_main_idx;
    app.focus = Focus::Sidebar;

    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();

    // Check .arbor.json was written
    let config = arbor::persistence::ArborConfig::load(dir.path());
    assert_eq!(config.worktrees["feature-b"].status, WorkflowStatus::InProgress);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test app_status`
Expected: FAIL

- [ ] **Step 3: Wire persistence into App**

In `src/app.rs`, add imports:

```rust
use crate::persistence::{ArborConfig, WorkflowStatus};
```

Add `config: ArborConfig` and `repo_root: PathBuf` fields to `App` struct.

In `App::new()`, after creating `worktree_mgr` and before creating `sidebar_state`, use the manager's resolved root (not the input path, which may be a subdirectory):

```rust
let repo_root = worktree_mgr.repo_root().to_path_buf();
let config = ArborConfig::load(&repo_root);
```

After getting `worktrees` from `list()`, apply config to each worktree:

```rust
let mut worktrees = worktree_mgr.list()?;
let config = ArborConfig::load(repo_path);
for wt in &mut worktrees {
    if wt.is_main {
        wt.workflow_status = WorkflowStatus::InProgress;
    } else if let Some(wt_config) = config.worktrees.get(&wt.name) {
        wt.workflow_status = wt_config.status;
        wt.short_name = wt_config.short_name.clone();
    }
}
```

Initialize `repo_root` and `config` in the `Self { ... }` block.

Implement `StatusCycle` in `handle_action`:

```rust
Action::StatusCycle => {
    let idx = self.sidebar_state.selected;
    if idx < self.sidebar_state.worktrees.len() {
        let wt = &mut self.sidebar_state.worktrees[idx];
        if !wt.is_main {
            wt.workflow_status = wt.workflow_status.next();
            // Persist
            let entry = self.config.worktrees
                .entry(wt.name.clone())
                .or_default();
            entry.status = wt.workflow_status;
            let _ = self.config.save(&self.repo_root);
        }
    }
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/app.rs tests/app_status.rs
git commit -m "feat: wire persistence into App, implement StatusCycle action"
```

---

### Task 8: Auto-sizing control panel

**Files:**
- Modify: `src/app.rs`
- Create: `tests/app_autosize.rs`

- [ ] **Step 1: Write failing test**

Create `tests/app_autosize.rs`:

```rust
use std::process::Command;
use tempfile::TempDir;

fn init_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "commit", "--allow-empty", "-m", "init"])
        .output()
        .unwrap();
    dir
}

#[test]
fn test_autosize_minimum_width() {
    let dir = init_test_repo();
    let app = arbor::app::App::new(dir.path()).unwrap();
    // With just "main" (4 chars), width should be at minimum 20
    assert!(app.panel_width() >= 20);
}

#[test]
fn test_autosize_grows_with_long_names() {
    let dir = init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("very-long-feature-branch-name-here").unwrap();

    let app = arbor::app::App::new(dir.path()).unwrap();
    // "very-long-feature-branch-name-here" = 34 chars + padding > 20
    assert!(app.panel_width() > 20);
}

#[test]
fn test_autosize_respects_maximum() {
    let dir = init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("a-really-extremely-long-branch-name-that-goes-way-beyond-sixty-characters-total").unwrap();

    let app = arbor::app::App::new(dir.path()).unwrap();
    assert!(app.panel_width() <= 60);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test app_autosize`
Expected: FAIL — `panel_width()` method doesn't exist

- [ ] **Step 3: Implement auto-sizing**

In `src/app.rs`, add a method and make `sidebar_width` computed:

```rust
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
```

Call `self.sidebar_width = self.calculate_panel_width();` at the end of `App::new()` and after any operation that changes the worktree list (create, delete, status cycle, focus switch that refreshes list).

Remove the old hardcoded `sidebar_width: 30` in `new()`, replace with `sidebar_width: 20` (will be overwritten immediately by `calculate_panel_width()`).

- [ ] **Step 4: Remove border drag logic from handle_mouse**

In `handle_mouse`, remove:
- The `border_col` and `near_border` calculations
- The `MouseEventKind::Drag` arm for `self.dragging_sidebar`
- The `hover_border` updates
- Remove `dragging_sidebar` and `hover_border` fields from `App` struct

Keep the click-to-focus logic (sidebar click vs terminal click).

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: all tests PASS (some `app_mouse` tests about border may need updating since border drag is removed)

- [ ] **Step 6: Update app_mouse tests**

Remove `test_click_near_border_does_not_change_focus` test since border drag is gone. The border area now behaves like sidebar area for click-to-focus.

- [ ] **Step 7: Commit**

```bash
git add src/app.rs tests/app_autosize.rs tests/app_mouse.rs
git commit -m "feat: auto-size control panel width, remove border drag"
```

---

### Task 9: Update create dialog — add Name field

**Files:**
- Modify: `src/app.rs` (Dialog enum, handle_dialog_key)
- Modify: `src/ui/control_panel.rs` (dialog rendering)
- Create: `tests/app_create_dialog.rs`

- [ ] **Step 1: Write failing test**

Create `tests/app_create_dialog.rs`:

```rust
use std::process::Command;
use tempfile::TempDir;
use arbor::app::Dialog;

fn init_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "commit", "--allow-empty", "-m", "init"])
        .output()
        .unwrap();
    dir
}

#[test]
fn test_create_dialog_has_short_name_field() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.handle_action(arbor::keys::Action::SidebarCreate).unwrap();

    match &app.dialog {
        Dialog::CreateInput { short_name, .. } => {
            assert_eq!(*short_name, String::new());
        }
        _ => panic!("Expected CreateInput dialog"),
    }
}

#[test]
fn test_create_with_short_name_persists() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();

    // Simulate opening dialog and setting values
    app.dialog = Dialog::CreateInput {
        input: "feature-x".to_string(),
        short_name: "fx".to_string(),
        active_field: arbor::app::DialogField::Branch,
        archived: vec![],
        selected_archived: None,
    };

    // Simulate Enter key
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    app.handle_dialog_key(enter).unwrap();

    // Check persistence
    let config = arbor::persistence::ArborConfig::load(dir.path());
    assert_eq!(config.worktrees["feature-x"].short_name, Some("fx".to_string()));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test app_create_dialog`
Expected: FAIL — `short_name` field doesn't exist on `CreateInput`

- [ ] **Step 3: Update Dialog enum**

In `src/app.rs`, add a `DialogField` enum and update `CreateInput`:

```rust
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
        archived: Vec<String>,
        selected_archived: Option<usize>,
    },
    ArchiveConfirm(usize, String),
}
```

- [ ] **Step 4: Update SidebarCreate action to initialize new fields**

```rust
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
```

- [ ] **Step 5: Make handle_dialog_key public and update for two fields**

Change `fn handle_dialog_key` to `pub fn handle_dialog_key` in `src/app.rs` (needed for tests). Update the `CreateInput` match arm:

- Add `ref mut short_name` and `ref mut active_field` destructuring
- Up/Down arrows switch `active_field` between `Branch` and `Name`
- Tab only cycles archived branches when `active_field == Branch`
- Char/Backspace input goes to whichever field is active
- On Enter, after `create()`, persist short_name if non-empty:

```rust
KeyCode::Enter => {
    let branch = if let Some(idx) = selected_archived {
        archived[*idx].clone()
    } else if !input.is_empty() {
        input.clone()
    } else {
        return Ok(true);
    };
    let sn = if short_name.is_empty() { None } else { Some(short_name.clone()) };
    self.worktree_mgr.create(&branch)?;
    // Persist short name
    let entry = self.config.worktrees.entry(branch.clone()).or_default();
    if let Some(ref name) = sn {
        entry.short_name = Some(name.clone());
    }
    let _ = self.config.save(&self.repo_root);
    // Refresh list and apply config
    self.sidebar_state.worktrees = self.worktree_mgr.list()?;
    self.apply_config();
    // ... rest of selection logic
}
```

Add `Down`/`Up` arrow handling:
```rust
KeyCode::Down => {
    *active_field = DialogField::Name;
}
KeyCode::Up => {
    *active_field = DialogField::Branch;
}
```

Route `Char`/`Backspace` based on `active_field`:
```rust
KeyCode::Char(c) => {
    *selected_archived = None;
    match active_field {
        DialogField::Branch => input.push(c),
        DialogField::Name => {
            if short_name.len() < 20 {
                short_name.push(c);
            }
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
```

- [ ] **Step 6: Add apply_config helper to App**

```rust
fn apply_config(&mut self) {
    for wt in &mut self.sidebar_state.worktrees {
        if wt.is_main {
            wt.workflow_status = WorkflowStatus::InProgress;
        } else if let Some(wt_config) = self.config.worktrees.get(&wt.name) {
            wt.workflow_status = wt_config.status;
            wt.short_name = wt_config.short_name.clone();
        }
    }
    self.sidebar_width = self.calculate_panel_width();
}
```

- [ ] **Step 7: Update dialog rendering in control_panel.rs**

Add the Name field row in the `CreateInput` dialog rendering, between Branch and the archived hint. Highlight the active field.

- [ ] **Step 8: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 9: Commit**

```bash
git add src/app.rs src/ui/control_panel.rs tests/app_create_dialog.rs
git commit -m "feat: add short name field to create dialog with persistence"
```

---

### Task 10: Drag and drop between status groups

**Files:**
- Modify: `src/app.rs` (handle_mouse)
- Create: `tests/app_drag.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/app_drag.rs`:

```rust
use std::process::Command;
use tempfile::TempDir;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind, KeyModifiers};
use arbor::keys::Focus;
use arbor::persistence::WorkflowStatus;

fn init_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "commit", "--allow-empty", "-m", "init"])
        .output()
        .unwrap();
    dir
}

fn mouse_event(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::empty(),
    }
}

#[test]
fn test_click_without_drag_selects() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.focus = Focus::Terminal;

    // Mouse down on sidebar area
    app.handle_mouse(mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 3)).unwrap();
    // Mouse up immediately (no drag)
    app.handle_mouse(mouse_event(MouseEventKind::Up(MouseButton::Left), 5, 3)).unwrap();

    assert_eq!(app.focus, Focus::Sidebar);
}

#[test]
fn test_main_worktree_cannot_be_dragged() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.focus = Focus::Sidebar;
    app.sidebar_state.selected = 0; // main

    // Attempt drag on main
    app.handle_mouse(mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 2)).unwrap();
    app.handle_mouse(mouse_event(MouseEventKind::Drag(MouseButton::Left), 5, 8)).unwrap();
    app.handle_mouse(mouse_event(MouseEventKind::Up(MouseButton::Left), 5, 8)).unwrap();

    // Main should still be InProgress
    assert_eq!(app.sidebar_state.worktrees[0].workflow_status, WorkflowStatus::InProgress);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test app_drag`
Expected: FAIL or unexpected behavior

- [ ] **Step 3: Add layout tracking to ControlPanelState**

Add to `ControlPanelState` in `src/ui/control_panel.rs`:

```rust
pub struct ControlPanelState {
    pub selected: usize,
    pub worktrees: Vec<WorktreeInfo>,
    pub show_plus: bool,
    /// Maps visual row (relative to panel inner area) → flat worktree index. Populated during render.
    pub row_to_flat_idx: Vec<Option<usize>>,
    /// Maps status group → (start_row, end_row) in panel inner area. Populated during render.
    pub group_regions: Vec<(WorkflowStatus, u16, u16)>,
}
```

During `render_control_panel`, populate `row_to_flat_idx` and `group_regions` as items are laid out. Clear them at the start of each render, then:
- When rendering a group header, record the start row for that group
- When rendering a worktree item, record `row_to_flat_idx[visual_row] = Some(flat_idx)`
- After all items in a group, record the end row

- [ ] **Step 4: Implement drag state and logic**

Add to `App` struct:

```rust
drag_state: Option<DragState>,
```

Add `DragState` struct:

```rust
struct DragState {
    worktree_idx: usize,
    start_row: u16,
    dragging: bool, // becomes true on first Drag event
}
```

Update `handle_mouse`. On `Down`, use `row_to_flat_idx` to determine which worktree was clicked:

```rust
pub fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) -> Result<()> {
    match mouse.kind {
        MouseEventKind::Down(_) => {
            if mouse.column < self.sidebar_width {
                // Determine which worktree was clicked using row mapping
                let row = mouse.row as usize;
                let clicked_idx = self.sidebar_state.row_to_flat_idx
                    .get(row)
                    .copied()
                    .flatten();

                if let Some(idx) = clicked_idx {
                    // Select the clicked item immediately
                    self.sidebar_state.selected = idx;
                    self.focus = Focus::Sidebar;

                    // Start potential drag (only for non-main)
                    let wt = &self.sidebar_state.worktrees[idx];
                    if !wt.is_main {
                        self.drag_state = Some(DragState {
                            worktree_idx: idx,
                            start_row: mouse.row,
                            dragging: false,
                        });
                    }
                } else {
                    // Clicked on header/empty area — just focus sidebar
                    self.focus = Focus::Sidebar;
                }
            } else {
                self.focus = Focus::Terminal;
            }
        }
        MouseEventKind::Drag(_) => {
            if let Some(ref mut drag) = self.drag_state {
                drag.dragging = true;
                // Visual feedback handled by render
            }
        }
        MouseEventKind::Up(_) => {
            if let Some(drag) = self.drag_state.take() {
                if drag.dragging {
                    // Find which status group the cursor is over using group_regions
                    let target_status = self.sidebar_state.group_regions.iter()
                        .find(|(_, start, end)| mouse.row >= *start && mouse.row < *end)
                        .map(|(status, _, _)| *status);

                    if let Some(new_status) = target_status {
                        let wt = &mut self.sidebar_state.worktrees[drag.worktree_idx];
                        if wt.workflow_status != new_status {
                            wt.workflow_status = new_status;
                            let entry = self.config.worktrees
                                .entry(wt.name.clone())
                                .or_default();
                            entry.status = new_status;
                            let _ = self.config.save(&self.repo_root);
                        }
                    }
                }
                // If !drag.dragging, the click already selected on Down
            }
        }
        MouseEventKind::Moved => {}
        _ => {}
    }
    Ok(())
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/app.rs tests/app_drag.rs
git commit -m "feat: implement drag-and-drop between status groups"
```

---

### Task 11: Update status bar with new hints

**Files:**
- Modify: `src/app.rs` (build_status_line)

- [ ] **Step 1: Update build_status_line**

Replace the `Focus::Sidebar` hints to include `s status` and remove resize hints:

```rust
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
```

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 3: Manual smoke test**

Run: `cargo run --manifest-path /Users/richardhope/Repositories/Enablis/arbor/Cargo.toml -- --repo /Users/richardhope/Repositories/Enablis/arbor`

Verify:
- Control panel shows grouped worktrees
- `s` key cycles status
- Status bar shows new hints
- Activity indicators appear
- Auto-sizing works

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "feat: update status bar hints for control panel"
```

---

### Task 12: Final cleanup and all-tests pass

**Files:**
- All modified files

- [ ] **Step 1: Run clippy**

Run: `cargo clippy`
Expected: no warnings (fix any that appear)

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 3: Commit any fixes**

```bash
git add -A
git commit -m "chore: clippy fixes and cleanup"
```
