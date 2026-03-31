use anyhow::{bail, Context, Result};
use git2::Repository;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::status::WorktreeStatus;
use crate::persistence::WorkflowStatus;

pub struct WorktreeInfo {
    pub name: String,
    pub branch: String,
    pub path: PathBuf,
    pub is_main: bool,
    pub status: Option<WorktreeStatus>,
    pub workflow_status: WorkflowStatus,
    pub short_name: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub repo_name: Option<String>,
    pub repo_root: PathBuf,
    /// Eagerly computed commit age for sort ordering.
    /// Populated during `list()` so sorting works before lazy status check.
    pub last_commit_age_secs: u64,
    pub commit_message: Option<String>,
    pub is_dirty: bool,
    pub pr: Option<(u32, crate::github::PrState)>,
}

pub struct WorktreeManager {
    repo: Repository,
    repo_root: PathBuf,
}

/// Read the HEAD commit timestamp and return age in seconds.
/// Returns `u64::MAX` if the age cannot be determined.
fn commit_age_secs(repo: &Repository) -> u64 {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return u64::MAX,
    };
    let commit = match head.peel_to_commit() {
        Ok(c) => c,
        Err(_) => return u64::MAX,
    };
    let commit_time = commit.time().seconds() as u64;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(commit_time)
}

fn commit_summary(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    commit.summary().map(String::from)
}

fn is_repo_dirty(repo: &Repository) -> bool {
    repo.statuses(None)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

impl WorktreeManager {
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::discover(path).context("Not a git repository")?;
        let repo_root = repo
            .workdir()
            .or_else(|| repo.path().parent())
            .context("Cannot determine repo root")?
            .to_path_buf();
        Ok(Self { repo, repo_root })
    }

    pub fn list(&self) -> Result<Vec<WorktreeInfo>> {
        let mut result = Vec::new();

        // Main worktree
        let head = self.repo.head().ok();
        let main_branch = head
            .as_ref()
            .and_then(|h| h.shorthand().map(String::from))
            .unwrap_or_else(|| "HEAD".to_string());

        let main_name = self.repo_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "main".to_string());

        result.push(WorktreeInfo {
            name: main_name,
            branch: main_branch,
            path: self.repo_root.clone(),
            is_main: true,
            status: None,
            workflow_status: WorkflowStatus::InProgress,
            short_name: None,
            ahead: 0,
            behind: 0,
            repo_name: None,
            repo_root: self.repo_root.clone(),
            last_commit_age_secs: commit_age_secs(&self.repo),
            commit_message: commit_summary(&self.repo),
            is_dirty: is_repo_dirty(&self.repo),
            pr: None,
        });

        // Additional worktrees
        let worktrees = self.repo.worktrees()?;
        for name in worktrees.iter() {
            let Some(name) = name else { continue };
            let wt = self.repo.find_worktree(name)?;
            let wt_path = wt.path().to_path_buf();
            let wt_repo = Repository::open(&wt_path)?;
            let branch = wt_repo
                .head()
                .ok()
                .and_then(|h| h.shorthand().map(String::from))
                .unwrap_or_else(|| name.to_string());

            let age = commit_age_secs(&wt_repo);
            result.push(WorktreeInfo {
                name: name.to_string(),
                branch,
                path: wt_path,
                is_main: false,
                status: None,
                workflow_status: WorkflowStatus::Queued,
                short_name: None,
                ahead: 0,
                behind: 0,
                repo_name: None,
                repo_root: self.repo_root.clone(),
                last_commit_age_secs: age,
                commit_message: commit_summary(&wt_repo),
                is_dirty: is_repo_dirty(&wt_repo),
                pr: None,
            });
        }

        // Sort: main first, then by most recent commit
        result.sort_by(|a, b| {
            match (a.is_main, b.is_main) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.last_commit_age_secs.cmp(&b.last_commit_age_secs),
            }
        });

        Ok(result)
    }

    pub fn repo_root(&self) -> &std::path::Path {
        &self.repo_root
    }

    pub fn create(&self, branch_name: &str) -> Result<PathBuf> {
        let worktree_dir = self.worktree_base_dir();
        std::fs::create_dir_all(&worktree_dir)?;
        let wt_path = worktree_dir.join(branch_name);

        // Create branch if it doesn't exist
        let head_commit = self.repo.head()?.peel_to_commit()?;
        if self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)
            .is_err()
        {
            self.repo.branch(branch_name, &head_commit, false)?;
        }

        let reference = self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)?
            .into_reference();

        let mut opts = git2::WorktreeAddOptions::new();
        opts.reference(Some(&reference));
        self.repo.worktree(branch_name, &wt_path, Some(&opts))?;

        Ok(wt_path)
    }

    pub fn delete(&self, name: &str, force: bool) -> Result<()> {
        let worktree = self.repo.find_worktree(name)
            .context("Worktree not found")?;
        if matches!(worktree.is_locked(), Ok(git2::WorktreeLockStatus::Locked(_))) {
            if !force {
                bail!("Worktree is locked");
            }
            worktree.unlock()?;
        }

        // Remove the working directory first so git2 doesn't consider it "valid"
        let wt_path = self.worktree_base_dir().join(name);
        if wt_path.exists() {
            std::fs::remove_dir_all(&wt_path)?;
        }

        let mut prune_opts = git2::WorktreePruneOptions::new();
        prune_opts.working_tree(true);
        prune_opts.valid(true);
        worktree.prune(Some(&mut prune_opts))?;

        Ok(())
    }

    /// List local branches that don't have an active worktree (archived).
    pub fn archived_branches(&self) -> Result<Vec<String>> {
        let active: Vec<String> = self.list()?.into_iter().map(|w| w.branch).collect();
        let mut archived = Vec::new();
        let branches = self.repo.branches(Some(git2::BranchType::Local))?;
        for branch in branches {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                if !active.contains(&name.to_string()) {
                    archived.push(name.to_string());
                }
            }
        }
        Ok(archived)
    }

    fn worktree_base_dir(&self) -> PathBuf {
        let repo_name = self
            .repo_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "repo".to_string());
        self.repo_root
            .parent()
            .unwrap_or(&self.repo_root)
            .join(format!("{}-worktrees", repo_name))
    }
}
