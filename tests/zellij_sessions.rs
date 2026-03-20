#[test]
fn test_sanitize_branch_name() {
    assert_eq!(
        arbor::zellij::sanitize_session_name("feat/auth-redesign"),
        "arbor-feat-auth-redesign"
    );
    assert_eq!(
        arbor::zellij::sanitize_session_name("fix/memory-leak"),
        "arbor-fix-memory-leak"
    );
    assert_eq!(
        arbor::zellij::sanitize_session_name("main"),
        "arbor-main"
    );
}

#[test]
fn test_generate_layout_kdl() {
    let kdl = arbor::zellij::generate_layout_kdl("/tmp/test-worktree");
    assert!(kdl.contains("cwd \"/tmp/test-worktree\""));
    assert!(kdl.contains("pane"));
}
