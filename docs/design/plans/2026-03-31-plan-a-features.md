# Plan A: Feature Enrichment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enrich the Arbor sidebar with two-row worktree cards, dirty/PR badges, a fixed footer bar, consolidated detail bar, filter functionality, and open-PR-in-browser action.

**Architecture:** Expand `WorktreeInfo` with `commit_message`, `is_dirty`, and `pr` fields. Refactor `control_panel.rs` to render two-line cards with right-aligned tags. Replace the inline `[+]` button with a fixed footer. Merge the header + info bar into a single two-row detail bar. Add filter and open-PR actions via new `Action` variants in `keys.rs`.

**Tech Stack:** Rust, ratatui 0.29, git2 0.19, crossterm 0.28

---

### Task 1: Add `commit_message` and `is_dirty` fields to WorktreeInfo

**Files:**
- Modify: `src/worktree/manager.rs:9-24` (WorktreeInfo struct)
- Modify: `src/worktree/manager.rs:76-89` (main worktree construction)
- Modify: `src/worktree/manager.rs:105-118` (additional worktree construction)
- Test: `tests/worktree_manager.rs`

- [ ] **Step 1: Write the failing tests**

Add to `tests/worktree_manager.rs`:

```rust
#[test]
fn test_worktree_info_has_commit_message() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    // init_test_repo creates a commit with message "init"
    assert_eq!(worktrees[0].commit_message.as_deref(), Some("init"));
}

#[test]
fn test_worktree_info_has_is_dirty() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    assert!(!worktrees[0].is_dirty);
}

#[test]
fn test_worktree_dirty_when_file_added() {
    let dir = common::init_test_repo();
    std::fs::write(dir.path().join("dirty.txt"), "hello").unwrap();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    assert!(worktrees[0].is_dirty);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_worktree_info_has_commit_message test_worktree_info_has_is_dirty test_worktree_dirty_when_file_added -- --nocapture`

Expected: FAIL — `no field named commit_message` / `no field named is_dirty`

- [ ] **Step 3: Add fields to WorktreeInfo and populate them**

In `src/worktree/manager.rs`, add fields to the struct (after line 23):

```rust
pub struct WorktreeInfo {
    pub name: String,
    pub branch: String,
    pub path: PathBuf,
    pub is_main: bool,
    pub status: Option<WorktreeStatus>,
    pub workflow_status: WorkflowStatus,
    pub short_name: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub repo_name: Option<String>,
    pub repo_root: PathBuf,
    pub last_commit_age_secs: u64,
    pub commit_message: Option<String>,
    pub is_dirty: bool,
}
```

Add a helper function after `commit_age_secs()`:

```rust
/// Read the HEAD commit summary message.
fn commit_summary(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    commit.summary().map(String::from)
}

/// Check if the working tree has uncommitted changes.
fn is_repo_dirty(repo: &Repository) -> bool {
    repo.statuses(None)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}
```

Update the main worktree push (around line 76):

```rust
result.push(WorktreeInfo {
    name: main_name,
    branch: main_branch,
    path: self.repo_root.clone(),
    is_main: true,
    status: None,
    workflow_status: WorkflowStatus::InProgress,
    short_name: None,
    ahead: 0,
    behind: 0,
    repo_name: None,
    repo_root: self.repo_root.clone(),
    last_commit_age_secs: commit_age_secs(&self.repo),
    commit_message: commit_summary(&self.repo),
    is_dirty: is_repo_dirty(&self.repo),
});
```

Update the additional worktree push (around line 105):

```rust
let age = commit_age_secs(&wt_repo);
result.push(WorktreeInfo {
    name: name.to_string(),
    branch,
    path: wt_path,
    is_main: false,
    status: None,
    workflow_status: WorkflowStatus::Queued,
    short_name: None,
    ahead: 0,
    behind: 0,
    repo_name: None,
    repo_root: self.repo_root.clone(),
    last_commit_age_secs: age,
    commit_message: commit_summary(&wt_repo),
    is_dirty: is_repo_dirty(&wt_repo),
});
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_worktree_info_has_commit_message test_worktree_info_has_is_dirty test_worktree_dirty_when_file_added -- --nocapture`

Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test`

Expected: PASS (no regressions)

- [ ] **Step 6: Commit**

```bash
git add src/worktree/manager.rs tests/worktree_manager.rs
git commit -m "feat: add commit_message and is_dirty fields to WorktreeInfo"
```

---

### Task 2: Add `pr` field to WorktreeInfo and populate in `build_worktree_list`

**Files:**
- Modify: `src/worktree/manager.rs:9-24` (add pr field)
- Modify: `src/app.rs:160-172` (populate pr in build_worktree_list)
- Modify: `src/github.rs` (make PrState public derivable)

- [ ] **Step 1: Add `pr` field to WorktreeInfo**

In `src/worktree/manager.rs`, add to the struct:

```rust
pub pr: Option<(u32, crate::github::PrState)>,
```

And set `pr: None` in both push sites (main worktree and additional worktrees).

- [ ] **Step 2: Populate `pr` in `build_worktree_list()`**

In `src/app.rs`, inside `build_worktree_list()`, after the `Self::apply_pr_auto_status(...)` call (around line 171), add:

```rust
// Copy PR info for card display
if let Some(cache) = self.github_caches.get(root) {
    if let Some(pr_info) = cache.get(&wt.branch) {
        wt.pr = Some((pr_info.number, pr_info.state));
    }
}
```

- [ ] **Step 3: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/worktree/manager.rs src/app.rs
git commit -m "feat: add pr field to WorktreeInfo, populate from GitHub cache"
```

