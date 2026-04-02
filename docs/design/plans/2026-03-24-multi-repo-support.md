# Multi-Repo Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow arbor to discover and manage worktrees across multiple git repositories when launched from a non-repo parent directory.

**Architecture:** `main.rs` detects single-repo vs multi-repo mode by attempting `git2::Repository::discover()` first, falling back to scanning child directories. `App` owns a `HashMap<PathBuf, WorktreeManager>` instead of a single manager. `WorktreeInfo` gains `repo_name` and `repo_root` fields. The UI renders a `<repo>/` prefix on worktree names in multi-repo mode. Persistence stays per-repo (each repo's `.arbor.json`).

**Tech Stack:** Rust, ratatui, crossterm, serde_json, git2

**Spec:** `docs/superpowers/specs/2026-03-24-multi-repo-support-design.md`

**Prerequisite:** Control panel redesign plan must be completed first.

---

### Task 1: Add repo_name and repo_root to WorktreeInfo

**Files:**
- Modify: `src/worktree/manager.rs:7-13`
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
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "commit", "--allow-empty", "-m", "init"])
        .output()
        .unwrap();
    dir
}

#[test]
fn test_worktree_info_has_repo_fields() {
    let dir = init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    // In single-repo mode, repo_name should be None
    assert_eq!(worktrees[0].repo_name, None);
    // repo_root should be the repo path
    assert_eq!(worktrees[0].repo_root, dir.path().to_path_buf());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test worktree_repo_fields`
Expected: FAIL — `repo_name` field doesn't exist

- [ ] **Step 3: Add fields to WorktreeInfo**

In `src/worktree/manager.rs`, add to `WorktreeInfo`:

```rust
pub struct WorktreeInfo {
    pub name: String,
    pub branch: String,
    pub path: PathBuf,
    pub is_main: bool,
    pub status: Option<WorktreeStatus>,
    pub workflow_status: WorkflowStatus,
    pub short_name: Option<String>,
    pub repo_name: Option<String>,
    pub repo_root: PathBuf,
}
```

In `list()`, add to both `WorktreeInfo` constructions:

```rust
repo_name: None,
repo_root: self.repo_root.clone(),
```

Add a public accessor for `repo_root`:

```rust
pub fn repo_root(&self) -> &Path {
    &self.repo_root
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/worktree/manager.rs tests/worktree_repo_fields.rs
git commit -m "feat: add repo_name and repo_root fields to WorktreeInfo"
```

---

### Task 2: Repo discovery — scan child directories

**Files:**
- Create: `src/discovery.rs`
- Create: `tests/discovery.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/discovery.rs`:

```rust
use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn init_repo_in(parent: &std::path::Path, name: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    Command::new("git")
        .args(["init", dir.to_str().unwrap()])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", dir.to_str().unwrap(), "commit", "--allow-empty", "-m", "init"])
        .output()
        .unwrap();
    dir
}

#[test]
fn test_discover_repos_finds_child_repos() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "repo-a");
    init_repo_in(parent.path(), "repo-b");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 2);
    let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"repo-a"));
    assert!(names.contains(&"repo-b"));
}

#[test]
fn test_discover_repos_ignores_non_repos() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "real-repo");
    std::fs::create_dir_all(parent.path().join("not-a-repo")).unwrap();
    std::fs::write(parent.path().join("some-file.txt"), "hi").unwrap();

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "real-repo");
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

pub struct DiscoveredRepo {
    pub name: String,
    pub path: PathBuf,
}

