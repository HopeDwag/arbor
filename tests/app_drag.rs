use std::process::Command;
use tempfile::TempDir;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind, KeyModifiers};
use arbor::keys::Focus;
use arbor::persistence::WorkflowStatus;

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

fn mouse_event(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
    MouseEvent { kind, column: col, row, modifiers: KeyModifiers::empty() }
}

#[test]
fn test_click_without_drag_focuses_sidebar() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.focus = Focus::Terminal;

    app.handle_mouse(mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 3)).unwrap();
    app.handle_mouse(mouse_event(MouseEventKind::Up(MouseButton::Left), 5, 3)).unwrap();

    assert_eq!(app.focus, Focus::Sidebar);
}

#[test]
fn test_main_worktree_cannot_be_dragged() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.focus = Focus::Sidebar;
    app.sidebar_state.selected = 0;

    // Simulate drag attempt on main
    app.handle_mouse(mouse_event(MouseEventKind::Down(MouseButton::Left), 5, 2)).unwrap();
    app.handle_mouse(mouse_event(MouseEventKind::Drag(MouseButton::Left), 5, 8)).unwrap();
    app.handle_mouse(mouse_event(MouseEventKind::Up(MouseButton::Left), 5, 8)).unwrap();

    assert_eq!(app.sidebar_state.worktrees[0].workflow_status, WorkflowStatus::InProgress);
}