---

### Task 3: Add `Filter` and `OpenPR` actions to keys.rs

**Files:**
- Modify: `src/keys.rs:9-23` (Action enum)
- Modify: `src/keys.rs:40-52` (handle_key bindings)
- Test: `tests/keys.rs`

- [ ] **Step 1: Write failing tests**

Add to `tests/keys.rs`:

```rust
#[test]
fn test_slash_triggers_filter() {
    let action = handle_key(make_key(KeyCode::Char('/')), &Focus::Sidebar);
    assert!(matches!(action, Action::Filter));
}

#[test]
fn test_ctrl_g_triggers_open_pr() {
    let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    let action = handle_key(key, &Focus::Sidebar);
    assert!(matches!(action, Action::OpenPR));
}

#[test]
fn test_ctrl_g_noop_in_terminal() {
    let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    let action = handle_key(key, &Focus::Terminal);
    assert!(matches!(action, Action::TerminalInput(_)));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_slash_triggers_filter test_ctrl_g_triggers_open_pr test_ctrl_g_noop_in_terminal -- --nocapture`

Expected: FAIL — `no variant named Filter` / `no variant named OpenPR`

- [ ] **Step 3: Add Action variants and key bindings**

In `src/keys.rs`, add to the `Action` enum:

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
    StatusCycle,
    Filter,
    OpenPR,
    TerminalInput(KeyEvent),
    Quit,
    None,
}
```

In `handle_key`, add `Ctrl+G` handling before the focus match (after the `Ctrl+A` block, around line 38):

```rust
if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('g') {
    return match focus {
        Focus::Sidebar => Action::OpenPR,
        Focus::Terminal => Action::TerminalInput(key),
    };
}
```

In the `Focus::Sidebar` match arm, add `/`:

```rust
Focus::Sidebar => match key.code {
    KeyCode::Up => Action::SidebarUp,
    KeyCode::Down => Action::SidebarDown,
    KeyCode::Enter => Action::SidebarSelect,
    KeyCode::Char('n') => Action::SidebarCreate,
    KeyCode::Char('a') => Action::SidebarArchive,
    KeyCode::Char('s') => Action::StatusCycle,
    KeyCode::Char('/') => Action::Filter,
    KeyCode::Esc | KeyCode::Char('q') => Action::Quit,
    _ => Action::None,
},
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_slash_triggers_filter test_ctrl_g_triggers_open_pr test_ctrl_g_noop_in_terminal -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/keys.rs tests/keys.rs
git commit -m "feat: add Filter and OpenPR action variants with keybindings"
```

---

### Task 4: Implement filter state and action handler in App

**Files:**
- Modify: `src/app.rs` (App struct, handle_action, handle_dialog_key)

- [ ] **Step 1: Add filter state to App struct**

In `src/app.rs`, add to the `App` struct (around line 60):

```rust
pub filter: Option<String>,
```

Initialize in `App::new()` (around line 125):

```rust
filter: None,
```

- [ ] **Step 2: Add filter action handlers**

In `handle_action()`, add the `Filter` and `OpenPR` match arms (after `StatusCycle`):

```rust
Action::Filter => {
    self.filter = Some(String::new());
}
Action::OpenPR => {
    let idx = self.sidebar_state.selected;
    if idx < self.sidebar_state.worktrees.len() {
        let wt = &self.sidebar_state.worktrees[idx];
        if let Some(cache) = self.github_caches.get(&wt.repo_root) {
            if let Some(pr) = cache.get(&wt.branch) {
                let _ = std::process::Command::new("open")
                    .arg(&pr.url)
                    .spawn();
                self.flash(format!("Opened PR #{}", pr.number));
            } else {
                self.flash("No PR for this branch");
            }
        } else {
            self.flash("No PR for this branch");
        }
    }
}
```

- [ ] **Step 3: Add filter key handling in the event loop**

In the main event loop in `run()`, before the dialog key handling (around line 379), add filter input handling:

```rust
// Filter mode consumes keys
if self.filter.is_some() {
    match key.code {
        KeyCode::Esc => {
            self.filter = None;
        }
        KeyCode::Backspace => {
            if let Some(ref mut f) = self.filter {
                f.pop();
            }
        }
        KeyCode::Char(c) if !key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            if let Some(ref mut f) = self.filter {
                f.push(c);
            }
        }
        KeyCode::Enter => {
            // Exit filter mode but keep the filter text active
            // (user can press Esc to clear it instead)
            self.filter = None;
        }
        _ => {}
    }
    continue;
}
```

- [ ] **Step 4: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat: add filter state and OpenPR action handler"
```

