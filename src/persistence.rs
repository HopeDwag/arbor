use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    #[default]
    Backlog,
    Queued,
    InProgress,
    InReview,
}

impl WorkflowStatus {
    /// Cycle through manual statuses. Returns None when cycling past InProgress
    /// (caller should trigger archive). Skips InReview — that's auto from PR state.
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Backlog => Some(Self::Queued),
            Self::Queued => Some(Self::InProgress),
            Self::InProgress => None, // signals "archive this"
            Self::InReview => Some(Self::InProgress), // manual override out of review
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    pub status: WorkflowStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            status: WorkflowStatus::Backlog,
            short_name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArborConfig {
    pub worktrees: HashMap<String, WorktreeConfig>,
}

impl ArborConfig {
    pub fn load(repo_root: &Path) -> Self {
        let path = repo_root.join(".arbor.json");
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("arbor: warning: malformed .arbor.json: {}", e);
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, repo_root: &Path) -> anyhow::Result<()> {
        let path = repo_root.join(".arbor.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }
}
