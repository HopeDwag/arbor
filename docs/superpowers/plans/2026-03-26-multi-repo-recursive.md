# Multi-Repo Recursive Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow arbor to discover and manage worktrees across multiple git repositories when launched from any directory, using recursive scanning with depth limits and skip patterns.

**Architecture:** `main.rs` detects single-repo vs multi-repo mode by attempting `git2::Repository::discover()` first, falling back to recursive directory scanning. `App` owns a `HashMap<PathBuf, WorktreeManager>` instead of a single manager. `WorktreeInfo` gains `repo_name` and `repo_root` fields. The UI renders a `<repo>/` prefix on worktree names in multi-repo mode. Persistence stays per-repo.

**Tech Stack:** Rust, ratatui, crossterm, serde_json, git2

**Spec:** `docs/superpowers/specs/2026-03-26-multi-repo-recursive-design.md`

---

### Task 1: Add `repo_name` and `repo_root` to WorktreeInfo

**Files:**
- Modify: `src/worktree/manager.rs:8-18`
- Create: `tests/worktree_repo_fields.rs`

- [ ] **Step 1: Write failing test**

Create `tests/worktree_repo_fields.rs`:

```rust
use std::process::Command;
use tempfile::TempDir;

fn init_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init", dir.path().to_str().unwrap()])
        .output().unwrap();
    Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "config", "user.email", "test@test"])
        .output().unwrap();
    Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "config", "user.name", "Test"])
        .output().unwrap();
    Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "commit", "--allow-empty", "-m", "init"])
        .output().unwrap();
    dir
}

#[test]
fn test_worktree_info_has_repo_fields() {
    let dir = init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    assert_eq!(worktrees[0].repo_name, None);
    assert_eq!(worktrees[0].repo_root, dir.path().canonicalize().unwrap());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test worktree_repo_fields`
Expected: FAIL — `repo_name` and `repo_root` fields don't exist

- [ ] **Step 3: Add fields to WorktreeInfo**

In `src/worktree/manager.rs`, add to `WorktreeInfo`:
```rust
pub repo_name: Option<String>,
pub repo_root: PathBuf,
```

In `list()`, set `repo_name: None` and `repo_root: self.repo_root.clone()` for both WorktreeInfo constructions (main and non-main).

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src/worktree/manager.rs tests/worktree_repo_fields.rs
git commit -m "feat: add repo_name and repo_root fields to WorktreeInfo"
```

---

### Task 2: Implement recursive repo discovery

**Files:**
- Create: `src/discovery.rs`
- Modify: `src/lib.rs`
- Create: `tests/discovery.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/discovery.rs`:

```rust
use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn init_repo_in(parent: &std::path::Path, name: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    Command::new("git").args(["init", dir.to_str().unwrap()]).output().unwrap();
    Command::new("git").args(["-C", dir.to_str().unwrap(), "config", "user.email", "t@t"]).output().unwrap();
    Command::new("git").args(["-C", dir.to_str().unwrap(), "config", "user.name", "T"]).output().unwrap();
    Command::new("git").args(["-C", dir.to_str().unwrap(), "commit", "--allow-empty", "-m", "init"]).output().unwrap();
    dir
}

#[test]
fn test_discover_repos_finds_nested_repos() {
    let parent = TempDir::new().unwrap();
    let enablis = parent.path().join("Enablis");
    std::fs::create_dir_all(&enablis).unwrap();
    init_repo_in(&enablis, "arbor");
    init_repo_in(&enablis, "fusion");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 2);
    let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Enablis/arbor"));
    assert!(names.contains(&"Enablis/fusion"));
}

#[test]
fn test_discover_repos_skips_hidden_dirs() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "visible-repo");
    let hidden = parent.path().join(".hidden");
    std::fs::create_dir_all(&hidden).unwrap();
    init_repo_in(&hidden, "secret-repo");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "visible-repo");
}

#[test]
fn test_discover_repos_skips_node_modules() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "real-repo");
    let nm = parent.path().join("node_modules");
    std::fs::create_dir_all(&nm).unwrap();
    init_repo_in(&nm, "some-dep");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
}

#[test]
fn test_discover_repos_skips_worktree_dirs() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "my-repo");
    let wt = parent.path().join("my-repo-worktrees");
    std::fs::create_dir_all(&wt).unwrap();
    init_repo_in(&wt, "feature-a");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "my-repo");
}

#[test]
fn test_discover_repos_stops_at_git() {
    let parent = TempDir::new().unwrap();
    let repo = init_repo_in(parent.path(), "outer-repo");
    // Create a nested repo inside the outer one
    init_repo_in(&repo, "inner-repo");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "outer-repo");
}