---

### Task 5: Two-row card rendering in control_panel.rs

**Files:**
- Modify: `src/ui/control_panel.rs:86-136` (worktree rendering loop)
- Modify: `src/app.rs:183-206` (calculate_panel_width)

- [ ] **Step 1: Update card rendering to two lines**

In `src/ui/control_panel.rs`, replace the worktree rendering inside the `for (flat_idx, wt) in &group_wts` loop (lines 86-136). The new version builds a two-line `ListItem`:

```rust
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

    // Row 1: icon + name + tags (dirty, PR)
    let mut row1_spans = vec![
        Span::raw("  "),
        icon,
        Span::styled(display_name.clone(), name_style),
    ];

    // Build right-aligned tags
    let mut tags: Vec<Span> = Vec::new();
    if wt.is_dirty {
        tags.push(Span::styled(" M", Style::default().fg(Color::Yellow)));
    }
    if let Some((number, ref pr_state)) = wt.pr {
        let (label, color) = match pr_state {
            crate::github::PrState::Open => (format!(" #{}", number), Color::Green),
            crate::github::PrState::Draft => (format!(" #{} Draft", number), Color::Yellow),
            crate::github::PrState::Merged => (format!(" #{} Merged", number), Color::Magenta),
            crate::github::PrState::Closed => (format!(" #{} Closed", number), Color::Red),
        };
        tags.push(Span::styled(label, Style::default().fg(color)));
    }
    row1_spans.extend(tags);

    let line1 = Line::from(row1_spans);

    // Row 2: commit message + stats (ahead/behind/age)
    let msg = wt.commit_message.as_deref().unwrap_or("");
    let msg_style = Style::default().fg(Color::DarkGray);
    let mut row2_spans = vec![
        Span::raw("    "),
        Span::styled(msg.chars().take(40).collect::<String>(), msg_style),
    ];

    let mut stats: Vec<Span> = Vec::new();
    if wt.ahead > 0 {
        stats.push(Span::styled(format!(" \u{2191}{}", wt.ahead), Style::default().fg(Color::Cyan)));
    }
    if wt.behind > 0 {
        stats.push(Span::styled(format!(" \u{2193}{}", wt.behind), Style::default().fg(Color::Yellow)));
    }
    if wt.last_commit_age_secs < u64::MAX {
        let age = crate::worktree::format_age(wt.last_commit_age_secs);
        stats.push(Span::styled(format!(" {}", age), Style::default().fg(Color::DarkGray)));
    }
    row2_spans.extend(stats);

    let line2 = Line::from(row2_spans);

    flat_to_visual.push(items.len());
    items.push(ListItem::new(vec![line1, line2]));

    // Map BOTH rows to this flat_idx
    let abs_row = (inner.y + visual_row as u16) as usize;
    if state.row_to_flat_idx.len() <= abs_row + 1 {
        state.row_to_flat_idx.resize(abs_row + 2, None);
    }
    state.row_to_flat_idx[abs_row] = Some(*flat_idx);
    state.row_to_flat_idx[abs_row + 1] = Some(*flat_idx);
    visual_row += 2;
}
```

- [ ] **Step 2: Update sidebar width calculation**

In `src/app.rs`, update `calculate_panel_width()` to increase the minimum:

```rust
fn calculate_panel_width(&self) -> u16 {
    let max_name_len = self.sidebar_state.worktrees.iter()
        .map(|wt| {
            let display = wt.short_name.as_deref().unwrap_or(&wt.branch);
            let mut len = display.len();
            if let Some(ref rn) = wt.repo_name {
                len += rn.len() + 1;
            }
            // Account for tags: "M" + "#NNN Draft" etc
            if wt.is_dirty { len += 2; }
            if wt.pr.is_some() { len += 10; }
            len
        })
        .max()
        .unwrap_or(0);
    let width = (max_name_len + 8) as u16;
    width.clamp(28, 60)
}
```

