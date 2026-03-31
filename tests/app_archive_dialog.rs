mod common;

use arbor::app::Dialog;
use arbor::keys::Focus;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn test_archive_confirm_y_removes_worktree() {
    let dir = common::init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("to-archive").unwrap();

    let mut app = arbor::app::App::new(dir.path()).unwrap();
    // Select the non-main worktree
    let idx = app.sidebar_state.worktrees.iter()
        .position(|w| w.name == "to-archive")
        .unwrap();
    app.sidebar_state.selected = idx;
    app.focus = Focus::Sidebar;

    // Trigger the archive action to open the confirm dialog
    app.handle_action(arbor::keys::Action::SidebarArchive).unwrap();
    assert!(matches!(app.dialog, Dialog::ArchiveConfirm(..)));

    // Confirm with 'y'
    app.handle_dialog_key(make_key(KeyCode::Char('y'))).unwrap();

    // Dialog should be dismissed
    assert!(matches!(app.dialog, Dialog::None));
    // Worktree list should only have main
    assert_eq!(app.sidebar_state.worktrees.len(), 1);
    assert!(app.sidebar_state.worktrees[0].is_main);
}

#[test]
fn test_archive_confirm_n_cancels() {
    let dir = common::init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("keep-me").unwrap();

    let mut app = arbor::app::App::new(dir.path()).unwrap();
    let idx = app.sidebar_state.worktrees.iter()
        .position(|w| w.name == "keep-me")
        .unwrap();
    app.sidebar_state.selected = idx;
    app.focus = Focus::Sidebar;

    app.handle_action(arbor::keys::Action::SidebarArchive).unwrap();
    assert!(matches!(app.dialog, Dialog::ArchiveConfirm(..)));

    // Cancel with 'n'
    app.handle_dialog_key(make_key(KeyCode::Char('n'))).unwrap();

    // Dialog dismissed, worktree still present
    assert!(matches!(app.dialog, Dialog::None));
    assert_eq!(app.sidebar_state.worktrees.len(), 2);
    assert!(app.sidebar_state.worktrees.iter().any(|w| w.name == "keep-me"));
}

#[test]
fn test_archive_confirm_esc_cancels() {
    let dir = common::init_test_repo();
    let mgr = arbor::worktree::WorktreeManager::open(dir.path()).unwrap();
    mgr.create("also-keep").unwrap();

    let mut app = arbor::app::App::new(dir.path()).unwrap();
    let idx = app.sidebar_state.worktrees.iter()
        .position(|w| w.name == "also-keep")
        .unwrap();
    app.sidebar_state.selected = idx;
    app.focus = Focus::Sidebar;

    app.handle_action(arbor::keys::Action::SidebarArchive).unwrap();
    assert!(matches!(app.dialog, Dialog::ArchiveConfirm(..)));

    // Cancel with Esc
    app.handle_dialog_key(make_key(KeyCode::Esc)).unwrap();

    assert!(matches!(app.dialog, Dialog::None));
    assert_eq!(app.sidebar_state.worktrees.len(), 2);
    assert!(app.sidebar_state.worktrees.iter().any(|w| w.name == "also-keep"));
}

#[test]
fn test_main_worktree_cannot_be_archived() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    // Select main worktree (index 0)
    app.sidebar_state.selected = 0;
    app.focus = Focus::Sidebar;
    assert!(app.sidebar_state.worktrees[0].is_main);

    // Try to archive — the guard in SidebarArchive should prevent the dialog from opening
    app.handle_action(arbor::keys::Action::SidebarArchive).unwrap();
    assert!(matches!(app.dialog, Dialog::None));
}