#[test]
fn test_discover_repos_respects_max_depth() {
    let parent = TempDir::new().unwrap();
    // Depth 4 — beyond the limit of 3
    let deep = parent.path().join("a").join("b").join("c").join("d");
    std::fs::create_dir_all(&deep).unwrap();
    init_repo_in(&deep, "too-deep");
    // Depth 2 — within limit
    let shallow = parent.path().join("a");
    init_repo_in(&shallow, "ok-repo");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "a/ok-repo");
}

#[test]
fn test_discover_repos_empty_parent_errors() {
    let parent = TempDir::new().unwrap();
    let result = arbor::discovery::discover_repos(parent.path());
    assert!(result.is_err());
}

#[cfg(unix)]
#[test]
fn test_discover_repos_does_not_follow_symlinks() {
    let parent = TempDir::new().unwrap();
    let real = init_repo_in(parent.path(), "real-repo");
    std::os::unix::fs::symlink(&real, parent.path().join("linked-repo")).unwrap();

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "real-repo");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test discovery`
Expected: FAIL — `arbor::discovery` doesn't exist

- [ ] **Step 3: Implement discovery module**

Create `src/discovery.rs`:

```rust
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 3;

const SKIP_DIRS: &[&str] = &[
    "node_modules", "vendor", "target", "__pycache__", "build", "dist",
];

pub struct DiscoveredRepo {
    pub name: String,
    pub path: PathBuf,
}

pub fn discover_repos(root: &Path) -> Result<Vec<DiscoveredRepo>> {
    let mut repos = Vec::new();
    scan_dir(root, root, 0, &mut repos)?;

    if repos.is_empty() {
        bail!("No git repositories found in {}", root.display());
    }

    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repos)
}

fn scan_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    repos: &mut Vec<DiscoveredRepo>,
) -> Result<()> {
    if depth > MAX_DEPTH {
        return Ok(());
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()), // skip unreadable dirs
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Skip symlinks
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if ft.is_symlink() {
            continue;
        }
        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip hidden directories
        if dir_name.starts_with('.') {
            continue;
        }

        // Skip junk directories
        if SKIP_DIRS.contains(&dir_name.as_str()) {
            continue;
        }

        // Skip worktree sibling directories
        if dir_name.ends_with("-worktrees") {
            continue;
        }

        // Check if this is a git repo
        if path.join(".git").exists() {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let name = rel.to_string_lossy().replace('\\', "/");
            repos.push(DiscoveredRepo { name, path });
            // Don't recurse into repos
            continue;
        }

        // Recurse into subdirectory
        scan_dir(root, &path, depth + 1, repos)?;
    }

    Ok(())
}
```

Add `pub mod discovery;` to `src/lib.rs`.

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add src/discovery.rs src/lib.rs tests/discovery.rs
git commit -m "feat: add recursive repo discovery with depth limit and skip patterns"
```

---

### Task 3: Refactor App to support multiple WorktreeManagers

**Files:**
- Modify: `src/app.rs`
- Create: `tests/app_multi_repo.rs`

This is the largest task. `App` needs to own a `HashMap<PathBuf, WorktreeManager>` instead of a single manager, and all operations (create, archive, status cycle, persist) need to route to the correct manager via `WorktreeInfo.repo_root`.

- [ ] **Step 1: Write failing tests**

Create `tests/app_multi_repo.rs`:

```rust
use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn init_repo_in(parent: &std::path::Path, name: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    Command::new("git").args(["init", dir.to_str().unwrap()]).output().unwrap();
    Command::new("git").args(["-C", dir.to_str().unwrap(), "config", "user.email", "t@t"]).output().unwrap();
    Command::new("git").args(["-C", dir.to_str().unwrap(), "config", "user.name", "T"]).output().unwrap();
    Command::new("git").args(["-C", dir.to_str().unwrap(), "commit", "--allow-empty", "-m", "init"]).output().unwrap();
    dir
}

#[test]
fn test_multi_repo_app_lists_all_worktrees() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "alpha");
    init_repo_in(parent.path(), "beta");

    let app = arbor::app::App::new(parent.path()).unwrap();
    let mains: Vec<_> = app.sidebar_state.worktrees.iter().filter(|w| w.is_main).collect();
    assert_eq!(mains.len(), 2);
}

#[test]
fn test_multi_repo_worktrees_have_repo_name() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "alpha");
    init_repo_in(parent.path(), "beta");

    let app = arbor::app::App::new(parent.path()).unwrap();
    for wt in &app.sidebar_state.worktrees {
        assert!(wt.repo_name.is_some());
    }
    let names: Vec<&str> = app.sidebar_state.worktrees.iter()
        .map(|w| w.repo_name.as_deref().unwrap()).collect();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));
}

#[test]
fn test_single_repo_worktrees_have_no_repo_name() {
    let dir = TempDir::new().unwrap();
    Command::new("git").args(["init", dir.path().to_str().unwrap()]).output().unwrap();
    Command::new("git").args(["-C", dir.path().to_str().unwrap(), "config", "user.email", "t@t"]).output().unwrap();
    Command::new("git").args(["-C", dir.path().to_str().unwrap(), "config", "user.name", "T"]).output().unwrap();
    Command::new("git").args(["-C", dir.path().to_str().unwrap(), "commit", "--allow-empty", "-m", "init"]).output().unwrap();

    let app = arbor::app::App::new(dir.path()).unwrap();
    for wt in &app.sidebar_state.worktrees {
        assert!(wt.repo_name.is_none());
    }
}

#[test]
fn test_multi_repo_mains_pinned_to_in_progress() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "alpha");
    init_repo_in(parent.path(), "beta");

    let app = arbor::app::App::new(parent.path()).unwrap();
    for wt in &app.sidebar_state.worktrees {
        if wt.is_main {
            assert_eq!(wt.workflow_status, arbor::persistence::WorkflowStatus::InProgress);
        }
    }
}

#[test]
fn test_multi_repo_persistence_isolation() {
    let parent = TempDir::new().unwrap();
    let alpha_path = init_repo_in(parent.path(), "alpha");
    let beta_path = init_repo_in(parent.path(), "beta");

    let alpha_mgr = arbor::worktree::WorktreeManager::open(&alpha_path).unwrap();
    alpha_mgr.create("feature-a").unwrap();

    let mut app = arbor::app::App::new(parent.path()).unwrap();
    let idx = app.sidebar_state.worktrees.iter()
        .position(|w| w.branch == "feature-a").unwrap();
    app.sidebar_state.selected = idx;
    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();

    let alpha_config = arbor::persistence::ArborConfig::load(&alpha_path);
    assert!(alpha_config.worktrees.contains_key("feature-a"));

    let beta_config = arbor::persistence::ArborConfig::load(&beta_path);
    assert!(beta_config.worktrees.is_empty());
}

#[test]
fn test_multi_repo_skips_failed_repos_with_warning() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "good-repo");
    // Create a directory with a .git file (not dir) to simulate a broken repo
    let bad = parent.path().join("bad-repo");
    std::fs::create_dir_all(&bad).unwrap();
    std::fs::write(bad.join(".git"), "not a real git repo").unwrap();

    // Should succeed with just the good repo, not crash
    let app = arbor::app::App::new(parent.path()).unwrap();
    assert_eq!(app.sidebar_state.worktrees.len(), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test app_multi_repo`
Expected: FAIL — App::new doesn't support non-repo paths

- [ ] **Step 3: Refactor App struct and constructor**

In `src/app.rs`, replace the single manager field:

```rust
// Old:
worktree_mgr: WorktreeManager,
config: ArborConfig,
repo_root: PathBuf,

// New:
managers: HashMap<PathBuf, WorktreeManager>,
configs: HashMap<PathBuf, ArborConfig>,
multi_repo: bool,
scan_root: PathBuf,
```

Refactor `App::new()`:

```rust
pub fn new(path: &std::path::Path) -> Result<Self> {
    let (managers, multi_repo, scan_root) = if git2::Repository::discover(path).is_ok() {
        // Single-repo mode
        let mgr = WorktreeManager::open(path)?;
        let root = mgr.repo_root().to_path_buf();
        let mut map = HashMap::new();
        map.insert(root.clone(), mgr);
        (map, false, root)
    } else {
        // Multi-repo mode — recursive discovery
        let discovered = crate::discovery::discover_repos(path)?;
        let mut map = HashMap::new();
        for repo in discovered {
            match WorktreeManager::open(&repo.path) {
                Ok(mgr) => { map.insert(repo.path, mgr); }
                Err(e) => { eprintln!("arbor: skipping {}: {}", repo.name, e); }
            }
        }
        if map.is_empty() {
            anyhow::bail!("No valid git repositories found");
        }
        (map, true, path.to_path_buf())
    };

    // Load configs per repo
    let configs: HashMap<PathBuf, ArborConfig> = managers.keys()
        .map(|root| (root.clone(), ArborConfig::load(root)))
        .collect();

    // Build combined worktree list
    let mut worktrees = Vec::new();
    for (root, mgr) in &managers {
        let config = &configs[root];
        let github_cache = SharedGitHubCache::new(root);
        let mut wts = mgr.list()?;
        for wt in &mut wts {
            if wt.is_main {
                wt.workflow_status = WorkflowStatus::InProgress;
            } else if let Some(wt_config) = config.worktrees.get(&wt.branch) {
                wt.workflow_status = wt_config.status;
                wt.short_name = wt_config.short_name.clone();
            }
            if multi_repo {
                let rel = root.strip_prefix(&scan_root).unwrap_or(root);
                wt.repo_name = Some(rel.to_string_lossy().replace('\\', "/"));
            }
            // Apply PR auto-status...
        }
        worktrees.extend(wts);
    }
    // ... build App struct with managers, configs, multi_repo, scan_root
}
```

