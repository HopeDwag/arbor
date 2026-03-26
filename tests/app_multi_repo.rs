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
