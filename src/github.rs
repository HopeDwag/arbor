use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

#[derive(Default)]
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

/// Shared, auto-refreshing GitHub cache. A background thread refreshes every 30s.
pub struct SharedGitHubCache {
    inner: Arc<Mutex<GitHubCache>>,
    refreshing: Arc<AtomicBool>,
}

impl SharedGitHubCache {
    /// Create and start background refresh thread.
    /// Starts with empty cache — first refresh happens in background, not blocking startup.
    pub fn new(repo_root: &Path) -> Self {
        let inner = Arc::new(Mutex::new(GitHubCache::default()));
        let refreshing = Arc::new(AtomicBool::new(false));

        let bg_inner = Arc::clone(&inner);
        let bg_refreshing = Arc::clone(&refreshing);
        let bg_path = repo_root.to_path_buf();
        std::thread::spawn(move || {
            // First refresh happens immediately in background
            bg_refreshing.store(true, Ordering::SeqCst);
            let new_cache = GitHubCache::refresh(&bg_path);
            if let Ok(mut guard) = bg_inner.lock() {
                *guard = new_cache;
            }
            bg_refreshing.store(false, Ordering::SeqCst);
            loop {
                std::thread::sleep(Duration::from_secs(30));
                bg_refreshing.store(true, Ordering::SeqCst);
                let new_cache = GitHubCache::refresh(&bg_path);
                if let Ok(mut guard) = bg_inner.lock() {
                    *guard = new_cache;
                }
                bg_refreshing.store(false, Ordering::SeqCst);
            }
        });

        Self { inner, refreshing }
    }

    /// Get a snapshot of the current cache for reading.
    pub fn get(&self, branch: &str) -> Option<PrInfo> {
        let guard = self.inner.lock().ok()?;
        guard.get(branch).cloned()
    }

    /// Trigger a background refresh (non-blocking).
    /// Skips if a refresh is already in progress to prevent unbounded thread spawning.
    pub fn force_refresh(&self, repo_root: &Path) {
        if self.refreshing.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return; // Already refreshing, skip
        }

        let inner = Arc::clone(&self.inner);
        let refreshing = Arc::clone(&self.refreshing);
        let path = repo_root.to_path_buf();
        std::thread::spawn(move || {
            let new_cache = GitHubCache::refresh(&path);
            if let Ok(mut guard) = inner.lock() {
                *guard = new_cache;
            }
            refreshing.store(false, Ordering::SeqCst);
        });
    }
}