- [ ] **Step 3: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/ui/control_panel.rs src/app.rs
git commit -m "feat: render two-row worktree cards with dirty/PR badges"
```

---

### Task 6: Footer bar replacing inline [+] button

**Files:**
- Modify: `src/ui/control_panel.rs:146-163` (remove [+] button)
- Modify: `src/ui/control_panel.rs:19-27` (render_control_panel — add footer rendering)
- Modify: `src/app.rs:494-498` (SidebarDown clamp)
- Modify: `src/app.rs:500-507` (SidebarSelect — remove [+] fallback)

- [ ] **Step 1: Remove the [+] button from the scrollable list**

In `src/ui/control_panel.rs`, delete the `[+] new worktree` block (lines 146-162). Also remove `plus_visual_idx` and the `else` branch in list_state selection (lines 169-171).

Replace the list_state logic:

```rust
let mut list_state = ListState::default();
if let Some(&visual_idx) = flat_to_visual.get(state.selected) {
    list_state.select(Some(visual_idx));
}
```

- [ ] **Step 2: Add footer rendering after the list**

In `render_control_panel()`, split the inner area into list + footer. Replace the current inner usage:

```rust
let inner = block.inner(area);
block.render(area, buf);

// Split inner into scrollable list + fixed footer
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
```

Use `list_area` instead of `inner` for the list rendering. Then render the footer:

```rust
// Render footer
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

// Right-align count
let count_span = Span::styled(&count_str, hint_style);
let count_x = footer_area.right().saturating_sub(count_str.len() as u16 + 1);
buf.set_line(count_x, footer_area.y, &Line::from(count_span), footer_area.width);
```

- [ ] **Step 3: Update SidebarDown clamp in app.rs**

In `src/app.rs`, change `SidebarDown` (around line 494):

```rust
Action::SidebarDown => {
    let max = self.sidebar_state.worktrees.len().saturating_sub(1);
    if self.sidebar_state.selected < max {
        self.sidebar_state.selected += 1;
    }
}
```

- [ ] **Step 4: Update SidebarSelect — remove [+] fallback**

In `src/app.rs`, change `SidebarSelect` (around line 500):

```rust
Action::SidebarSelect => {
    if self.sidebar_state.selected < self.sidebar_state.worktrees.len() {
        let size = crossterm::terminal::size()?;
        self.ensure_pty_for_selected(size.1, size.0)?;
        self.focus = Focus::Terminal;
    }
}
```

- [ ] **Step 5: Remove `show_plus` from ControlPanelState if unused**

Check if `show_plus` field is still referenced anywhere. If not, remove it from the struct and initialization.

- [ ] **Step 6: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/ui/control_panel.rs src/app.rs
git commit -m "feat: replace inline [+] button with fixed footer bar"
```

---

### Task 7: Consolidated detail bar

**Files:**
- Modify: `src/app.rs:251-330` (right pane rendering)

- [ ] **Step 1: Replace the header + info bar with a two-row detail bar**

In `src/app.rs`, replace the right pane layout and rendering (lines 251-330) with:

