mod common;

#[test]
fn test_list_worktrees_returns_main() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    assert!(!worktrees.is_empty());
    assert!(worktrees[0].is_main);
}

#[test]
fn test_create_worktree() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    manager.create("test-branch").unwrap();
    let worktrees = manager.list().unwrap();
    assert_eq!(worktrees.len(), 2);
}

#[test]
fn test_delete_worktree() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    manager.create("to-delete").unwrap();
    manager.delete("to-delete", false).unwrap();
    let worktrees = manager.list().unwrap();
    assert_eq!(worktrees.len(), 1);
}

#[test]
fn test_cannot_delete_main() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let result = manager.delete("main", false);
    assert!(result.is_err());
}

#[test]
fn test_worktree_info_has_workflow_status_and_short_name() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    assert_eq!(worktrees[0].workflow_status, arbor::persistence::WorkflowStatus::Root);
    assert_eq!(worktrees[0].short_name, None);
}

#[test]
fn test_repo_root_accessor() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    assert!(manager.repo_root().exists());
}

#[test]
fn test_worktree_has_ahead_behind_fields() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    // No remote, so both should be 0
    assert_eq!(worktrees[0].ahead, 0);
    assert_eq!(worktrees[0].behind, 0);
}

#[test]
fn test_full_crud_cycle() {
    let dir = common::init_test_repo();
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

#[test]
fn test_worktrees_sorted_by_most_recent_commit_first() {
    use std::process::Command;

    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();

    // Create three worktrees — they all start at the same commit as main
    manager.create("oldest").unwrap();
    manager.create("middle").unwrap();
    manager.create("newest").unwrap();

    let worktrees_dir = dir.path().parent().unwrap().join(
        format!("{}-worktrees", dir.path().file_name().unwrap().to_str().unwrap()),
    );

    // Make commits with explicit timestamps so ordering is deterministic.
    // oldest gets a commit dated 1 hour ago, middle 30 min ago, newest now.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let oldest_time = now - 3600; // 1 hour ago
    let middle_time = now - 1800; // 30 min ago
    let newest_time = now;        // now

    for (name, timestamp) in [("oldest", oldest_time), ("middle", middle_time), ("newest", newest_time)] {
        let wt_path = worktrees_dir.join(name);
        let date_str = format!("{} +0000", timestamp);
        Command::new("git")
            .args(["-C", wt_path.to_str().unwrap(), "commit", "--allow-empty", "-m", &format!("commit in {}", name)])
            .env("GIT_AUTHOR_DATE", &date_str)
            .env("GIT_COMMITTER_DATE", &date_str)
            .output()
            .unwrap();
    }

    // Re-open manager to get fresh list
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let wts = manager.list().unwrap();

    // Should be: main first, then non-main sorted by most recent (smallest age) first
    assert_eq!(wts.len(), 4);
    assert!(wts[0].is_main);

    // Collect non-main worktree names in list order
    let non_main: Vec<&str> = wts.iter()
        .filter(|w| !w.is_main)
        .map(|w| w.name.as_str())
        .collect();
    assert_eq!(non_main, vec!["newest", "middle", "oldest"]);

    // Also verify ages are monotonically increasing (most recent first)
    let non_main_ages: Vec<u64> = wts.iter()
        .filter(|w| !w.is_main)
        .map(|w| w.last_commit_age_secs)
        .collect();
    for window in non_main_ages.windows(2) {
        assert!(window[0] <= window[1],
            "Expected ages in ascending order (most recent first), got {:?}", non_main_ages);
    }
}

#[test]
fn test_worktree_info_has_commit_message() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    assert_eq!(worktrees[0].commit_message.as_deref(), Some("init"));
}

#[test]
fn test_worktree_info_has_is_dirty() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    assert!(!worktrees[0].is_dirty);
}

#[test]
fn test_worktree_dirty_when_file_added() {
    let dir = common::init_test_repo();
    std::fs::write(dir.path().join("dirty.txt"), "hello").unwrap();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    assert!(worktrees[0].is_dirty);
}
