use arbor::github::{GitHubCache, PrState};

#[test]
fn test_parse_gh_json() {
    let json = r#"[
        {"number": 42, "headRefName": "feature-auth", "state": "OPEN", "isDraft": false, "url": "https://github.com/org/repo/pull/42"},
        {"number": 43, "headRefName": "fix-login", "state": "OPEN", "isDraft": true, "url": "https://github.com/org/repo/pull/43"},
        {"number": 40, "headRefName": "old-feature", "state": "MERGED", "isDraft": false, "url": "https://github.com/org/repo/pull/40"},
        {"number": 39, "headRefName": "rejected", "state": "CLOSED", "isDraft": false, "url": "https://github.com/org/repo/pull/39"}
    ]"#;

    let cache = GitHubCache::from_json(json);

    let pr = cache.get("feature-auth").unwrap();
    assert_eq!(pr.number, 42);
    assert_eq!(pr.state, PrState::Open);
    assert_eq!(pr.url, "https://github.com/org/repo/pull/42");

    let pr = cache.get("fix-login").unwrap();
    assert_eq!(pr.state, PrState::Draft);

    let pr = cache.get("old-feature").unwrap();
    assert_eq!(pr.state, PrState::Merged);

    let pr = cache.get("rejected").unwrap();
    assert_eq!(pr.state, PrState::Closed);

    assert!(cache.get("no-such-branch").is_none());
}

#[test]
fn test_parse_empty_json() {
    let cache = GitHubCache::from_json("[]");
    assert!(cache.get("anything").is_none());
}

#[test]
fn test_parse_invalid_json() {
    let cache = GitHubCache::from_json("not json at all");
    assert!(cache.get("anything").is_none());
}

#[test]
fn test_empty_cache() {
    let cache = GitHubCache::empty();
    assert!(cache.get("anything").is_none());
}
