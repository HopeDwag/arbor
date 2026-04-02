# Promote/Demote Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to swap a sidecar worktree's branch into the main workspace (promote) and back (demote), preserving dirty state via git stash.

**Architecture:** A new `src/promote.rs` module handles the git stash/checkout/pop mechanics. `persistence.rs` gains `promoted` and `previous_branch` fields on `WorktreeConfig`. `keys.rs` adds `Action::Promote`. `app.rs` adds a confirmation dialog and calls into the promote module. The promote module uses `git2` for all operations (no shelling out).

**Tech Stack:** Rust, git2, serde_json

**Spec:** `docs/superpowers/specs/2026-03-31-promote-demote-design.md`

---

### Task 1: Add persistence fields for promoted state

**Files:**
- Modify: `src/persistence.rs`
- Create: `tests/persistence_promote.rs`

- [ ] **Step 1: Write failing test**

Create `tests/persistence_promote.rs`:

```rust
use arbor::persistence::{ArborConfig, WorktreeConfig, WorkflowStatus};

#[test]
fn test_promoted_field_roundtrips() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut config = ArborConfig::default();
    let mut wt = WorktreeConfig::default();
    wt.promoted = true;
    wt.previous_branch = Some("main".to_string());
    config.worktrees.insert("feature-auth".to_string(), wt);

    config.save(dir.path()).unwrap();
    let loaded = ArborConfig::load(dir.path());
    let wt = loaded.worktrees.get("feature-auth").unwrap();
    assert!(wt.promoted);
    assert_eq!(wt.previous_branch.as_deref(), Some("main"));
}

#[test]
fn test_promoted_defaults_to_false() {
    let config = WorktreeConfig::default();
    assert!(!config.promoted);
    assert!(config.previous_branch.is_none());
}

#[test]
fn test_promoted_not_serialized_when_false() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut config = ArborConfig::default();
    config.worktrees.insert("feat".to_string(), WorktreeConfig::default());
    config.save(dir.path()).unwrap();

    let json = std::fs::read_to_string(dir.path().join(".arbor.json")).unwrap();
    assert!(!json.contains("promoted"));
    assert!(!json.contains("previous_branch"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test persistence_promote`
Expected: FAIL — `promoted` field doesn't exist

- [ ] **Step 3: Add fields to WorktreeConfig**

