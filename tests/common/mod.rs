use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary git repo with a single empty commit.
/// Includes user.email and user.name config for CI compatibility.
#[allow(dead_code)]
pub fn init_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    Command::new("git")
        .args(["init", path])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", path, "config", "user.email", "test@test"])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", path, "config", "user.name", "Test"])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", path, "commit", "--allow-empty", "-m", "init"])
        .output()
        .unwrap();
    dir
}

/// Create a git repo inside `parent/name` and return its path.
/// Includes user.email and user.name config for CI compatibility.
#[allow(dead_code)]
pub fn init_repo_in(parent: &Path, name: &str) -> PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.to_str().unwrap();
    Command::new("git")
        .args(["init", path])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", path, "config", "user.email", "test@test"])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", path, "config", "user.name", "Test"])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", path, "commit", "--allow-empty", "-m", "init"])
        .output()
        .unwrap();
    dir
}
