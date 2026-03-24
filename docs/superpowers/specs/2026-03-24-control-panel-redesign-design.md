# Control Panel Redesign

## Overview

Redesign the sidebar into a "control panel" with status-grouped worktrees, drag-and-drop reordering, activity indicators, auto-sizing, optional short names, and persistent state.

## Status Groups

Three statuses, displayed as section headers in the control panel:

- **In Progress** — actively being worked on
- **Queued** — waiting to be worked on (default for new worktrees)
- **Done** — finished

Worktrees are listed under their status group header. Within each group, ordering follows the existing sort (main first, then by most recent commit).

## Control Panel Layout

```
┌ arbor ──────────────┐
│ IN PROGRESS          │
│  ⟳ auth              │
│  ! fix-login         │
│                      │
│ QUEUED               │
│  ▶ add-search        │
│  ▶ refactor-api      │
│                      │
│ DONE                 │
│  ✓ update-deps       │
│                      │
│  [+] new worktree    │
└──────────────────────┘
```

The main worktree is always shown at the top of its status group and cannot be dragged.

## Auto-sizing

The control panel calculates its width based on the longest displayed name (short name or branch name) plus fixed padding for icons, borders, and status indicators. Recalculates whenever the worktree list changes (create, delete, status change). Minimum width: 20 columns. Maximum width: 60 columns.

The manual drag-to-resize border handle is removed.

## Short Names

When creating a worktree, the dialog prompts for:

1. Branch name (or Tab to select archived branch, as existing)
2. Short name (optional) — a human-friendly label

If a short name is provided, it is displayed in the control panel. If omitted, the branch name is shown. The full branch name is always visible in the terminal header bar.

## Activity Indicators

Each worktree shows an activity icon to the left of its name. Detection is based on PTY output timing — a `last_output` timestamp is updated each time the PTY reader thread receives data.

On each render tick (~50ms):

| Condition | Icon | Meaning |
|-----------|------|---------|
| PTY output in last 500ms | `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` (animated spinner) | Terminal is busy |
| No PTY output for 500ms+ | `!` | Waiting for user input |
| No PTY session (queued/done) | `▶` / `✓` | Status icon only |

The spinner cycles through braille animation frames. The `!` icon is styled in yellow to draw attention.

## Drag and Drop

Users can click-and-hold a worktree item and drag it to a different status group. Implementation:

1. **Mouse down on a worktree item** — begins drag; store the dragged worktree index
2. **Mouse drag** — highlight the dragged item and show a drop target indicator on the status group header the cursor is over
3. **Mouse up over a status group header** — move the worktree to that status, persist the change
4. **Mouse up elsewhere** — cancel the drag

The main worktree cannot be dragged. The `[+] new worktree` button cannot be dragged.

## Create Dialog Changes

The existing create dialog adds a second field:

```
┌ New worktree ─────────┐
│ Branch: feature-auth_  │
│ Name:   auth_          │
│                        │
│ Tab: restore (2 arch.) │
│ Enter confirm · Esc    │
└────────────────────────┘
```

The "Name" field is optional. Pressing Enter with it blank uses the branch name as the display name.

## Persistence

State is stored in `.arbor.json` at the repo root:

```json
{
  "worktrees": {
    "feature-auth": {
      "status": "in_progress",
      "short_name": "auth"
    },
    "add-search": {
      "status": "queued"
    },
    "update-deps": {
      "status": "done"
    }
  }
}
```

- Loaded on startup via `App::new`
- Saved on every status change (drag-and-drop completion)
- Worktrees not present in the file default to `queued` status with no short name
- When a worktree is archived/deleted, its entry remains in the file (so restoring it preserves its short name and last status)

## Affected Modules

- **`app.rs`** — add drag state, persistence loading/saving, short name to create dialog, remove border drag logic, auto-size calculation
- **`ui/sidebar.rs`** — rename to `ui/control_panel.rs`, render status group headers, drag visual feedback, activity icons, new dialog field
- **`pty/session.rs`** — add `last_output` timestamp (updated by reader thread, read by render)
- **`worktree/manager.rs`** — `WorktreeInfo` gains `status` and `short_name` fields populated from `.arbor.json`
- **`keys.rs`** — no changes expected (drag is mouse-only, sidebar keys unchanged)
- **`main.rs`** — no changes expected

## Testing Strategy

- **Unit tests for persistence** — serialize/deserialize `.arbor.json`, handle missing file, handle unknown worktrees defaulting to queued
- **Integration tests for drag-and-drop** — simulate mouse down/drag/up sequences on `App`, verify status changes
- **Integration tests for auto-sizing** — create worktrees with various name lengths, verify calculated width
- **Integration tests for activity detection** — mock `last_output` timestamps relative to now, verify correct icon selection
- **Existing tests** — `worktree_manager.rs` and `app_mouse.rs` must continue to pass