In `src/persistence.rs`, add to `WorktreeConfig`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    pub status: WorkflowStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub promoted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_branch: Option<String>,
}
```

Update `Default`:
```rust
impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            status: WorkflowStatus::Queued,
            short_name: None,
            promoted: false,
            previous_branch: None,
        }
    }
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src/persistence.rs tests/persistence_promote.rs
git commit -m "feat: add promoted and previous_branch fields to WorktreeConfig"
```

---

### Task 2: Implement promote/demote git operations

**Files:**
- Create: `src/promote.rs`
- Modify: `src/lib.rs`
- Create: `tests/promote.rs`

This is the core logic — stash, checkout, pop, detach sidecar HEAD.

**Key insight:** Git worktrees share the same `.git` object store, which means **stashes are shared**. A stash created via `sidecar_repo.stash_save()` is visible from `main_repo.stash_pop()` because both Repository handles reference the same underlying repo. The stash stack is LIFO, so ordering matters: stash sidecar first (goes to index 0), then main (pushes sidecar to index 1). After checkout, pop index 0 (main's stash), then pop index 0 again (sidecar's stash, now at top).

**Rollback:** If checkout fails after stashing, pop the stashes back and return an error. If `stash_pop` fails (conflicts), leave it stashed and warn the user.

- [ ] **Step 1: Write failing tests**

Create `tests/promote.rs`:

```rust
use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn init_repo_with_worktree() -> (TempDir, PathBuf, PathBuf) {
    let dir = TempDir::new().unwrap();
    let main_path = dir.path().join("repo");
    std::fs::create_dir_all(&main_path).unwrap();
    Command::new("git").args(["init", main_path.to_str().unwrap()]).output().unwrap();
    Command::new("git").args(["-C", main_path.to_str().unwrap(), "config", "user.email", "t@t"]).output().unwrap();
    Command::new("git").args(["-C", main_path.to_str().unwrap(), "config", "user.name", "T"]).output().unwrap();
    // Create initial file and commit
    std::fs::write(main_path.join("file.txt"), "hello").unwrap();
    Command::new("git").args(["-C", main_path.to_str().unwrap(), "add", "."]).output().unwrap();
    Command::new("git").args(["-C", main_path.to_str().unwrap(), "commit", "-m", "init"]).output().unwrap();
    // Create feature branch with a change
    Command::new("git").args(["-C", main_path.to_str().unwrap(), "checkout", "-b", "feature-x"]).output().unwrap();
    std::fs::write(main_path.join("feature.txt"), "new feature").unwrap();
    Command::new("git").args(["-C", main_path.to_str().unwrap(), "add", "."]).output().unwrap();
    Command::new("git").args(["-C", main_path.to_str().unwrap(), "commit", "-m", "add feature"]).output().unwrap();
    Command::new("git").args(["-C", main_path.to_str().unwrap(), "checkout", "main"]).output().unwrap();
    // Create worktree
    let wt_path = dir.path().join("repo-worktrees").join("feature-x");
    std::fs::create_dir_all(wt_path.parent().unwrap()).unwrap();
    Command::new("git").args(["-C", main_path.to_str().unwrap(), "worktree", "add", wt_path.to_str().unwrap(), "feature-x"]).output().unwrap();
    (dir, main_path, wt_path)
}

#[test]
fn test_promote_switches_main_to_feature_branch() {
    let (_dir, main_path, wt_path) = init_repo_with_worktree();
    arbor::promote::promote(&main_path, &wt_path, "feature-x").unwrap();
    let repo = git2::Repository::open(&main_path).unwrap();
    let branch = repo.head().unwrap().shorthand().unwrap().to_string();
    assert_eq!(branch, "feature-x");
}

#[test]
fn test_promote_preserves_main_dirty_changes() {
    let (_dir, main_path, wt_path) = init_repo_with_worktree();
    std::fs::write(main_path.join("dirty.txt"), "wip").unwrap();
    arbor::promote::promote(&main_path, &wt_path, "feature-x").unwrap();
    assert!(main_path.join("dirty.txt").exists());
    assert_eq!(std::fs::read_to_string(main_path.join("dirty.txt")).unwrap(), "wip");
}

#[test]
fn test_promote_applies_sidecar_dirty_changes() {
    let (_dir, main_path, wt_path) = init_repo_with_worktree();
    std::fs::write(wt_path.join("sidecar-wip.txt"), "testing").unwrap();
    arbor::promote::promote(&main_path, &wt_path, "feature-x").unwrap();
    assert!(main_path.join("sidecar-wip.txt").exists());
}

#[test]
fn test_promote_detaches_sidecar_head() {
    let (_dir, main_path, wt_path) = init_repo_with_worktree();
    arbor::promote::promote(&main_path, &wt_path, "feature-x").unwrap();
    let wt_repo = git2::Repository::open(&wt_path).unwrap();
    assert!(wt_repo.head_detached().unwrap());
}

#[test]
fn test_demote_restores_previous_branch() {
    let (_dir, main_path, wt_path) = init_repo_with_worktree();
    arbor::promote::promote(&main_path, &wt_path, "feature-x").unwrap();
    arbor::promote::demote(&main_path, &wt_path, "feature-x", "main").unwrap();
    let repo = git2::Repository::open(&main_path).unwrap();
    let branch = repo.head().unwrap().shorthand().unwrap().to_string();
    assert_eq!(branch, "main");
}

