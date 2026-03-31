mod common;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use arbor::keys::{Action, Focus};

fn make_scroll_up(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }
}

fn make_scroll_down(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }
}

#[test]
fn test_scroll_up_increases_offset_when_terminal_focused() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    assert_eq!(app.focus, Focus::Terminal);
    assert_eq!(app.scroll_offset, 0);

    app.handle_mouse(make_scroll_up(35, 5)).unwrap();
    assert_eq!(app.scroll_offset, 3);

    app.handle_mouse(make_scroll_up(35, 5)).unwrap();
    assert_eq!(app.scroll_offset, 6);
}

#[test]
fn test_scroll_down_decreases_offset_saturating_at_zero() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    assert_eq!(app.focus, Focus::Terminal);

    // Scroll up first to get a non-zero offset
    app.handle_mouse(make_scroll_up(35, 5)).unwrap();
    assert_eq!(app.scroll_offset, 3);

    // Scroll down brings it back
    app.handle_mouse(make_scroll_down(35, 5)).unwrap();
    assert_eq!(app.scroll_offset, 0);

    // Scroll down again saturates at 0
    app.handle_mouse(make_scroll_down(35, 5)).unwrap();
    assert_eq!(app.scroll_offset, 0);
}

#[test]
fn test_scroll_ignored_when_sidebar_focused() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.focus = Focus::Sidebar;
    assert_eq!(app.scroll_offset, 0);

    app.handle_mouse(make_scroll_up(5, 5)).unwrap();
    assert_eq!(app.scroll_offset, 0);

    app.handle_mouse(make_scroll_down(5, 5)).unwrap();
    assert_eq!(app.scroll_offset, 0);
}

#[test]
fn test_terminal_input_resets_scroll_offset() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    assert_eq!(app.focus, Focus::Terminal);

    // Scroll up to get a non-zero offset
    app.handle_mouse(make_scroll_up(35, 5)).unwrap();
    assert_eq!(app.scroll_offset, 3);

    // Terminal input resets offset to 0
    let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
    app.handle_action(Action::TerminalInput(key)).unwrap();
    assert_eq!(app.scroll_offset, 0);
}
