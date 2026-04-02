# Multi-Repo Support (Recursive Discovery)

Supersedes: `2026-03-24-multi-repo-support-design.md` — updates discovery from single-level to recursive scan with guardrails.

## Overview

Allow arbor to be launched from any directory and work across multiple git repositories simultaneously. Repos are discovered by recursively scanning subdirectories. Worktrees from all repos are displayed in the control panel grouped by workflow status (not by repo), with each worktree tagged by its repo's relative path.

## Discovery

When arbor starts, it checks whether the current directory (or `--repo` path) is a git repository:

- **Is a git repo:** single-repo mode, works as today.
- **Is not a git repo:** multi-repo mode. Recursively scans subdirectories for git repositories (directories containing `.git`).

### Recursive scan guardrails

- **Max depth:** 3 levels from the starting directory
- **Skip hidden directories:** anything starting with `.`
- **Skip junk directories:** `node_modules`, `vendor`, `target`, `__pycache__`, `build`, `dist`
- **Skip worktree sibling directories:** anything matching `*-worktrees` (Arbor's own worktree dirs)
- **Stop descending into repos:** once a `.git` is found, that directory is a repo — don't recurse into it
- **No symlink following**
- **Failed repos skipped:** if `WorktreeManager::open()` fails on a discovered repo, skip it with a warning to stderr
- **No repos found:** exit with error message

Discovery happens once at startup. Repos are not re-scanned during the session.

### Example

From `~/Repositories` with this structure:
```
Repositories/
├── Enablis/
│   ├── arbor/          (git repo)
│   ├── fusion-platform/ (git repo)
│   └── fusion-dashboard/ (git repo)
├── Nexus/
│   └── galaxy/         (git repo)
└── os/
    └── claude_code/    (git repo)
```

Discovers: `Enablis/arbor`, `Enablis/fusion-platform`, `Enablis/fusion-dashboard`, `Nexus/galaxy`, `os/claude_code`.

## Repo Naming

In multi-repo mode, `repo_name` is the **relative path from the scan root** to the repo directory. This disambiguates repos with the same directory name under different parents (e.g. `Enablis/fusion-dashboard` vs `Enablis-2/fusion-dashboard`).

**Display name format:** `<repo_name>/<short_name or branch>`. Examples:
- `Enablis/arbor/main`
- `Nexus/galaxy/feature-auth`

In single-repo mode, no prefix (unchanged from current behavior).

## Control Panel Layout (Multi-Repo)

Status groups remain the top-level organiser. Worktrees are tagged with their repo path:

```
┌ arbor ──────────────────────────┐
│ IN PROGRESS                      │
│  ⟳ Enablis/arbor/main           │
│  ! Enablis/fusion-platform/main │
│  · Nexus/galaxy/auth            │
│                                  │
│ QUEUED                           │
│  ▶ Enablis/arbor/search         │
│  ▶ Nexus/galaxy/nav             │
│                                  │
│  [+] new worktree               │
└──────────────────────────────────┘
```

**Sorting within groups:** Mains first (sorted alphabetically by repo_name), then non-mains sorted by most recent commit. All mains pinned to IN PROGRESS.

**Default selection:** Index 0 (first main worktree alphabetically). `--worktree` overrides: `--worktree Enablis/arbor/feature-auth` matches exactly; `--worktree feature-auth` matches first worktree with that branch (alphabetical repo order tiebreaker).

## Terminal Header Bar (Multi-Repo)

Shows repo context:
```
 /Users/me/Repositories/Enablis/arbor-worktrees/feature-auth  ⎇ feature-auth  📁 Enablis/arbor
```

## Auto-sizing

Width calculation accounts for the `<repo>/` prefix. Multi-repo names are longer, so the panel will typically be wider. Same min/max bounds (20–60 columns).

## PTY Sessions

No changes. Each worktree gets its own PTY. The `pty_sessions` HashMap key is `PathBuf` (worktree path), which already works across repos.

## Persistence

Each repo owns its own `.arbor.json` in its repo root. In multi-repo mode, config is loaded per repo. Status changes write only to that repo's `.arbor.json`. Per-repo state survives independently — running arbor on a single repo later sees the same state.

## Create Dialog (Multi-Repo)

Adds a repo selector:

```
┌ New worktree ─────────────────────┐
│ Repo:   Enablis/arbor              │
│ Branch: feature-auth_              │
│ Name:   auth_                      │
│                                    │
│ Tab: restore (2 arch.)             │
│ ↑/↓ fields · ←/→ repo · Enter · Esc│
└────────────────────────────────────┘
```

**Repo field:** Left/Right arrow cycles discovered repos (alphabetical). Archived branches update when repo changes. Defaults to repo of currently selected worktree, or first repo if `[+]` is selected.

**Field navigation:** Up/Down moves between Repo → Branch → Name. Tab on Branch cycles archived branches. In single-repo mode, Repo field hidden (two fields as today).

## Architecture Changes

**`App`** owns `HashMap<PathBuf, WorktreeManager>` (single entry in single-repo mode, many in multi-repo). Boolean `multi_repo` flag controls UI behavior. `WorktreeManager` itself has no awareness of multi-repo — `App` owns the tagging.

**`WorktreeInfo`** gains:
- `repo_name: Option<String>` — relative path from scan root in multi-repo mode, `None` in single-repo
- `repo_root: PathBuf` — repo root path, used to route operations to correct manager

**New module: `discovery.rs`** — recursive repo scanner with guardrails.

**Routing:** All operations (create, archive, status cycle, persist) look up the correct `WorktreeManager` via `WorktreeInfo.repo_root`.

## CLI Changes

No new flags. Existing flags preserved:
- `--repo /path/to/git-repo` → single-repo mode
- `--repo /path/to/any-dir` → multi-repo mode (recursive scan)
- No `--repo` → uses current directory (either mode)

## Testing Strategy

- **Discovery tests** — recursive scan finds repos at multiple depths. Hidden/junk/worktree dirs skipped. Symlinks not followed. Empty parent errors. Failed repos skipped with warning.
- **Combined list tests** — multiple managers, flat list merges with status grouping across repos. Mains pinned to IN PROGRESS.
- **Persistence isolation** — status change in repo A only writes repo A's `.arbor.json`.
- **Create dialog** — repo selector cycles, archived branches update per repo.
- **Single-repo backward compat** — no repo prefix, no repo selector, identical to current behavior.
- **UAT tests** — update harness for multi-repo smoke tests.