pub fn discover_repos(parent: &Path) -> Result<Vec<DiscoveredRepo>> {
    let mut repos = Vec::new();

    let entries = std::fs::read_dir(parent)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip symlinks
        if entry.file_type()?.is_symlink() {
            continue;
        }

        if !path.is_dir() {
            continue;
        }

        // Check if it's a git repo
        if path.join(".git").exists() {
            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            repos.push(DiscoveredRepo { name, path });
        }
    }

    if repos.is_empty() {
        bail!("No git repositories found in {}", parent.display());
    }

    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repos)
}
```

Add to `src/lib.rs`:

```rust
pub mod discovery;
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/discovery.rs src/lib.rs tests/discovery.rs
git commit -m "feat: add repo discovery for multi-repo mode"
```

---

### Task 3: Refactor App to support multiple WorktreeManagers

**Files:**
- Modify: `src/app.rs`
- Create: `tests/app_multi_repo.rs`

- [ ] **Step 1: Write failing tests**

Create `tests/app_multi_repo.rs`:

```rust
use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn init_repo_in(parent: &std::path::Path, name: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    Command::new("git")
        .args(["init", dir.to_str().unwrap()])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", dir.to_str().unwrap(), "commit", "--allow-empty", "-m", "init"])
        .output()
        .unwrap();
    dir
}

#[test]
fn test_multi_repo_app_lists_all_worktrees() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "alpha");
    init_repo_in(parent.path(), "beta");

    let app = arbor::app::App::new(parent.path()).unwrap();
    // Should have 2 main worktrees (one per repo)
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
        .map(|w| w.repo_name.as_deref().unwrap())
        .collect();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));
}

