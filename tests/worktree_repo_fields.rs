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
