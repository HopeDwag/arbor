# Promote/Demote: Swap Worktrees into Main Workspace

## Problem

Worktrees live in sidecar directories (`<repo>-worktrees/<branch>/`), but IDEs, test scripts, and local dev servers are configured against the main repo directory. When you need to work on a feature in your IDE or test it locally, you have to manually `git checkout` in the main workspace — losing any uncommitted state in the process.

## Solution

A "promote" operation that swaps a sidecar worktree's branch into the main workspace, preserving dirty changes from both sides. A "demote" operation reverses it.

## Promote Operation

Triggered by pressing `p` on a sidecar worktree in the sidebar.

### Steps

1. **Check: no other worktree is already promoted** — only one promoted worktree per repo at a time. If one exists, show "demote current first."
2. **Stash main's uncommitted changes** (if any) — tagged with `arbor-promote:<branch>` so we can identify it later.
3. **Stash sidecar's uncommitted changes** (if any) — tagged with `arbor-sidecar:<branch>`.
4. **Checkout the target branch in the main directory** — `git checkout <branch>` in the main repo root.
5. **Pop main's stash** — restore main's dirty changes on top of the new branch. These are the user's existing work-in-progress that should persist.
6. **Attempt to pop sidecar's stash** — only if it applies cleanly. If conflicts occur, leave it stashed and notify: "Sidecar changes stashed. Apply manually with `git stash pop`."
7. **Mark the worktree as promoted** in `.arbor.json` and update the sidebar.

### After Promote

- The main workspace has the sidecar's branch checked out, with main's dirty changes preserved and sidecar's dirty changes applied (if clean)
- The sidecar directory stays on disk but is inactive — its entry in the sidebar shows a promoted icon
- The main worktree entry in the sidebar updates to show the new branch

## Demote Operation

Triggered by pressing `p` on a promoted worktree (toggle behavior).

### Steps

1. **Stash any uncommitted changes in main** — tagged with `arbor-demote:<branch>`.
2. **Checkout the previous branch** (stored in `.arbor.json`) in main.
3. **Pop the stash onto the sidecar** — same safe-pop logic: only if clean, otherwise leave stashed with notification.
4. **Remove promoted marker** from `.arbor.json` and update the sidebar.

## UI Changes

### Sidebar

Promoted worktrees display a distinct icon (`★`) to show they're currently in the main workspace. The main worktree entry updates its branch display to reflect what's actually checked out.

### Status Bar

When a promoted worktree exists, the keybinding hint shows `p demote` instead of `p promote`.

### Keybinding

- `p` on a sidecar worktree → promote (opens confirmation dialog)
- `p` on a promoted worktree → demote
- `p` on the main worktree entry → no-op

### Confirmation Dialog

```
Promote to main workspace?
Switch galaxy/ to branch chore/rbac-scripts-tidy
[main has uncommitted changes — they will be preserved]
Enter confirm · Esc cancel
```

If main is clean, the uncommitted changes line is omitted.

## Persistence

Promoted state stored in `.arbor.json`:

```json
{
  "worktrees": {
    "feature-auth": {
      "status": "in_progress",
      "promoted": true,
      "previous_branch": "main"
    }
  }
}
```

`promoted` and `previous_branch` tell Arbor on startup that this branch is in the main workspace and what to restore on demote. Only one worktree per repo can have `promoted: true`.

Sidecar stashes that couldn't be popped are regular git stashes — Arbor doesn't track them. The user manages them via `git stash list` / `git stash pop`.

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Sidecar stash pop conflicts on promote | Changes stay stashed, user notified |
| Checkout fails (branch deleted, etc.) | Abort, restore main's stash, show error |
| Main stash pop fails after checkout | Abort, checkout original branch, pop stash, show error |
| Another worktree already promoted | Block — "demote current first" |
| Demote when main is dirty | Stash and pop onto sidecar (same safe-pop logic) |

## Git Operations

All operations use `git2` (no shelling out):

- `repo.stash_save()` — create stash with message
- `repo.checkout_tree()` + `repo.set_head()` — branch checkout
- `repo.stash_pop()` — apply and drop stash (with conflict detection via `CheckoutBuilder`)
- `repo.statuses()` — check if dirty

### Stash identity

Stashes are LIFO — the most recently created stash is always at index 0. Since promote/demote creates a stash and immediately pops it (in the same operation, before the user can create other stashes), always use index 0. No need to search by message or store the index.

The tagged message (e.g. `arbor-promote:feature-auth`) is for human readability in `git stash list`, not for lookup.

### Branch sharing constraint

Git does not allow two worktrees to have the same branch checked out. This affects demote: main has the promoted branch, and the sidecar's HEAD also points to it. To handle this:

1. On promote, after checking out the target branch in main, **detach the sidecar's HEAD** (`git2::Repository::set_head_detached()` on the sidecar repo). The sidecar is inactive anyway — detaching avoids the conflict.
2. On demote, before checking out the promoted branch back on the sidecar, **checkout the previous branch in main first** (which frees up the promoted branch for the sidecar).

### Sidecar operations

When popping a stash "onto the sidecar" during demote:
1. Open a `git2::Repository` against the sidecar path
2. Checkout the promoted branch there (now free since main released it)
3. Call `stash_pop(0)` on that repository to restore dirty changes
4. If conflicts, leave stashed and notify the user

## Scope

- Single-repo mode: works as described
- Multi-repo mode: promote/demote operates on the repo of the selected worktree, looked up via `WorktreeInfo.repo_root`
- One promoted worktree per repo (not per Arbor session)

## Fallback Approaches

If the git stash-based approach proves unreliable in practice, documented alternatives:

**Approach B: Symlink swap** — make the main directory a symlink to whichever worktree is active. Instant, but breaks tools that don't follow symlinks and causes IDE path caching issues.

**Approach C: rsync copy** — copy the sidecar's working tree over main, ignoring `.git`. Simple but slow for large repos, doesn't preserve git index state, no clean reversal.

## Affected Modules

- **`src/app.rs`** — new `Action::Promote` / `Action::Demote`, confirmation dialog, stash/checkout/pop logic
- **`src/keys.rs`** — `p` key mapping
- **`src/persistence.rs`** — `promoted: bool` and `previous_branch: Option<String>` on `WorktreeConfig`
- **`src/ui/control_panel.rs`** — promoted icon rendering, updated status bar hints
- **`src/worktree/manager.rs`** — new methods for stash/checkout/pop operations

## Testing Strategy

- **Unit tests**: stash-checkout-pop sequence on temp repos, conflict detection, dirty state checks
- **Persistence tests**: promoted flag survives restart, only one promoted per repo
- **Error tests**: promote blocked when another is promoted, checkout failure rolls back
- **UAT**: promote a worktree, verify main has the branch, demote back, verify restored
