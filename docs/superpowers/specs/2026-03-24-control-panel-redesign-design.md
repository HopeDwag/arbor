# Control Panel Redesign

## Overview

Redesign the sidebar into a "control panel" with status-grouped worktrees, drag-and-drop reordering, activity indicators, auto-sizing, optional short names, and persistent state.

## Status Groups

Three statuses, displayed as section headers in the control panel:

- **In Progress** — actively being worked on
- **Queued** — waiting to be worked on (default for new worktrees)
- **Done** — finished

The **main worktree** always defaults to "In Progress" and cannot have its status changed. It is pinned first within its group regardless of commit age.

Worktrees are listed under their status group header. Within each group, ordering is by most recent commit (after pinned items). Users cannot manually reorder within a group — ordering is always automatic.

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

**Keyboard navigation:** `j`/`k` moves through worktree items and the `[+]` button — group headers are skipped. When moving past the last item in a group, the cursor jumps to the first item in the next non-empty group (or `[+]`). Empty groups are skipped entirely.

**Keyboard status change:** `s` cycles the selected worktree's status: Queued → In Progress → Done → Queued. Does not apply to the main worktree (pressing `s` on main is a no-op).

## Auto-sizing

The control panel calculates its width based on the longest displayed name (short name or branch name) plus fixed padding for icons, borders, and status indicators. Group header text ("IN PROGRESS" = 11 chars) is always shorter than the minimum width, so it never drives the calculation. Recalculates whenever the worktree list changes (create, delete, status change). Minimum width: 20 columns. Maximum width: 60 columns.

The manual drag-to-resize border handle is removed. The `<`/`>` keyboard resize bindings and `SidebarResizeLeft`/`SidebarResizeRight` actions are also removed.

## Short Names

When creating a worktree, the dialog prompts for:

1. Branch name (or Tab to cycle archived branches, as existing)
2. Short name (optional) — a human-friendly label

If a short name is provided, it is displayed in the control panel. If omitted, the branch name is shown. The full branch name is always visible in the terminal header bar.

**Validation:** Short names have no character restrictions (spaces and unicode are allowed). Max length: 20 characters. Duplicates are allowed — they are display labels, not identifiers.

**Immutability:** Short names are set at creation time only. Editing short names after creation is out of scope for this design.

## Activity Indicators

Each worktree shows an activity icon to the left of its name. Detection is based on PTY output timing — a `last_output` field of type `Arc<AtomicU64>` is added to `PtySession`, storing the timestamp (epoch millis) of the last PTY output. The reader thread updates this atomically on each read. The render loop reads it without locking.

A **global spinner frame counter** (a `u8` on `App`) advances by 1 on every render tick. All busy worktrees display the same spinner phase — the frame index is `counter % 10` into the braille frames array.

On each render tick (~50ms):

| Condition | Icon | Meaning |
|-----------|------|---------|
| PTY output in last 500ms | `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` (animated spinner) | Terminal is busy |
| No PTY output for 500ms+ | `!` (Yellow) | Waiting for user input |
| No PTY session | `▶` / `✓` | Status icon only |

## PTY Lifecycle on Status Change

PTY sessions are **not** killed when a worktree's status changes. A worktree moved to "Done" keeps its PTY session alive — selecting it still shows the terminal. The activity icon reflects the actual PTY state regardless of status group. This keeps things simple and avoids losing terminal state.

## Drag and Drop

Users can click-and-hold a worktree item and drag it to a different status group. Implementation:

1. **Mouse down on a worktree item** — begins drag; store the dragged worktree index. Mouse down on the main worktree or `[+]` button is ignored (no drag state initiated).
2. **Mouse drag** — highlight the dragged item (Cyan background) and show a drop target indicator (Yellow background) on the status group region the cursor is currently over.
3. **Mouse up within a status group region** — move the worktree to that status, persist the change. The drop target is the entire vertical region of a group (from its header to the line before the next header), not just the header line.
4. **Mouse up outside any group region** — cancel the drag.

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

**Field navigation:** Down/Up arrow moves focus between the Branch and Name fields. Tab on the Branch field cycles archived branches (as existing). Tab on the Name field is a no-op.

The "Name" field is optional (max 20 chars). Pressing Enter with it blank uses the branch name as the display name.

## Persistence

State is stored in `.arbor.json` at the repo root. The **key is the git worktree name** (i.e., `WorktreeInfo.name`), not the branch name — these can differ.

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
- Saved on every status change (drag-and-drop or `s` key) and on worktree creation (to persist short name)
- Worktrees not present in the file default to `queued` status with no short name, **except** the main worktree which always defaults to `in_progress`
- When a worktree is archived/deleted, its entry remains in the file (so restoring it preserves its short name and last status)
- **Error handling:** If `.arbor.json` is missing or contains malformed JSON, treat it as empty (all defaults). Log a warning to stderr on parse failure. No file locking — last write wins if multiple instances run.

## Affected Modules

- **`app.rs`** — add drag state, persistence loading/saving, short name to create dialog, remove border drag logic, auto-size calculation, `s` key handling, global spinner counter, update status bar hints to include `s`
- **`ui/sidebar.rs`** — rename to `ui/control_panel.rs`, render status group headers, drag visual feedback, activity icons, new dialog field
- **`pty/session.rs`** — add `last_output: Arc<AtomicU64>` field, updated by reader thread, exposed via a `last_output_millis()` method
- **`worktree/manager.rs`** — `WorktreeInfo` gains `workflow_status` and `short_name` fields (note: `workflow_status` avoids collision with the existing `status: Option<WorktreeStatus>` field which tracks git dirty/clean state). Populated from `.arbor.json`.
- **`keys.rs`** — remove `SidebarResizeLeft`/`SidebarResizeRight`, add `StatusCycle` action on `s`
- **`main.rs`** — no changes expected

## Testing Strategy

- **Unit tests for persistence** — serialize/deserialize `.arbor.json`, handle missing file, handle malformed JSON, handle unknown worktrees defaulting to queued, main worktree always in_progress
- **Integration tests for drag-and-drop** — simulate mouse down/drag/up sequences on `App`, verify status changes. Verify main worktree drag is ignored.
- **Integration tests for keyboard status cycle** — simulate `s` key, verify status transitions, verify main worktree is immune
- **Integration tests for auto-sizing** — create worktrees with various name lengths, verify calculated width
- **Integration tests for activity detection** — set `last_output` timestamps relative to now, verify correct icon selection
- **Existing tests** — `worktree_manager.rs` and `app_mouse.rs` must continue to pass
