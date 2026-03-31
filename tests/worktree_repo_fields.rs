mod common;

#[test]
fn test_worktree_info_has_repo_fields() {
    let dir = common::init_test_repo();
    let manager = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    let worktrees = manager.list().unwrap();
    assert_eq!(worktrees[0].repo_name, None);
    assert_eq!(worktrees[0].repo_root, dir.path().canonicalize().unwrap());
}
