# Multi-Repo Support

## Overview

Allow arbor to be launched from a non-repo root directory and work across multiple git repositories simultaneously. Repos are discovered by scanning immediate child directories. Worktrees from all repos are displayed in the control panel grouped by workflow status (not by repo), with each worktree tagged by its repo name.

## Discovery

When arbor starts, it checks whether the current directory (or `--repo` path) is a git repository:

- **Is a git repo:** single-repo mode, works as today.
- **Is not a git repo:** multi-repo mode. Scans immediate child directories (one level deep) for git repositories (directories containing `.git`). Non-repo children are ignored. If no repos are found, exits with an error message.

Discovery happens once at startup. Repos are not re-scanned during the session.

## Control Panel Layout (Multi-Repo)

Status groups remain the top-level organiser, same as the control panel redesign spec. Worktrees are tagged with their repo name to distinguish them:

```
┌ arbor ──────────────────┐
│ IN PROGRESS              │
│  ⟳ arbor/auth            │
│  ! fusion-platform/login │
│                          │
│ QUEUED                   │
│  ▶ arbor/search          │
│  ▶ fusion-dashboard/nav  │
│                          │
│ DONE                     │
│  ✓ arbor/deps            │
│                          │
│  [+] new worktree        │
└──────────────────────────┘
```

**Display name format:** `<repo>/<short_name or branch>`. The repo name is the directory name (e.g., `arbor`, `fusion-platform`). In single-repo mode, the repo prefix is omitted (no change from current behavior).

**Sorting within groups:** Main worktrees from all repos are pinned first (sorted alphabetically by repo name), then remaining worktrees sorted by most recent commit across all repos.

## Terminal Header Bar (Multi-Repo)

The header bar shows repo context when in multi-repo mode:

```
 /Users/me/repos/arbor/arbor-worktrees/feature-auth  ⎇ feature-auth  📁 arbor
```

The repo name tag is appended to the existing header.

## Auto-sizing (Multi-Repo Impact)

The width calculation accounts for the `<repo>/` prefix. In multi-repo mode, display names are longer, so the control panel will typically be wider. The same min/max bounds apply (20–60 columns).

## PTY Sessions (Multi-Repo)

Each worktree gets its own PTY session, same as single-repo mode. The PTY spawns a shell in that worktree's directory. The `pty_sessions` HashMap key is already `PathBuf` (the worktree path), so this works across repos without changes.

## Persistence (Multi-Repo)

Each repo owns its own `.arbor.json` in its repo root. In multi-repo mode, arbor loads `.arbor.json` from each discovered repo independently. When a worktree's status changes, only that repo's `.arbor.json` is updated.

This means:
- Moving a worktree between status groups writes to its repo's `.arbor.json`
- Each repo's state survives independently
- Running arbor on a single repo later sees the same state

## Create Dialog (Multi-Repo)

When pressing `n` in multi-repo mode, the dialog adds a repo selector:

```
┌ New worktree ─────────────┐
│ Repo:   arbor              │
│ Branch: feature-auth_      │
│ Name:   auth_              │
│                            │
│ Tab: restore (2 arch.)     │
│ ↑/↓ fields · Enter · Esc  │
└────────────────────────────┘
```

**Repo field:** Shows the currently selected repo. Left/Right arrow cycles through discovered repos. The archived branches list updates when the repo changes. Defaults to the repo of the currently selected worktree, or the first repo alphabetically if `[+]` is selected.

In single-repo mode, the Repo field is not shown.

## WorktreeManager Changes

Currently `App` owns a single `WorktreeManager`. In multi-repo mode, `App` owns a `HashMap<PathBuf, WorktreeManager>` keyed by repo root path. Each manager handles its own repo's worktrees.

`App::new()` changes:
- If the path is a git repo: create one `WorktreeManager` (as today)
- If not: scan children, create a `WorktreeManager` for each discovered repo

The flat worktree list is built by collecting `list()` results from all managers, tagging each `WorktreeInfo` with its repo root and repo display name.

## WorktreeInfo Changes

`WorktreeInfo` gains a `repo_name: Option<String>` field. Set to `Some("repo-dir-name")` in multi-repo mode, `None` in single-repo mode. Used by the UI to render the `<repo>/` prefix.

## Navigation and Selection

The selection model is unchanged — `selected` is an index into the flat combined worktree list. The flat list is ordered by status group, then by the sorting rules within each group. Keyboard and mouse navigation work identically to single-repo mode.

## Drag and Drop (Multi-Repo)

Drag-and-drop works the same — dragging a worktree to a different status group changes its `workflow_status` and persists to its repo's `.arbor.json`. Cross-repo drag has no special meaning (you're changing status, not moving worktrees between repos).

## Archive/Delete (Multi-Repo)

Archiving a worktree in multi-repo mode works the same — `WorktreeManager::delete()` is called on the correct manager (looked up by the worktree's repo root). The confirmation dialog shows the repo name for clarity: "Remove arbor/feature-auth? (y/n)".

## CLI Changes

No new CLI flags needed. The existing `--repo` flag works for both modes:
- `--repo /path/to/git-repo` → single-repo mode
- `--repo /path/to/parent-dir` → multi-repo mode (scans children)

The `--worktree` flag in multi-repo mode matches against `<repo>/<branch>` or just `<branch>` (selects first match across repos).

## Affected Modules

- **`app.rs`** — own `HashMap<PathBuf, WorktreeManager>` instead of single manager. Build combined flat list from all managers. Route create/delete/persist to correct manager. Add repo selector to create dialog. Update archive confirm text.
- **`main.rs`** — detect git-repo vs parent-dir in `find_repo_root()`, pass discovery info to `App::new()`
- **`worktree/manager.rs`** — `WorktreeInfo` gains `repo_name: Option<String>`. No other changes.
- **`ui/control_panel.rs`** — render `<repo>/` prefix when `repo_name` is `Some`. Update create dialog for repo field. Update archive dialog text.
- **`keys.rs`** — no changes expected
- **`pty/session.rs`** — no changes expected

## Testing Strategy

- **Discovery tests** — create temp dir with multiple git repos as children, verify all are found. Verify non-repo children are ignored. Verify empty parent errors.
- **Combined list tests** — multiple managers, verify flat list merges correctly with status grouping and sorting across repos.
- **Persistence isolation tests** — change status of worktree in repo A, verify only repo A's `.arbor.json` is updated.
- **Create dialog tests** — verify repo selector cycles repos, verify archived branches update per repo.
- **Single-repo backward compat** — verify launching on a git repo works identically to current behavior (no repo prefix, no repo selector).
- **CLI tests** — `--worktree` flag matches across repos, `--repo` detects mode correctly.
