use tempfile::TempDir;

#[test]
fn test_load_missing_file_returns_defaults() {
    let dir = TempDir::new().unwrap();
    let config = arbor::persistence::ArborConfig::load(dir.path());
    assert!(config.worktrees.is_empty());
}

#[test]
fn test_load_malformed_json_returns_defaults() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".arbor.json");
    std::fs::write(&path, "not valid json {{{").unwrap();
    let config = arbor::persistence::ArborConfig::load(dir.path());
    assert!(config.worktrees.is_empty());
}

#[test]
fn test_save_and_load_roundtrip() {
    let dir = TempDir::new().unwrap();
    let mut config = arbor::persistence::ArborConfig::default();
    config.worktrees.insert(
        "feature-auth".to_string(),
        arbor::persistence::WorktreeConfig {
            status: arbor::persistence::WorkflowStatus::InProgress,
            short_name: Some("auth".to_string()),
        },
    );
    config.save(dir.path()).unwrap();

    let loaded = arbor::persistence::ArborConfig::load(dir.path());
    assert_eq!(loaded.worktrees.len(), 1);
    let wt = &loaded.worktrees["feature-auth"];
    assert_eq!(wt.status, arbor::persistence::WorkflowStatus::InProgress);
    assert_eq!(wt.short_name, Some("auth".to_string()));
}

#[test]
fn test_default_status_is_queued() {
    let config = arbor::persistence::WorktreeConfig::default();
    assert_eq!(config.status, arbor::persistence::WorkflowStatus::Queued);
}

#[test]
fn test_workflow_status_cycle() {
    use arbor::persistence::WorkflowStatus;
    assert_eq!(WorkflowStatus::Queued.next(), WorkflowStatus::InProgress);
    assert_eq!(WorkflowStatus::InProgress.next(), WorkflowStatus::Done);
    assert_eq!(WorkflowStatus::Done.next(), WorkflowStatus::Queued);
}
