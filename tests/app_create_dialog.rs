mod common;

use arbor::app::{derive_short_name, Dialog};

#[test]
fn test_create_dialog_starts_with_empty_branch() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.handle_action(arbor::keys::Action::SidebarCreate).unwrap();
    match &app.dialog {
        Dialog::CreateInput { input, .. } => {
            assert_eq!(*input, String::new());
        }
        _ => panic!("Expected CreateInput dialog"),
    }
}

#[test]
fn test_create_without_prefix_no_short_name() {
    let dir = common::init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();

    let repo_root = app.sidebar_state.worktrees[0].repo_root.clone();
    app.dialog = Dialog::CreateInput {
        input: "feature-x".to_string(),
        active_field: arbor::app::DialogField::Branch,
        archived: vec![],
        selected_archived: None,
        repo_root: repo_root.clone(),
        repo_names: vec![],
        selected_repo: 0,
    };

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    app.handle_dialog_key(enter).unwrap();

    let config = arbor::persistence::ArborConfig::load(&repo_root);
    // No slash in branch name, so no short_name derived
    assert_eq!(config.worktrees["feature-x"].short_name, None);
}

#[test]
fn test_derive_short_name_with_prefix() {
    assert_eq!(derive_short_name("feature/auth-flow"), Some("auth-flow".to_string()));
}

#[test]
fn test_derive_short_name_with_deep_prefix() {
    assert_eq!(derive_short_name("bugfix/JIRA-1234-fix-login"), Some("JIRA-1234-fix-login".to_string()));
}

#[test]
fn test_derive_short_name_no_prefix() {
    assert_eq!(derive_short_name("main"), None);
    assert_eq!(derive_short_name("feature-x"), None);
}

#[test]
fn test_derive_short_name_truncates_to_20() {
    let result = derive_short_name("feature/this-is-a-very-long-branch-suffix-name");
    let name = result.unwrap();
    assert_eq!(name.len(), 20);
    assert_eq!(name, "this-is-a-very-long-");
}