#[test]
fn test_demote_restores_sidecar_branch() {
    let (_dir, main_path, wt_path) = init_repo_with_worktree();
    arbor::promote::promote(&main_path, &wt_path, "feature-x").unwrap();
    arbor::promote::demote(&main_path, &wt_path, "feature-x", "main").unwrap();
    let wt_repo = git2::Repository::open(&wt_path).unwrap();
    assert!(!wt_repo.head_detached().unwrap());
    let branch = wt_repo.head().unwrap().shorthand().unwrap().to_string();
    assert_eq!(branch, "feature-x");
}

#[test]
fn test_full_promote_demote_with_dirty_state() {
    let (_dir, main_path, wt_path) = init_repo_with_worktree();
    // Both sides dirty
    std::fs::write(main_path.join("main-wip.txt"), "main work").unwrap();
    std::fs::write(wt_path.join("sidecar-wip.txt"), "sidecar work").unwrap();
    arbor::promote::promote(&main_path, &wt_path, "feature-x").unwrap();
    // Both dirty files should be in main now
    assert!(main_path.join("main-wip.txt").exists());
    assert!(main_path.join("sidecar-wip.txt").exists());
    // Demote
    arbor::promote::demote(&main_path, &wt_path, "feature-x", "main").unwrap();
    // main-wip and sidecar-wip should now be in sidecar
    // (they were all stashed from main during demote)
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test promote`
Expected: FAIL — `arbor::promote` doesn't exist

- [ ] **Step 3: Implement promote module**

Create `src/promote.rs`:

```rust
use anyhow::{Context, Result};
use git2::{Repository, StashFlags};
use std::path::Path;

/// Promote a sidecar worktree's branch into the main workspace.
///
/// Stash ordering (LIFO, shared across worktrees):
/// 1. Stash sidecar dirty changes (if any) → index 0
/// 2. Stash main dirty changes (if any) → index 0 (sidecar now at 1)
/// 3. Detach sidecar HEAD (avoids "branch in use" error)
/// 4. Checkout target branch in main
/// 5. Pop index 0 = main's stash (restores main's WIP on new branch)
/// 6. Pop index 0 = sidecar's stash (applies sidecar changes if clean)
pub fn promote(
    main_path: &Path,
    sidecar_path: &Path,
    target_branch: &str,
) -> Result<()> {
    let main_repo = Repository::open(main_path).context("Cannot open main repo")?;
    let sidecar_repo = Repository::open(sidecar_path).context("Cannot open sidecar repo")?;

    let main_dirty = !main_repo.statuses(None)?.is_empty();
    let sidecar_dirty = !sidecar_repo.statuses(None)?.is_empty();

    // Step 1: Stash sidecar FIRST (goes to index 0, will be pushed down)
    let sidecar_stashed = if sidecar_dirty {
        let sig = sidecar_repo.signature()?;
        sidecar_repo.stash_save(
            &sig,
            &format!("arbor-sidecar:{}", target_branch),
            Some(StashFlags::INCLUDE_UNTRACKED),
        )?;
        true
    } else {
        false
    };

    // Step 2: Stash main (goes to index 0, sidecar now at index 1)
    let main_stashed = if main_dirty {
        let sig = main_repo.signature()?;
        main_repo.stash_save(
            &sig,
            &format!("arbor-promote:{}", target_branch),
            Some(StashFlags::INCLUDE_UNTRACKED),
        )?;
        true
    } else {
        false
    };

    // Step 3: Detach sidecar HEAD
    let sidecar_head_oid = sidecar_repo.head()?.target()
        .context("Sidecar HEAD has no target")?;
    sidecar_repo.set_head_detached(sidecar_head_oid)?;

    // Step 4: Checkout target branch in main
    let branch_ref = main_repo
        .find_branch(target_branch, git2::BranchType::Local)
        .context("Target branch not found")?;
    let refname = branch_ref.get().name().context("Invalid branch ref")?.to_string();

    if let Err(e) = (|| -> Result<()> {
        main_repo.set_head(&refname)?;
        main_repo.checkout_head(Some(
            git2::build::CheckoutBuilder::new().force(),
        ))?;
        Ok(())
    })() {
        // Rollback: pop stashes back in reverse order
        if main_stashed {
            let _ = main_repo.stash_pop(0, None);
        }
        if sidecar_stashed {
            let _ = sidecar_repo.stash_pop(0, None);
        }
        return Err(e).context("Checkout failed, rolled back stashes");
    }

    // Step 5: Pop main's stash (index 0) — restores main's WIP on new branch
    if main_stashed {
        if let Err(e) = main_repo.stash_pop(0, None) {
            eprintln!("arbor: warning: could not restore main's changes: {}", e);
            eprintln!("arbor: your changes are in `git stash list`");
        }
    }

    // Step 6: Pop sidecar's stash (now at index 0) onto main — only if clean
    if sidecar_stashed {
        if let Err(_) = main_repo.stash_pop(0, None) {
            eprintln!("arbor: sidecar changes stashed. Apply manually with `git stash pop`");
        }
    }

    Ok(())
}

/// Demote: restore main to its previous branch, move promoted branch back to sidecar.
///
/// Stash ordering:
/// 1. Stash main dirty changes → index 0
/// 2. Checkout previous branch in main (frees promoted branch)
/// 3. Re-attach sidecar to promoted branch
/// 4. Pop stash (index 0) onto sidecar
pub fn demote(
    main_path: &Path,
    sidecar_path: &Path,
    promoted_branch: &str,
    previous_branch: &str,
) -> Result<()> {
    let main_repo = Repository::open(main_path).context("Cannot open main repo")?;

    let main_dirty = !main_repo.statuses(None)?.is_empty();

    // Step 1: Stash main's dirty changes
    let stashed = if main_dirty {
        let sig = main_repo.signature()?;
        main_repo.stash_save(
            &sig,
            &format!("arbor-demote:{}", promoted_branch),
            Some(StashFlags::INCLUDE_UNTRACKED),
        )?;
        true
    } else {
        false
    };

    // Step 2: Checkout previous branch in main
    let branch_ref = main_repo
        .find_branch(previous_branch, git2::BranchType::Local)
        .context("Previous branch not found")?;
    let refname = branch_ref.get().name().context("Invalid branch ref")?.to_string();
    main_repo.set_head(&refname)?;
    main_repo.checkout_head(Some(
        git2::build::CheckoutBuilder::new().force(),
    ))?;

    // Step 3: Re-attach sidecar to the promoted branch
    let sidecar_repo = Repository::open(sidecar_path).context("Cannot open sidecar repo")?;
    let sidecar_branch = sidecar_repo
        .find_branch(promoted_branch, git2::BranchType::Local)
        .context("Promoted branch not found in sidecar")?;
    let sidecar_refname = sidecar_branch.get().name()
        .context("Invalid sidecar branch ref")?.to_string();
    sidecar_repo.set_head(&sidecar_refname)?;
    sidecar_repo.checkout_head(Some(
        git2::build::CheckoutBuilder::new().force(),
    ))?;

    // Step 4: Pop stash onto sidecar (stash is shared, index 0 is main's stash)
    if stashed {
        if let Err(_) = sidecar_repo.stash_pop(0, None) {
            eprintln!("arbor: changes stashed. Apply in sidecar with `git stash pop`");
        }
    }

    Ok(())
}
```

Add `pub mod promote;` to `src/lib.rs`.

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src/promote.rs src/lib.rs tests/promote.rs
git commit -m "feat: implement promote/demote git operations"
```

---

### Task 3: Wire promote/demote into the TUI

**Files:**
- Modify: `src/keys.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Add Action::Promote to keys.rs**

Add `Promote` variant to the `Action` enum. Map `p` key in sidebar:

```rust
KeyCode::Char('p') => Action::Promote,
```

- [ ] **Step 2: Add promote confirmation dialog to app.rs**

Add `Dialog::PromoteConfirm` variant:

```rust
PromoteConfirm {
    worktree_idx: usize,
    branch: String,
    repo_root: PathBuf,
    main_dirty: bool,
},
```

In `handle_action` for `Action::Promote`:
- If selected worktree is main → no-op
- If selected worktree is already promoted → trigger demote
- If another worktree is promoted → show "demote current first"
- Otherwise → show PromoteConfirm dialog

In `handle_dialog_key` for `PromoteConfirm`:
- Enter → call `promote::promote()`. On `Err`, show error in status bar (store in `self.status_message: Option<String>`, render in status bar, clear on next keypress). On `Ok`, update config and rebuild worktree list.
- Esc → cancel

- [ ] **Step 3: Add `promoted` field to WorktreeInfo**

Add `pub promoted: bool` to `WorktreeInfo` in `src/worktree/manager.rs`. Default to `false` in `list()`. Populate from `ArborConfig` in `build_worktree_list()` (same place where `workflow_status` and `short_name` are applied from config):

```rust
if let Some(wt_config) = config.worktrees.get(&wt.branch) {
    wt.workflow_status = wt_config.status;
    wt.short_name = wt_config.short_name.clone();
    wt.promoted = wt_config.promoted; // NEW
}
```

- [ ] **Step 4: Handle promoted state in config**

After promote, set `promoted: true` and `previous_branch` in the config for that worktree. After demote, clear them. Rebuild the worktree list after both operations.

- [ ] **Step 4: Run all tests**

Run: `cargo test && cargo clippy`
Expected: all pass, no warnings

- [ ] **Step 5: Commit**

```bash
git add src/keys.rs src/app.rs
git commit -m "feat: wire promote/demote into TUI with confirmation dialog"
```

---

### Task 4: Update sidebar to show promoted state

**Files:**
- Modify: `src/ui/control_panel.rs`
- Modify: `src/app.rs` (status bar hints)

- [ ] **Step 1: Show promoted icon in sidebar**

In `control_panel.rs`, when rendering a worktree where `wt.promoted` is `true` (populated from config in `build_worktree_list`), use `★` icon instead of the normal status icon.

- [ ] **Step 2: Update status bar keybinding hints**

When a promoted worktree is selected, show `p demote` instead of `p promote`. When main is selected, don't show `p` at all.

- [ ] **Step 3: Render promote confirmation dialog**

Add rendering for `Dialog::PromoteConfirm` in the control panel, showing:
```
Promote to main workspace?
Switch <main_name>/ to <branch>
[main has uncommitted changes — preserved]
Enter confirm · Esc cancel
```

- [ ] **Step 4: Run all tests and clippy**

Run: `cargo test && cargo clippy`
Expected: all pass, no warnings

- [ ] **Step 5: Commit**

```bash
git add src/ui/control_panel.rs src/app.rs
git commit -m "feat: show promoted icon and dialog in sidebar"
```

---

### Task 5: UAT and smoke test

**Files:**
- Modify: `uat/run_tests.sh`

- [ ] **Step 1: Add promote/demote UAT test**

Add test scenario that:
1. Creates a worktree
2. Makes it dirty (write a file)
3. Presses `p` on it → confirm
4. Verifies main switches to that branch (header bar changes)
5. Presses `p` again → demote
6. Verifies main returns to original branch

- [ ] **Step 2: Manual smoke test**

Run Arbor against a real repo, promote a worktree, verify IDE sees the new branch, demote back.

- [ ] **Step 3: Run full test suite**

Run: `cargo test && cargo clippy && ./uat/run_tests.sh`
Expected: all pass

- [ ] **Step 4: Commit**

```bash
git add uat/run_tests.sh
git commit -m "feat: add promote/demote UAT test"
```

---

### Fallback Approaches (if stash-based promote proves unreliable)

**Approach B: Symlink swap** — make the main directory a symlink to the active worktree. Instant but breaks tools that don't follow symlinks.

**Approach C: rsync copy** — copy the sidecar's working tree over main (ignoring `.git`). Simple but slow, doesn't preserve git index.
