# Multi-Repo Support

## Overview

Allow arbor to be launched from a non-repo root directory and work across multiple git repositories simultaneously. Repos are discovered by scanning immediate child directories. Worktrees from all repos are displayed in the control panel grouped by workflow status (not by repo), with each worktree tagged by its repo name.

## Discovery

When arbor starts, it checks whether the current directory (or `--repo` path) is a git repository:

- **Is a git repo:** single-repo mode, works as today.
- **Is not a git repo:** multi-repo mode. Scans immediate child directories (one level deep, no symlink following) for git repositories (directories containing `.git`). Non-repo children are silently ignored. If a child is a git repo but `WorktreeManager::open()` fails, it is skipped with a warning to stderr. If no repos are found at all, exits with an error message.

Discovery happens once at startup. Repos are not re-scanned during the session.

**Control flow change in `main.rs`:** The current `find_repo_root()` calls `git2::Repository::discover()` which fails on non-repo directories. This function is restructured: attempt `discover()` first; if it fails, try scanning children. `App::new()` receives either a single repo path or a list of repo paths depending on the mode detected.

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

**Sorting within groups:** All main worktrees are pinned to "In Progress" (same rule as the control panel spec — main worktrees cannot change status). Within each group, main worktrees appear first sorted alphabetically by repo name, then remaining worktrees sorted by most recent commit across all repos.

**Default selection on startup:** Index 0 in the flat combined list (the first main worktree alphabetically by repo name in the "In Progress" group). The `--worktree` flag overrides this. In multi-repo mode, `--worktree arbor/feature-auth` matches exactly; `--worktree feature-auth` matches the first worktree with that branch across repos (alphabetical repo order as tiebreaker).

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

**Repo field:** Shows the currently selected repo. Left/Right arrow cycles through discovered repos (alphabetical order). The archived branches list updates when the repo changes. Defaults to the repo of the currently selected worktree, or the first repo alphabetically if `[+]` is selected.

**Field navigation:** Up/Down arrow moves between all three fields (Repo → Branch → Name). Tab on the Branch field cycles archived branches (as in control panel spec). Tab on Repo and Name fields is a no-op.

**Dialog state is always freshly initialized** when opened — it does not remember values from a previous cancelled dialog.

In single-repo mode, the Repo field is not shown (dialog has two fields as per control panel spec).

## WorktreeManager Changes

Currently `App` owns a single `WorktreeManager`. In multi-repo mode, `App` owns a `HashMap<PathBuf, WorktreeManager>` keyed by repo root path. Each manager handles its own repo's worktrees.

`App::new()` changes:
- If the path is a git repo: create one `WorktreeManager` (as today)
- If not: scan children, create a `WorktreeManager` for each discovered repo

The flat worktree list is built by `App` collecting `list()` results from all managers and tagging each `WorktreeInfo` with its repo name. The `WorktreeManager` itself has no awareness of multi-repo mode — `App` owns the tagging.

**Performance note:** `list()` calls `check_status()` (git dirty/clean) for every worktree, which can be slow with many repos. For the initial implementation, this is called on every focus switch (same as current behavior). If this becomes a bottleneck, caching can be added later.

## WorktreeInfo Changes

`WorktreeInfo` gains two fields:
- `repo_name: Option<String>` — set to `Some("repo-dir-name")` in multi-repo mode, `None` in single-repo mode. Used by the UI to render the `<repo>/` prefix.
- `repo_root: PathBuf` — the repo root path, used by `App` to route operations to the correct `WorktreeManager`. In single-repo mode, this is the same as the single manager's root.

## Navigation and Selection

The selection model is unchanged — `selected` is an index into the flat combined worktree list. The flat list is ordered by status group, then by the sorting rules within each group. Keyboard and mouse navigation work identically to single-repo mode.

## Drag and Drop (Multi-Repo)

Drag-and-drop works the same — dragging a worktree to a different status group changes its `workflow_status` and persists to its repo's `.arbor.json`. Cross-repo drag has no special meaning (you're changing status, not moving worktrees between repos).

## Archive/Delete (Multi-Repo)

Archiving a worktree in multi-repo mode works the same — `WorktreeManager::delete()` is called on the correct manager (looked up via `WorktreeInfo.repo_root`). The confirmation dialog shows the repo name for clarity: "Remove arbor/feature-auth? (y/n)".

## CLI Changes

No new CLI flags needed. All existing flags (`--repo`, `--worktree`, `--toggle-key`) are preserved unchanged.

- `--repo /path/to/git-repo` → single-repo mode
- `--repo /path/to/parent-dir` → multi-repo mode (scans children)
- `--worktree` in multi-repo mode: matches `<repo>/<branch>` exactly, or `<branch>` with alphabetical repo tiebreaker

## Affected Modules

- **`app.rs`** — own `HashMap<PathBuf, WorktreeManager>` instead of single manager. Build combined flat list from all managers (tagging `repo_name` and `repo_root`). Route create/delete/persist to correct manager via `repo_root`. Add repo selector to create dialog. Update archive confirm text.
- **`main.rs`** — restructure `find_repo_root()` to detect git-repo vs parent-dir. Pass single path or list of paths to `App::new()`.
- **`worktree/manager.rs`** — `WorktreeInfo` gains `repo_name: Option<String>` and `repo_root: PathBuf`. No other changes to manager logic.
- **`ui/control_panel.rs`** — render `<repo>/` prefix when `repo_name` is `Some`. Update create dialog for repo field (3 fields with Up/Down). Update archive dialog text.
- **`keys.rs`** — no changes expected
- **`pty/session.rs`** — no changes expected

## Testing Strategy

- **Discovery tests** — create temp dir with multiple git repos as children, verify all are found. Verify non-repo children are ignored. Verify corrupted repo is skipped with warning. Verify empty parent errors. Verify symlinks are not followed.
- **Combined list tests** — multiple managers, verify flat list merges correctly with status grouping and sorting across repos. Verify all mains are pinned to "In Progress".
- **Persistence isolation tests** — change status of worktree in repo A, verify only repo A's `.arbor.json` is updated.
- **Create dialog tests** — verify repo selector cycles repos, verify archived branches update per repo, verify dialog state resets on reopen.
- **Single-repo backward compat** — verify launching on a git repo works identically to current behavior (no repo prefix, no repo selector, single WorktreeManager).
- **CLI tests** — `--worktree` flag matches across repos with disambiguation, `--repo` detects mode correctly.
