# GitHub Integration — PR Status & Remote Tracking

## Overview

Show PR status and ahead/behind remote tracking for each worktree in the control panel.

## Display

Each worktree line in the control panel appends remote and PR info to the right:

```
 IN PROGRESS
  ⟳ auth              ↑2 ↓1   #42
  ! fix-login         ↑3      #43
  · main
```

### Ahead/Behind Remote

- `↑N` — N commits ahead of remote (Cyan)
- `↓N` — N commits behind remote (Yellow)
- Omitted if 0 in either direction
- Omitted entirely if branch has no upstream

Calculated via `git2` — compare local branch HEAD to upstream tracking branch.

### PR Status

Uses Nerd Font icons with GitHub-style colors:

| State | Icon | Color |
|-------|------|-------|
| Open |  | Green |
| Draft |  | Yellow |
| Merged |  | Magenta |
| Closed |  | Red |
| No PR | (nothing) | — |

The `#N` PR number is an OSC 8 hyperlink to the PR URL on GitHub.

## Data Sources

### Ahead/Behind — git2

`git2::Repository::graph_ahead_behind(local_oid, upstream_oid)` returns `(ahead, behind)`. Called per worktree when building the worktree list. Already have `git2` as a dependency.

### PR Status — `gh` CLI

Shell out to `gh pr list --json number,headRefName,state,isDraft,url --limit 100` once on startup and cache the result. Map branch names to PR data.

**Refresh strategy:** Cache refreshed when:
- App starts
- User creates or deletes a worktree
- User presses `r` (new keybinding for manual refresh)

**Error handling:** If `gh` is not installed or fails, PR column is simply omitted. No error displayed — this is a nice-to-have overlay.

## Caching

A `GitHubCache` struct holds:
- `prs: HashMap<String, PrInfo>` — branch name → PR info
- `last_refresh: Instant`

`PrInfo`:
- `number: u32`
- `state: PrState` (Open, Draft, Merged, Closed)
- `url: String`

## Affected Modules

- **Create: `src/github.rs`** — `GitHubCache`, `PrInfo`, `PrState`, refresh logic (shell out to `gh`)
- **Modify: `src/worktree/status.rs`** — add `ahead_behind(repo, branch)` function using git2
- **Modify: `src/worktree/manager.rs`** — `WorktreeInfo` gains `ahead: u32`, `behind: u32`, `pr: Option<PrInfo>` fields
- **Modify: `src/ui/control_panel.rs`** — render ahead/behind and PR status after worktree name
- **Modify: `src/app.rs`** — own `GitHubCache`, pass PR data to worktree list building, add `r` key for refresh
- **Modify: `src/keys.rs`** — add `Refresh` action on `r`
- **Modify: `src/lib.rs`** — add `pub mod github`

## Status Bar

Add `r refresh` hint to the sidebar status bar.

## Testing

- **Unit tests for github.rs** — parse `gh` JSON output, handle missing `gh`, handle empty output
- **Unit tests for ahead_behind** — test with git2 in temp repos with diverged branches
- **Integration tests** — verify PrInfo is None when no `gh` available