```rust
// Right pane: detail bar (2 rows) + terminal
let right_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(2),
        Constraint::Min(1),
    ])
    .split(chunks[1]);

// Render detail bar
if self.sidebar_state.selected < self.sidebar_state.worktrees.len() {
    let wt = &self.sidebar_state.worktrees[self.sidebar_state.selected];

    // Row 1: branch + status + PR + sync
    let mut row1_spans: Vec<Span> = vec![
        Span::styled(
            format!(" \u{2387} {} ", wt.branch),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
    ];

    let status_label = match wt.workflow_status {
        WorkflowStatus::Queued => "QUEUED",
        WorkflowStatus::InProgress => "IN PROGRESS",
        WorkflowStatus::InReview => "IN REVIEW",
        WorkflowStatus::Done => "DONE",
    };
    row1_spans.push(Span::styled(
        format!(" {} ", status_label),
        Style::default().fg(Color::DarkGray),
    ));

    if let Some(pr) = self.github_caches.get(&wt.repo_root).and_then(|c| c.get(&wt.branch)) {
        let (state_label, color) = match pr.state {
            crate::github::PrState::Open => ("Open", Color::Green),
            crate::github::PrState::Draft => ("Draft", Color::Yellow),
            crate::github::PrState::Merged => ("Merged", Color::Magenta),
            crate::github::PrState::Closed => ("Closed", Color::Red),
        };
        row1_spans.push(Span::styled(
            format!(" #{} {} ", pr.number, state_label),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }

    if wt.ahead > 0 {
        row1_spans.push(Span::styled(format!(" \u{2191}{}", wt.ahead), Style::default().fg(Color::Cyan)));
    }
    if wt.behind > 0 {
        row1_spans.push(Span::styled(format!(" \u{2193}{}", wt.behind), Style::default().fg(Color::Yellow)));
    }

    let detail_row1 = Line::from(row1_spans);

    // Row 2: path + PR URL
    let mut row2_spans: Vec<Span> = vec![
        Span::styled(
            format!(" {} ", wt.path.display()),
            Style::default().fg(Color::DarkGray),
        ),
    ];

    if let Some(pr) = self.github_caches.get(&wt.repo_root).and_then(|c| c.get(&wt.branch)) {
        row2_spans.push(Span::styled("\u{2502} ", Style::default().fg(Color::DarkGray)));
        row2_spans.push(Span::styled(pr.url.clone(), Style::default().fg(Color::DarkGray)));
    }

    let detail_row2 = Line::from(row2_spans);

    // Render into the detail area (2 rows)
    let detail_area = right_chunks[0];
    frame.render_widget(detail_row1, Rect { height: 1, ..detail_area });
    frame.render_widget(detail_row2, Rect {
        y: detail_area.y + 1,
        height: 1,
        ..detail_area
    });
}

// Render terminal in remaining space
let terminal_area = right_chunks[1];
```

Update the terminal rendering section to use `terminal_area` (replacing `right_chunks[2]`).

- [ ] **Step 2: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "feat: consolidate header and info bar into two-row detail bar"
```

---

### Task 8: Filter rendering in sidebar

**Files:**
- Modify: `src/ui/control_panel.rs` (add filter_text parameter, render filter row, apply filtering)
- Modify: `src/app.rs` (pass filter to render_control_panel)

- [ ] **Step 1: Add filter parameter to render_control_panel**

In `src/ui/control_panel.rs`, add `filter: &Option<String>` to the function signature:

```rust
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
```

- [ ] **Step 2: Render filter bar and apply filtering**

After the inner/list_area calculation, if filter is active, render a filter row and reduce list_area:

```rust
let mut list_area = list_area;
if let Some(ref filter_text) = filter {
    let filter_line = Line::from(vec![
        Span::styled(" \u{2315} ", Style::default().fg(Color::Cyan)),
        Span::styled(format!("{}_", filter_text), Style::default().fg(Color::Cyan)),
    ]);
    buf.set_line(list_area.x, list_area.y, &filter_line, list_area.width);
    list_area = Rect {
        y: list_area.y + 1,
        height: list_area.height.saturating_sub(1),
        ..list_area
    };
}
```

In the group rendering loop, filter worktrees when filter is active:

```rust
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
```

- [ ] **Step 3: Update the call site in app.rs**

In `src/app.rs`, update the `render_control_panel` call:

```rust
ui::render_control_panel(
    &mut self.sidebar_state,
    &self.dialog,
    chunks[0],
    frame.buffer_mut(),
    self.focus == Focus::Sidebar,
    self.spinner_frame,
    &pty_last_outputs,
    &self.filter,
);
```

- [ ] **Step 4: Update status bar to show filter hint**

In `build_status_line()`, add the `/` filter hint in the sidebar-focused status bar (after the `q quit` entry):

```rust
spans.push(sep.clone());
spans.push(Span::styled("/", key_style));
spans.push(Span::styled(" filter", label_style));
```

And add `Ctrl+G` hint:

```rust
spans.push(sep.clone());
spans.push(Span::styled("Ctrl+G", key_style));
spans.push(Span::styled(" PR", label_style));
```

- [ ] **Step 5: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/ui/control_panel.rs src/app.rs
git commit -m "feat: add filter bar with fuzzy matching in sidebar"
```

---

### Task 9: Section count badges

**Files:**
- Modify: `src/ui/control_panel.rs:73-84` (group header rendering)

- [ ] **Step 1: Add count badge to group headers**

In `src/ui/control_panel.rs`, replace the group header rendering:

```rust
let count = group_wts.len();
items.push(ListItem::new(Line::from(vec![
    Span::styled(
        format!(" {}", label),
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
    ),
    Span::styled(
        format!("  {}", count),
        Style::default().fg(Color::Gray),
    ),
])));
```

- [ ] **Step 2: Verify it compiles and tests pass**

Run: `cargo test`

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/ui/control_panel.rs
git commit -m "feat: add count badges to sidebar section headers"
```
