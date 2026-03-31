mod common;

#[test]
fn test_autosize_minimum_width() {
    let dir = common::init_test_repo();
    let app = arbor::app::App::new(dir.path()).unwrap();
    assert!(app.panel_width() >= 20);
}

#[test]
fn test_autosize_grows_with_long_names() {
    let dir = common::init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("very-long-feature-branch-name-here").unwrap();
    let app = arbor::app::App::new(dir.path()).unwrap();
    assert!(app.panel_width() > 20);
}

#[test]
fn test_autosize_respects_maximum() {
    let dir = common::init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("a-really-extremely-long-branch-name-that-goes-way-beyond-sixty-chars").unwrap();
    let app = arbor::app::App::new(dir.path()).unwrap();
    assert!(app.panel_width() <= 60);
}
