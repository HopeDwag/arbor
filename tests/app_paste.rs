mod common;

use arbor::app::{App, Dialog, DialogField};

/// Helper: set up an App with a CreateInput dialog in the given field.
fn app_with_create_dialog(dir: &std::path::Path, field: DialogField) -> App {
    let mut app = App::new(dir).unwrap();
    app.dialog = Dialog::CreateInput {
        input: String::new(),
        active_field: field,
        archived: vec![],
        selected_archived: None,
        repo_root: dir.to_path_buf(),
        repo_names: vec![],
        selected_repo: 0,
    };
    app
}

#[test]
fn test_paste_into_branch_field_appends() {
    let dir = common::init_test_repo();
    let mut app = app_with_create_dialog(dir.path(), DialogField::Branch);

    let handled = app.handle_dialog_paste("feature-foo");
    assert!(handled);

    match &app.dialog {
        Dialog::CreateInput { input, .. } => {
            assert_eq!(input, "feature-foo");
        }
        _ => panic!("Expected CreateInput dialog"),
    }

    // Paste again — should append
    app.handle_dialog_paste("-bar");
    match &app.dialog {
        Dialog::CreateInput { input, .. } => {
            assert_eq!(input, "feature-foo-bar");
        }
        _ => panic!("Expected CreateInput dialog"),
    }
}

#[test]
fn test_paste_strips_newlines() {
    let dir = common::init_test_repo();
    let mut app = app_with_create_dialog(dir.path(), DialogField::Branch);

    app.handle_dialog_paste("line1\nline2\r\nline3");
    match &app.dialog {
        Dialog::CreateInput { input, .. } => {
            assert_eq!(input, "line1line2line3");
        }
        _ => panic!("Expected CreateInput dialog"),
    }
}

#[test]
fn test_paste_into_repo_field_is_noop() {
    let dir = common::init_test_repo();
    let mut app = app_with_create_dialog(dir.path(), DialogField::Repo);

    let handled = app.handle_dialog_paste("should-not-appear");
    // Paste into Repo is handled (returns true — dialog is open) but field is read-only
    assert!(handled);

    match &app.dialog {
        Dialog::CreateInput { input, .. } => {
            assert_eq!(input, "");
        }
        _ => panic!("Expected CreateInput dialog"),
    }
}

#[test]
fn test_paste_with_no_dialog_returns_false() {
    let dir = common::init_test_repo();
    let mut app = App::new(dir.path()).unwrap();

    // No dialog open (Dialog::None by default)
    let handled = app.handle_dialog_paste("hello");
    assert!(!handled);
}
