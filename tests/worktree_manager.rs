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
fn test_list_worktrees_returns_main() {
    let dir = init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    assert!(!worktrees.is_empty());
    assert!(worktrees[0].is_main);
}

#[test]
fn test_create_worktree() {
    let dir = init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    manager.create("test-branch").unwrap();
    let worktrees = manager.list().unwrap();
    assert_eq!(worktrees.len(), 2);
}

#[test]
fn test_delete_worktree() {
    let dir = init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    manager.create("to-delete").unwrap();
    manager.delete("to-delete", false).unwrap();
    let worktrees = manager.list().unwrap();
    assert_eq!(worktrees.len(), 1);
}

#[test]
fn test_cannot_delete_main() {
    let dir = init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let result = manager.delete("main", false);
    assert!(result.is_err());
}

#[test]
fn test_full_crud_cycle() {
    let dir = init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();

    // List — starts with just main
    let wts = manager.list().unwrap();
    assert_eq!(wts.len(), 1);
    assert!(wts[0].is_main);

    // Create
    let path = manager.create("feature-a").unwrap();
    assert!(path.exists());

    let wts = manager.list().unwrap();
    assert_eq!(wts.len(), 2);

    // Delete
    manager.delete("feature-a", false).unwrap();
    let wts = manager.list().unwrap();
    assert_eq!(wts.len(), 1);
}
