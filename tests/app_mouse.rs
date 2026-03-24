use std::process::Command;
use tempfile::TempDir;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use arbor::keys::Focus;

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

fn make_mouse_down(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: crossterm::event::KeyModifiers::empty(),
    }
}

#[test]
fn test_click_sidebar_focuses_sidebar() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    // App starts with focus on Terminal
    assert_eq!(app.focus, Focus::Terminal);

    // Click inside the sidebar area (column 5, well within default sidebar_width of 30)
    app.handle_mouse(make_mouse_down(5, 5)).unwrap();
    assert_eq!(app.focus, Focus::Sidebar);
}

#[test]
fn test_click_terminal_focuses_terminal() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    // Start in sidebar focus
    app.focus = Focus::Sidebar;

    // Click in the terminal area (column 35, past default sidebar_width of 30)
    app.handle_mouse(make_mouse_down(35, 5)).unwrap();
    assert_eq!(app.focus, Focus::Terminal);
}

#[test]
fn test_click_sidebar_then_terminal() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    assert_eq!(app.focus, Focus::Terminal);

    // Click sidebar
    app.handle_mouse(make_mouse_down(5, 5)).unwrap();
    assert_eq!(app.focus, Focus::Sidebar);

    // Click terminal
    app.handle_mouse(make_mouse_down(35, 5)).unwrap();
    assert_eq!(app.focus, Focus::Terminal);
}

#[test]
fn test_click_near_border_does_not_change_focus() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    assert_eq!(app.focus, Focus::Terminal);

    // Click on the border (column 29 = sidebar_width 30 - 1)
    app.handle_mouse(make_mouse_down(29, 5)).unwrap();
    // Should start drag, not change focus
    assert_eq!(app.focus, Focus::Terminal);
}