#[test]
fn test_single_repo_worktrees_have_no_repo_name() {
    let dir = TempDir::new().unwrap();
    Command::new("git")
        .args(["init", dir.path().to_str().unwrap()])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "commit", "--allow-empty", "-m", "init"])
        .output()
        .unwrap();

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

    // Create worktree in alpha
    let alpha_mgr = arbor::worktree::WorktreeManager::open(&alpha_path).unwrap();
    alpha_mgr.create("feature-a").unwrap();

    let mut app = arbor::app::App::new(parent.path()).unwrap();

    // Find the alpha/feature-a worktree and cycle its status
    let idx = app.sidebar_state.worktrees.iter()
        .position(|w| w.branch == "feature-a")
        .unwrap();
    app.sidebar_state.selected = idx;
    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();

    // Alpha's .arbor.json should have the change
    let alpha_config = arbor::persistence::ArborConfig::load(&alpha_path);
    assert!(alpha_config.worktrees.contains_key("feature-a"));

    // Beta's .arbor.json should not exist or be empty
    let beta_config = arbor::persistence::ArborConfig::load(&beta_path);
    assert!(beta_config.worktrees.is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test app_multi_repo`
Expected: FAIL — App::new doesn't support non-repo paths

- [ ] **Step 3: Refactor App to hold multiple managers**

In `src/app.rs`, replace:

```rust
worktree_mgr: WorktreeManager,
```

with:

```rust
managers: HashMap<PathBuf, WorktreeManager>,
multi_repo: bool,
```

Keep `repo_root` for single-repo mode. In multi-repo mode, each manager's root is the key.

Refactor `App::new()`:

```rust
pub fn new(path: &std::path::Path) -> Result<Self> {
    let (managers, multi_repo) = if git2::Repository::discover(path).is_ok() {
        // Single-repo mode
        let mgr = WorktreeManager::open(path)?;
        let root = mgr.repo_root().to_path_buf();
        let mut map = HashMap::new();
        map.insert(root, mgr);
        (map, false)
    } else {
        // Multi-repo mode
        let discovered = crate::discovery::discover_repos(path)?;
        let mut map = HashMap::new();
        for repo in discovered {
            match WorktreeManager::open(&repo.path) {
                Ok(mgr) => {
                    map.insert(repo.path, mgr);
                }
                Err(e) => {
                    eprintln!("arbor: warning: skipping {}: {}", repo.name, e);
                }
            }
        }
        if map.is_empty() {
            anyhow::bail!("No valid git repositories found");
        }
        (map, true)
    };

    let worktrees = Self::build_worktree_list(&managers, multi_repo, path)?;
    // ... load configs, apply, etc.
}
```

Add a static method to build the combined flat list. **Important:** config must be applied *before* sorting, because `list()` returns hardcoded defaults (InProgress for main, Queued for others). The sorting groups by `workflow_status`, which is only meaningful after config is applied.

```rust
fn build_worktree_list(
    managers: &HashMap<PathBuf, WorktreeManager>,
    multi_repo: bool,
) -> Result<Vec<WorktreeInfo>> {
    let mut all = Vec::new();
    for (root, mgr) in managers {
        let config = ArborConfig::load(root);
        let mut wts = mgr.list()?;

        for wt in &mut wts {
            // Apply config
            if wt.is_main {
                wt.workflow_status = WorkflowStatus::InProgress;
            } else if let Some(wt_config) = config.worktrees.get(&wt.name) {
                wt.workflow_status = wt_config.status;
                wt.short_name = wt_config.short_name.clone();
            }

            // Tag with repo name in multi-repo mode
            if multi_repo {
                let repo_name = root.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                wt.repo_name = Some(repo_name);
            }
        }
        all.extend(wts);
    }

    // Sort AFTER config application so workflow_status is correct
    all.sort_by(|a, b| {
        let status_ord = |s: &WorkflowStatus| match s {
            WorkflowStatus::InProgress => 0,
            WorkflowStatus::Queued => 1,
            WorkflowStatus::Done => 2,
        };
        status_ord(&a.workflow_status).cmp(&status_ord(&b.workflow_status))
            .then_with(|| b.is_main.cmp(&a.is_main))
            .then_with(|| a.repo_name.cmp(&b.repo_name))
            .then_with(|| {
                let age_a = a.status.as_ref().map(|s| s.last_commit_age_secs).unwrap_or(u64::MAX);
                let age_b = b.status.as_ref().map(|s| s.last_commit_age_secs).unwrap_or(u64::MAX);
                age_a.cmp(&age_b)
            })
    });
    Ok(all)
}
```

Canonicalize paths when building the manager HashMap to avoid duplicate keys from different path representations:

```rust
let canonical = repo.path.canonicalize().unwrap_or(repo.path.clone());
map.insert(canonical, mgr);
```

Update all operations (create, delete, status cycle, persist) to look up the correct manager via `WorktreeInfo.repo_root`.

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/app.rs tests/app_multi_repo.rs
git commit -m "feat: refactor App to support multiple WorktreeManagers"
```

---

### Task 4: Update main.rs for mode detection

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update find_repo_root and main**

Replace `find_repo_root()` — it no longer needs to succeed. Instead, `App::new()` handles both modes. Simplify `main()`:

```rust
fn main() -> Result<()> {
    let cli = Cli::parse();

    let repo_path = match &cli.repo {
        Some(p) => p.clone(),
        None => std::env::current_dir()?,
    };

    let mut app = arbor::app::App::new(&repo_path)?;

    if let Some(ref wt_name) = cli.worktree {
        // Match against repo/branch or just branch
        if let Some(idx) = app.sidebar_state.worktrees.iter()
            .position(|w| {
                // Try repo/branch match first
                if let Some(ref rn) = w.repo_name {
                    if format!("{}/{}", rn, w.branch) == *wt_name {
                        return true;
                    }
                }
                w.branch == *wt_name || w.name == *wt_name
            })
        {
            app.sidebar_state.selected = idx;
        } else {
            eprintln!("arbor: worktree '{}' not found, starting with default", wt_name);
        }
    }

    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    result
}
```

Remove `find_repo_root()` function entirely.

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: simplify main.rs for single and multi-repo mode detection"
```

---

### Task 5: Update UI — repo prefix and create dialog repo selector

**Files:**
- Modify: `src/ui/control_panel.rs`
- Modify: `src/app.rs` (Dialog enum for repo field)

- [ ] **Step 1: Update control panel rendering for repo prefix**

In the worktree name rendering, prepend the repo name when present:

```rust
let display_name = if let Some(ref repo) = wt.repo_name {
    let name = wt.short_name.as_deref().unwrap_or(&wt.branch);
    format!("{}/{}", repo, name)
} else {
    wt.short_name.as_deref().unwrap_or(&wt.branch).to_string()
};
```

- [ ] **Step 2: Update header bar for multi-repo**

In `app.rs` render, add repo name to header when present:

```rust
let mut header_spans = vec![
    Span::styled(format!(" {} ", wt.path.display()), Style::default().fg(Color::DarkGray)),
    Span::styled(format!("⎇ {} ", wt.branch), Style::default().fg(Color::Cyan)),
];
if let Some(ref repo_name) = wt.repo_name {
    header_spans.push(Span::styled(
        format!("📁 {} ", repo_name),
        Style::default().fg(Color::Yellow),
    ));
}
let header = Line::from(header_spans);
```

- [ ] **Step 3: Update create dialog for repo selector**

Add `DialogField::Repo` variant. Update `CreateInput` dialog to include `selected_repo: Option<usize>` and `repo_names: Vec<String>` fields. These are only populated in multi-repo mode.

In the dialog rendering, add the Repo field row when `repo_names` is non-empty. Left/Right arrows cycle through repos when the Repo field is active. Archived branches update when repo changes.

- [ ] **Step 4: Update archive confirm dialog for repo prefix**

When `repo_name` is present, show "Remove alpha/feature-a? (y/n)" instead of just "Remove feature-a? (y/n)".

- [ ] **Step 5: Write tests for repo selector and --worktree matching**

Add to `tests/app_multi_repo.rs`:

```rust
#[test]
fn test_worktree_flag_matches_repo_slash_branch() {
    let parent = TempDir::new().unwrap();
    let alpha_path = init_repo_in(parent.path(), "alpha");
    let alpha_mgr = arbor::worktree::WorktreeManager::open(&alpha_path).unwrap();
    alpha_mgr.create("feature-x").unwrap();

    let app = arbor::app::App::new(parent.path()).unwrap();
    // Find by repo/branch format
    let idx = app.sidebar_state.worktrees.iter()
        .position(|w| {
            w.repo_name.as_deref() == Some("alpha") && w.branch == "feature-x"
        });
    assert!(idx.is_some());
}

#[test]
fn test_worktree_flag_matches_branch_only_with_tiebreak() {
    let parent = TempDir::new().unwrap();
    let alpha_path = init_repo_in(parent.path(), "alpha");
    let beta_path = init_repo_in(parent.path(), "beta");
    let alpha_mgr = arbor::worktree::WorktreeManager::open(&alpha_path).unwrap();
    alpha_mgr.create("shared-branch").unwrap();
    let beta_mgr = arbor::worktree::WorktreeManager::open(&beta_path).unwrap();
    beta_mgr.create("shared-branch").unwrap();

    let app = arbor::app::App::new(parent.path()).unwrap();
    // Both repos have shared-branch; first match should be alphabetical (alpha)
    let idx = app.sidebar_state.worktrees.iter()
        .position(|w| w.branch == "shared-branch");
    assert!(idx.is_some());
    let wt = &app.sidebar_state.worktrees[idx.unwrap()];
    assert_eq!(wt.repo_name.as_deref(), Some("alpha"));
}
```

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/ui/control_panel.rs src/app.rs tests/app_multi_repo.rs
git commit -m "feat: add repo prefix to control panel and create dialog repo selector"
```

---

### Task 6: Final cleanup and integration test

**Files:**
- All

- [ ] **Step 1: Run clippy**

Run: `cargo clippy`
Expected: no warnings

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 3: Manual smoke test — single repo**

Run: `cargo run --manifest-path /Users/richardhope/Repositories/Enablis/arbor/Cargo.toml -- --repo /Users/richardhope/Repositories/Enablis/arbor`

Verify: works identically to before (no repo prefix, no repo selector in create dialog)

- [ ] **Step 4: Manual smoke test — multi repo**

Run: `cargo run --manifest-path /Users/richardhope/Repositories/Enablis/arbor/Cargo.toml -- --repo /Users/richardhope/Repositories/Enablis`

Verify:
- Discovers multiple repos (arbor, fusion-platform, fusion-dashboard, etc.)
- Worktrees show repo prefix
- Status groups work across repos
- Create dialog shows repo selector
- Persistence is per-repo

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: clippy fixes and multi-repo cleanup"
```
