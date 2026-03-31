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
fn test_status_cycle_noop_on_main() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.focus = Focus::Sidebar;
    app.sidebar_state.selected = 0;
    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();
    assert_eq!(app.sidebar_state.worktrees[0].workflow_status, WorkflowStatus::InProgress);
}

#[test]
fn test_status_cycle_on_non_main() {
    let dir = common::init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("feature-a").unwrap();

    let mut app = arbor::app::App::new(dir.path()).unwrap();
    let non_main_idx = app.sidebar_state.worktrees.iter()
        .position(|w| !w.is_main)
        .unwrap();
    app.sidebar_state.selected = non_main_idx;
    app.focus = Focus::Sidebar;

    assert_eq!(app.sidebar_state.worktrees[non_main_idx].workflow_status, WorkflowStatus::Queued);

    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();
    assert_eq!(app.sidebar_state.worktrees[non_main_idx].workflow_status, WorkflowStatus::InProgress);

    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();
    assert_eq!(app.sidebar_state.worktrees[non_main_idx].workflow_status, WorkflowStatus::Done);

    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();
    assert_eq!(app.sidebar_state.worktrees[non_main_idx].workflow_status, WorkflowStatus::Queued);
}

#[test]
fn test_status_cycle_persists_to_file() {
    let dir = common::init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("feature-b").unwrap();

    let mut app = arbor::app::App::new(dir.path()).unwrap();
    let non_main_idx = app.sidebar_state.worktrees.iter()
        .position(|w| !w.is_main)
        .unwrap();
    app.sidebar_state.selected = non_main_idx;
    app.focus = Focus::Sidebar;

    app.handle_action(arbor::keys::Action::StatusCycle).unwrap();

    let config = arbor::persistence::ArborConfig::load(dir.path());
    assert_eq!(config.worktrees["feature-b"].status, WorkflowStatus::InProgress);
}

#[test]
fn test_config_override_main_always_in_progress() {
    let dir = common::init_test_repo();
    // Write a config that tries to set main to Done
    let mut config = arbor::persistence::ArborConfig::default();
    config.worktrees.insert("main".to_string(), arbor::persistence::WorktreeConfig {
        status: WorkflowStatus::Done,
        short_name: None,
    });
    config.save(dir.path()).unwrap();

    let app = arbor::app::App::new(dir.path()).unwrap();
    assert_eq!(app.sidebar_state.worktrees[0].workflow_status, WorkflowStatus::InProgress);
}
