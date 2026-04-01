mod common;

use arbor::keys::Focus;
use arbor::persistence::WorkflowStatus;

#[test]
fn test_main_worktree_defaults_to_in_progress() {
    let dir = common::init_test_repo();
    let app = arbor::app::App::new(dir.path()).unwrap();
    let wt = &app.sidebar_state.worktrees[0];
    assert!(wt.is_main);
    assert_eq!(wt.workflow_status, WorkflowStatus::InProgress);
}

#[test]
fn test_non_main_worktree_defaults_to_backlog() {
    let dir = common::init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("feature-a").unwrap();

    let app = arbor::app::App::new(dir.path()).unwrap();
    let non_main_idx = app.sidebar_state.worktrees.iter()
        .position(|w| !w.is_main)
        .unwrap();

    assert_eq!(app.sidebar_state.worktrees[non_main_idx].workflow_status, WorkflowStatus::Backlog);
}

#[test]
fn test_config_override_main_always_in_progress() {
    let dir = common::init_test_repo();
    // Write a config that tries to set main to Backlog
    let mut config = arbor::persistence::ArborConfig::default();
    config.worktrees.insert("main".to_string(), arbor::persistence::WorktreeConfig {
        status: WorkflowStatus::Backlog,
        short_name: None,
    });
    config.save(dir.path()).unwrap();

    let app = arbor::app::App::new(dir.path()).unwrap();
    assert_eq!(app.sidebar_state.worktrees[0].workflow_status, WorkflowStatus::InProgress);
}
