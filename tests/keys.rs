use arbor::keys::{handle_key, Action, Focus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn test_s_key_is_noop() {
    let action = handle_key(make_key(KeyCode::Char('s')), &Focus::Sidebar);
    assert!(matches!(action, Action::None));
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

#[test]
fn test_slash_triggers_filter() {
    let action = handle_key(make_key(KeyCode::Char('/')), &Focus::Sidebar);
    assert!(matches!(action, Action::Filter));
}

#[test]
fn test_ctrl_g_triggers_open_pr() {
    let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    let action = handle_key(key, &Focus::Sidebar);
    assert!(matches!(action, Action::OpenPR));
}

#[test]
fn test_ctrl_g_noop_in_terminal() {
    let key = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    let action = handle_key(key, &Focus::Terminal);
    assert!(matches!(action, Action::TerminalInput(_)));
}