- [ ] **Step 4: Add helper to look up manager for a worktree**

```rust
fn manager_for(&self, repo_root: &Path) -> Option<&WorktreeManager> {
    self.managers.get(repo_root)
}

fn manager_for_mut(&mut self, repo_root: &Path) -> Option<&mut WorktreeManager> {
    self.managers.get_mut(repo_root)
}

fn config_for_mut(&mut self, repo_root: &Path) -> &mut ArborConfig {
    self.configs.entry(repo_root.to_path_buf()).or_default()
}
```

- [ ] **Step 5: Update operations to use routing helpers**

Replace all `self.worktree_mgr.create(...)` / `self.worktree_mgr.delete(...)` calls with:
```rust
let repo_root = self.sidebar_state.worktrees[self.sidebar_state.selected].repo_root.clone();
if let Some(mgr) = self.manager_for(&repo_root) {
    mgr.create(&branch)?;
}
```

Similarly for status cycle persistence:
```rust
let config = self.config_for_mut(&wt.repo_root);
let entry = config.worktrees.entry(wt.branch.clone()).or_default();
entry.status = wt.workflow_status;
let _ = config.save(&wt.repo_root);
```

Key methods to update: `handle_action` (StatusCycle, ArchiveConfirm, CreateConfirm), `handle_dialog_key` (Enter in create dialog), `apply_config`, and `rebuild_worktree_list` (new method to refresh the flat list from all managers).

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: all pass

- [ ] **Step 7: Commit**

```bash
git add src/app.rs tests/app_multi_repo.rs
git commit -m "feat: refactor App to support multiple WorktreeManagers"
```

---

### Task 4: Update main.rs for mode detection

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Simplify main.rs**

Remove `find_repo_root()`. `App::new()` now handles both modes. Update `--worktree` matching to support `repo/branch` format in multi-repo mode.

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all pass

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: simplify main.rs for single and multi-repo mode detection"
```

---

### Task 5: Update UI for multi-repo display and create dialog

**Files:**
- Modify: `src/ui/control_panel.rs`
- Modify: `src/app.rs` (Dialog enum)

- [ ] **Step 1: Update sidebar to show repo prefix**

In `control_panel.rs`, prepend `repo_name/` to the display name when `repo_name` is `Some`.

- [ ] **Step 2: Update header bar for repo context**

Add `📁 <repo_name>` to the terminal header bar when in multi-repo mode.

- [ ] **Step 3: Add repo selector to create dialog**

Add `DialogField::Repo` variant. In multi-repo mode, the create dialog shows a Repo field with Left/Right arrow cycling repos. Archived branches update when repo changes.

- [ ] **Step 4: Update archive dialog text**

Show `repo/branch` in the confirmation message when in multi-repo mode.

- [ ] **Step 5: Run all tests and clippy**

Run: `cargo test && cargo clippy`
Expected: all pass, no warnings

- [ ] **Step 6: Commit**

```bash
git add src/ui/control_panel.rs src/app.rs
git commit -m "feat: multi-repo UI — repo prefix, header tag, create dialog repo selector"
```

---

### Task 6: Integration test and UAT

**Files:**
- Modify: `uat/harness.sh` (add multi-repo seed function)
- Modify: `uat/run_tests.sh` (add multi-repo test scenarios)

- [ ] **Step 1: Add `_uat_seed_multi_repo` to harness**

Create a seed function that builds 2-3 repos in subdirectories of the temp dir, each with branches and worktrees.

- [ ] **Step 2: Add multi-repo UAT tests**

Add test scenarios:
- App launches in multi-repo mode, shows repos with prefix
- Navigation works across repos
- Create worktree in specific repo
- Status cycling persists to correct repo
- Archive works across repos

- [ ] **Step 3: Manual smoke test**

Run: `cargo run -- --repo ~/Repositories`
Verify: discovers repos, shows them with relative path prefixes, all operations work.

- [ ] **Step 4: Run full test suite**

Run: `cargo test && cargo clippy && ./uat/run_tests.sh`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add uat/harness.sh uat/run_tests.sh
git commit -m "feat: multi-repo UAT harness and test scenarios"
```
