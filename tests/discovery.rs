use std::process::Command;
use std::path::PathBuf;
use tempfile::TempDir;

fn init_repo_in(parent: &std::path::Path, name: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    Command::new("git").args(["init", dir.to_str().unwrap()]).output().unwrap();
    Command::new("git").args(["-C", dir.to_str().unwrap(), "config", "user.email", "t@t"]).output().unwrap();
    Command::new("git").args(["-C", dir.to_str().unwrap(), "config", "user.name", "T"]).output().unwrap();
    Command::new("git").args(["-C", dir.to_str().unwrap(), "commit", "--allow-empty", "-m", "init"]).output().unwrap();
    dir
}

#[test]
fn test_discover_repos_finds_nested_repos() {
    let parent = TempDir::new().unwrap();
    let enablis = parent.path().join("Enablis");
    std::fs::create_dir_all(&enablis).unwrap();
    init_repo_in(&enablis, "arbor");
    init_repo_in(&enablis, "fusion");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 2);
    let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Enablis/arbor"));
    assert!(names.contains(&"Enablis/fusion"));
}

#[test]
fn test_discover_repos_skips_hidden_dirs() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "visible-repo");
    let hidden = parent.path().join(".hidden");
    std::fs::create_dir_all(&hidden).unwrap();
    init_repo_in(&hidden, "secret-repo");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "visible-repo");
}

#[test]
fn test_discover_repos_skips_node_modules() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "real-repo");
    let nm = parent.path().join("node_modules");
    std::fs::create_dir_all(&nm).unwrap();
    init_repo_in(&nm, "some-dep");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
}

#[test]
fn test_discover_repos_skips_worktree_dirs() {
    let parent = TempDir::new().unwrap();
    init_repo_in(parent.path(), "my-repo");
    let wt = parent.path().join("my-repo-worktrees");
    std::fs::create_dir_all(&wt).unwrap();
    init_repo_in(&wt, "feature-a");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "my-repo");
}

#[test]
fn test_discover_repos_stops_at_git() {
    let parent = TempDir::new().unwrap();
    let repo = init_repo_in(parent.path(), "outer-repo");
    init_repo_in(&repo, "inner-repo");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "outer-repo");
}

#[test]
fn test_discover_repos_respects_max_depth() {
    let parent = TempDir::new().unwrap();
    let deep = parent.path().join("a").join("b").join("c").join("d");
    std::fs::create_dir_all(&deep).unwrap();
    init_repo_in(&deep, "too-deep");
    let shallow = parent.path().join("a");
    init_repo_in(&shallow, "ok-repo");

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "a/ok-repo");
}

#[test]
fn test_discover_repos_empty_parent_errors() {
    let parent = TempDir::new().unwrap();
    let result = arbor::discovery::discover_repos(parent.path());
    assert!(result.is_err());
}

#[cfg(unix)]
#[test]
fn test_discover_repos_does_not_follow_symlinks() {
    let parent = TempDir::new().unwrap();
    let real = init_repo_in(parent.path(), "real-repo");
    std::os::unix::fs::symlink(&real, parent.path().join("linked-repo")).unwrap();

    let repos = arbor::discovery::discover_repos(parent.path()).unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].name, "real-repo");
}
