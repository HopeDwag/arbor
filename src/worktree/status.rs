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
