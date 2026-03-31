# Plan A: Feature Enrichment

Adds data, functionality, and layout changes to the Arbor sidebar and right pane. This plan is a prerequisite for Plan B (Visual Refresh) — it surfaces the data that Plan B will style.

## 1. Data Enrichment

### WorktreeInfo Expansion

Add two fields to `WorktreeInfo`:

- `commit_message: Option<String>` — HEAD commit summary, populated from `repo.head()?.peel_to_commit()?.summary()` during `WorktreeManager::list()`
- `is_dirty: bool` — whether the working tree has uncommitted changes, populated by checking `repo.statuses()` for any non-clean entries

Add PR data for card display:

- `pr: Option<(u32, PrState)>` — PR number and state, copied from `SharedGitHubCache` during `build_worktree_list()` alongside the existing `apply_pr_auto_status()` call

All three fields are cheap to compute during the existing list-building pass. No new dependencies.

## 2. Two-Row Cards

### Layout

Each worktree renders as a 2-line `ListItem` instead of 1-line:

```
  ⠹ agent-routing              M  #247
    feat: add multi-agent...   ↑3 ↓1 2h
```

- **Row 1:** status icon + short name (left-aligned), tags right-aligned — dirty `M` in yellow (if `is_dirty`), PR badge in state-colored text (if `pr` exists, e.g. `#247` or `#241 Draft`)
- **Row 2:** commit message truncated to fit (left-aligned), stats right-aligned — ahead/behind arrows + relative age

### Rendering Changes (control_panel.rs)

- Each worktree produces a 2-line `ListItem` with two `Line` values
- `row_to_flat_idx` mapping accounts for 2 rows per item
- `group_regions` tracking accounts for doubled row counts
- Mouse click targets cover both rows of each card

### Tag Rendering

Tags are `Span`s with colored text (no background — colored-bg text looks heavy in terminals):
- Dirty: `M` in `Color::Yellow`
- PR Open: `#NNN` in `Color::Green`
- PR Draft: `#NNN Draft` in `Color::Yellow`
- PR Merged: `#NNN Merged` in `Color::Magenta`
- PR Closed: `#NNN Closed` in `Color::Red`

Tags are right-aligned via padding calculation against the sidebar width.

### Sidebar Width

`calculate_panel_width()` needs updating — minimum width increases from 20 to ~28 to accommodate tags on row 1 and stats on row 2.

## 3. Footer Bar

### Replaces Inline [+] Button

The `[+] new worktree` item is removed from the scrollable list. A fixed 1-line footer renders below the scroll area.

### Layout

The sidebar `inner` area splits into two vertical constraints: `[Min(1), Length(1)]` — scrollable list on top, footer at bottom.

### Content

```
 [+]New  Archive  Status        8 wt
```

- `[+]New` in green (triggers create dialog, same as `n` key)
- `Archive` and `Status` as dim text hints
- Right-aligned worktree count

### Interaction

- Footer is not part of `ListState` selection
- `state.sel` range becomes `0..worktrees.len()-1` only — no more `sel == worktrees.len()` for the plus button
- `SidebarDown` clamps at the last worktree
- Mouse clicks on footer items trigger corresponding actions via extended `row_to_flat_idx`
- `n` key still opens create dialog directly (unchanged)

## 4. Consolidated Detail Bar

### Merges Header + Info Bar

Current layout: three right-pane chunks `[Length(1), Length(info_bar_height), Min(1)]`. New layout: two chunks `[Length(2), Min(1)]`.

### Row 1

```
⎇ feat/agent-routing  IN PROGRESS  #247 Open  ↑3 ↓1
```

- Branch name in cyan bold
- Status label from `WorkflowStatus`
- PR state + number (colored by state)
- Ahead/behind counts

### Row 2

```
~/.../fusion-platform-worktrees/feat/agent-routing │ github.com/.../pull/247
```

- Path in dim text
- PR URL in dim text (when PR exists)

Always renders 2 rows regardless of PR existence — empty spans when nothing to show. Removes the conditional `has_pr` logic.

## 5. Filter Bar

### Activation

`/` key enters filter mode when sidebar is focused. New `Action::Filter` in keys.rs.

### State

New `App` field: `filter: Option<String>`.

### Rendering

When active, a filter input row renders at the top of the sidebar (below the border title):

```
⌕ agent_
```

Styled like the create dialog input — cyan text with cursor indicator.

### Behavior

- The worktree list is filtered in the render pass: `worktrees.iter().filter(|wt| wt.branch.contains(&filter))` applied before grouping into status sections
- Typing appends to filter string
- `Backspace` removes last character
- `Esc` clears filter and returns to normal sidebar navigation
- Empty groups are hidden when filter is active

## 6. Open PR in Browser

### Activation

`Ctrl+G` when sidebar is focused. New `Action::OpenPR` in keys.rs.

### Implementation

- Look up PR URL from `github_caches` for the selected worktree's branch
- Call `std::process::Command::new("open").arg(&url).spawn()` (macOS `open` command)
- Flash message: `"Opened PR #247"` or `"No PR for this branch"`

## Files Changed

| File | Changes |
|------|---------|
| `src/worktree/manager.rs` | Add `commit_message`, `is_dirty` fields to `WorktreeInfo`, populate in `list()` via existing `commit_age_secs()` path and `repo.statuses()` |
| `src/app.rs` | Add `pr` field population in `build_worktree_list()`, add `filter` state, consolidate detail bar rendering, implement filter/open-PR actions, remove `[+]` from selection range |
| `src/ui/control_panel.rs` | Two-row card rendering, tag/badge spans, footer bar, filter input row, section count display |
| `src/keys.rs` | Add `Action::Filter` and `Action::OpenPR`, bind `/` and `Ctrl+G` |
| `src/github.rs` | No changes (data already available) |
| `src/persistence.rs` | No changes |
