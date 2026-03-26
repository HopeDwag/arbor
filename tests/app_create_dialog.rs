use std::process::Command;
use tempfile::TempDir;
use arbor::app::Dialog;

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

#[test]
fn test_create_dialog_has_short_name_field() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();
    app.handle_action(arbor::keys::Action::SidebarCreate).unwrap();
    match &app.dialog {
        Dialog::CreateInput { short_name, .. } => {
            assert_eq!(*short_name, String::new());
        }
        _ => panic!("Expected CreateInput dialog"),
    }
}

#[test]
fn test_create_with_short_name_persists() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();

    let repo_root = app.sidebar_state.worktrees[0].repo_root.clone();
    app.dialog = Dialog::CreateInput {
        input: "feature-x".to_string(),
        short_name: "fx".to_string(),
        active_field: arbor::app::DialogField::Branch,
        archived: vec![],
        selected_archived: None,
        repo_root: repo_root.clone(),
    };

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    app.handle_dialog_key(enter).unwrap();

    let config = arbor::persistence::ArborConfig::load(&repo_root);
    assert_eq!(config.worktrees["feature-x"].short_name, Some("fx".to_string()));
}

#[test]
fn test_short_name_max_length() {
    let dir = init_test_repo();
    let mut app = arbor::app::App::new(dir.path()).unwrap();

    app.dialog = Dialog::CreateInput {
        input: String::new(),
        short_name: String::new(),
        active_field: arbor::app::DialogField::Name,
        archived: vec![],
        selected_archived: None,
        repo_root: dir.path().to_path_buf(),
    };

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    // Type 25 characters
    for _ in 0..25 {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        app.handle_dialog_key(key).unwrap();
    }

    match &app.dialog {
        Dialog::CreateInput { short_name, .. } => {
            assert_eq!(short_name.len(), 20); // max 20
        }
        _ => panic!("Expected CreateInput dialog"),
    }
}
