mod common;

use arbor::keys::Focus;

/// Mouse capture must always be enabled so clicks on the sidebar work
/// from any focus state. The previous approach of disabling mouse capture
/// in terminal focus prevented the terminal emulator from delivering
/// sidebar click events to arbor at all.
///
/// Instead, mouse capture stays on always. Native text selection in the
/// terminal pane is handled by passing mouse events through to the PTY
/// when terminal is focused (like tmux does).
#[test]
fn test_mouse_capture_always_enabled() {
    let dir = common::init_test_repo();
    let app = arbor::app::App::new(dir.path()).unwrap();
    // Default focus is Terminal
    assert_eq!(app.focus, Focus::Terminal);
    // Mouse capture must still be on so sidebar clicks are received
    assert!(app.mouse_capture_enabled());
}

#[test]
fn test_mouse_capture_enabled_after_focus_sidebar() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.handle_action(arbor::keys::Action::FocusSidebar).unwrap();
    assert_eq!(app.focus, Focus::Sidebar);
    assert!(app.mouse_capture_enabled());
}

#[test]
fn test_mouse_capture_enabled_after_focus_terminal() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.handle_action(arbor::keys::Action::FocusSidebar).unwrap();
    app.handle_action(arbor::keys::Action::FocusTerminal).unwrap();
    assert_eq!(app.focus, Focus::Terminal);
    assert!(app.mouse_capture_enabled());
}
