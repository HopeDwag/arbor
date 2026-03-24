use anyhow::Result;
use git2::Repository;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct WorktreeStatus {
    pub is_dirty: bool,
    pub last_commit_age_secs: u64,
}

pub fn check_status(worktree_path: &Path) -> Result<WorktreeStatus> {
    let repo = Repository::open(worktree_path)?;
    let statuses = repo.statuses(None)?;
    let is_dirty = !statuses.is_empty();

    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let commit_time = commit.time().seconds() as u64;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let age = now.saturating_sub(commit_time);

    Ok(WorktreeStatus {
        is_dirty,
        last_commit_age_secs: age,
    })
}

/// Calculate how many commits the local branch is ahead/behind its upstream.
/// Returns (ahead, behind). Returns (0, 0) if no upstream is set.
pub fn ahead_behind(repo_path: &Path) -> (u32, u32) {
    let repo = match Repository::open(repo_path) {
        Ok(r) => r,
        Err(_) => return (0, 0),
    };

    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return (0, 0),
    };

    let local_oid = match head.target() {
        Some(oid) => oid,
        None => return (0, 0),
    };

    // Find upstream branch
    let branch_name = match head.shorthand() {
        Some(name) => name.to_string(),
        None => return (0, 0),
    };

    let branch = match repo.find_branch(&branch_name, git2::BranchType::Local) {
        Ok(b) => b,
        Err(_) => return (0, 0),
    };

    let upstream = match branch.upstream() {
        Ok(u) => u,
        Err(_) => return (0, 0), // No upstream configured
    };

    let upstream_oid = match upstream.get().target() {
        Some(oid) => oid,
        None => return (0, 0),
    };

    match repo.graph_ahead_behind(local_oid, upstream_oid) {
        Ok((ahead, behind)) => (ahead as u32, behind as u32),
        Err(_) => (0, 0),
    }
}

pub fn format_age(secs: u64) -> String {
    if secs < 60 {
        return format!("{}s", secs);
    }
    if secs < 3600 {
        return format!("{}m", secs / 60);
    }
    if secs < 86400 {
        return format!("{}h", secs / 3600);
    }
    if secs < 604800 {
        return format!("{}d", secs / 86400);
    }
    format!("{}w", secs / 604800)
}
