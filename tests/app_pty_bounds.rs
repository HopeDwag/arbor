mod common;

#[test]
fn test_ensure_pty_empty_worktrees_returns_ok() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();

    // Clear worktrees to simulate empty state
    app.sidebar_state.worktrees.clear();

    // Should return Ok(()) without panicking
    let result = app.ensure_pty_for_selected(24, 80);
    assert!(result.is_ok());
}

#[test]
fn test_ensure_pty_selected_out_of_bounds_returns_ok() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();

    // Set selected beyond list length
    app.sidebar_state.selected = app.sidebar_state.worktrees.len() + 10;

    // Should return Ok(()) without panicking
    let result = app.ensure_pty_for_selected(24, 80);
    assert!(result.is_ok());
}
