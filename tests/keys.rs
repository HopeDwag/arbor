use arbor::keys::{handle_key, Action, Focus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn test_s_key_triggers_status_cycle() {
    let action = handle_key(make_key(KeyCode::Char('s')), &Focus::Sidebar);
    assert!(matches!(action, Action::StatusCycle));
}

#[test]
fn test_less_than_no_longer_resizes() {
    let action = handle_key(make_key(KeyCode::Char('<')), &Focus::Sidebar);
    assert!(matches!(action, Action::None));
}

#[test]
fn test_greater_than_no_longer_resizes() {
    let action = handle_key(make_key(KeyCode::Char('>')), &Focus::Sidebar);
    assert!(matches!(action, Action::None));
}
