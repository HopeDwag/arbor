use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrState {
    Open,
    Draft,
    Merged,
    Closed,
}

#[derive(Debug, Clone)]
pub struct PrInfo {
    pub number: u32,
    pub state: PrState,
    pub url: String,
}

pub struct GitHubCache {
    prs: HashMap<String, PrInfo>,
}

#[derive(Deserialize)]
struct GhPrEntry {
    number: u32,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    state: String,
    #[serde(rename = "isDraft")]
    is_draft: bool,
    url: String,
}

impl GitHubCache {
    pub fn empty() -> Self {
        Self { prs: HashMap::new() }
    }

    pub fn from_json(json: &str) -> Self {
        let entries: Vec<GhPrEntry> = match serde_json::from_str(json) {
            Ok(e) => e,
            Err(_) => return Self::empty(),
        };

        let mut prs = HashMap::new();
        for entry in entries {
            let state = match (entry.state.as_str(), entry.is_draft) {
                ("OPEN", true) => PrState::Draft,
                ("OPEN", false) => PrState::Open,
                ("MERGED", _) => PrState::Merged,
                ("CLOSED", _) => PrState::Closed,
                _ => PrState::Open,
            };
            prs.insert(entry.head_ref_name, PrInfo {
                number: entry.number,
                state,
                url: entry.url,
            });
        }
        Self { prs }
    }

    /// Refresh by shelling out to `gh pr list` in the given repo directory.
    /// Returns empty cache if `gh` is not installed or fails.
    pub fn refresh(repo_root: &Path) -> Self {
        let output = Command::new("gh")
            .args(["pr", "list", "--state", "all", "--json", "number,headRefName,state,isDraft,url", "--limit", "100"])
            .current_dir(repo_root)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json = String::from_utf8_lossy(&out.stdout);
                Self::from_json(&json)
            }
            _ => Self::empty(),
        }
    }

    pub fn get(&self, branch: &str) -> Option<&PrInfo> {
        self.prs.get(branch)
    }
}
