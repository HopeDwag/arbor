use anyhow::{bail, Context, Result};
use git2::Repository;
use std::path::{Path, PathBuf};

use super::status::{self, WorktreeStatus};
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
}

pub struct WorktreeManager {
    repo: Repository,
    repo_root: PathBuf,
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
        let main_status = status::check_status(&self.repo_root).ok();
        let (ahead, behind) = status::ahead_behind(&self.repo_root);

        result.push(WorktreeInfo {
            name: main_branch.clone(),
            branch: main_branch,
            path: self.repo_root.clone(),
            is_main: true,
            status: main_status,
            workflow_status: WorkflowStatus::InProgress,
            short_name: None,
            ahead,
            behind,
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
            let wt_status = status::check_status(&wt_path).ok();
            let (ahead, behind) = status::ahead_behind(&wt_path);

            result.push(WorktreeInfo {
                name: name.to_string(),
                branch,
                path: wt_path,
                is_main: false,
                status: wt_status,
                workflow_status: WorkflowStatus::Queued,
                short_name: None,
                ahead,
                behind,
            });
        }

        // Sort: main first, then by most recent commit
        result.sort_by(|a, b| {
            match (a.is_main, b.is_main) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    let age_a = a.status.as_ref().map(|s| s.last_commit_age_secs).unwrap_or(u64::MAX);
                    let age_b = b.status.as_ref().map(|s| s.last_commit_age_secs).unwrap_or(u64::MAX);
                    age_a.cmp(&age_b)
                }
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
        // Check if it's the main worktree
        let worktrees = self.list()?;
        let wt = worktrees
            .iter()
            .find(|w| w.name == name)
            .context("Worktree not found")?;
        if wt.is_main {
            bail!("Cannot delete the main worktree");
        }

        let worktree = self.repo.find_worktree(name)?;
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
