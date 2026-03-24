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
fn test_autosize_minimum_width() {
    let dir = init_test_repo();
    let app = arbor::app::App::new(dir.path()).unwrap();
    assert!(app.panel_width() >= 20);
}

#[test]
fn test_autosize_grows_with_long_names() {
    let dir = init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("very-long-feature-branch-name-here").unwrap();
    let app = arbor::app::App::new(dir.path()).unwrap();
    assert!(app.panel_width() > 20);
}

#[test]
fn test_autosize_respects_maximum() {
    let dir = init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("a-really-extremely-long-branch-name-that-goes-way-beyond-sixty-chars").unwrap();
    let app = arbor::app::App::new(dir.path()).unwrap();
    assert!(app.panel_width() <= 60);
}
