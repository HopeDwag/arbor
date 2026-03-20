use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn sanitize_session_name(branch: &str) -> String {
    let sanitized: String = branch.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    format!("arbor-{}", sanitized)
}

pub fn generate_layout_kdl(worktree_path: &str) -> String {
    format!(
        r#"layout {{
    cwd "{}"
    pane
}}"#,
        worktree_path
    )
}

pub struct ZellijManager {
    layout_dir: PathBuf,
}

impl ZellijManager {
    pub fn new() -> Result<Self> {
        let layout_dir = std::env::temp_dir().join("arbor-layouts");
        std::fs::create_dir_all(&layout_dir)?;
        Ok(Self { layout_dir })
    }

    pub fn create_session(&self, branch: &str, worktree_path: &Path) -> Result<String> {
        let session_name = sanitize_session_name(branch);
        let layout_content = generate_layout_kdl(&worktree_path.to_string_lossy());
        let layout_path = self.layout_dir.join(format!("{}.kdl", session_name));
        std::fs::write(&layout_path, &layout_content)?;
        Ok(session_name)
    }

    pub fn session_exists(&self, session_name: &str) -> bool {
        Command::new("zellij")
            .args(["list-sessions"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(session_name))
            .unwrap_or(false)
    }

    pub fn kill_session(&self, session_name: &str) -> Result<()> {
        Command::new("zellij")
            .args(["kill-sessions", session_name])
            .output()
            .context("Failed to kill zellij session")?;
        let layout_path = self.layout_dir.join(format!("{}.kdl", session_name));
        let _ = std::fs::remove_file(layout_path);
        Ok(())
    }

    pub fn zellij_launch_args(&self, session_name: &str) -> Vec<String> {
        let layout_path = self.layout_dir.join(format!("{}.kdl", session_name));
        vec![
            "--session".to_string(),
            session_name.to_string(),
            "--layout".to_string(),
            layout_path.to_string_lossy().to_string(),
        ]
    }

    pub fn zellij_attach_args(&self, session_name: &str) -> Vec<String> {
        vec![
            "attach".to_string(),
            session_name.to_string(),
        ]
    }

    pub fn cleanup_orphaned(&self, valid_branches: &[&str]) -> Result<Vec<String>> {
        let output = Command::new("zellij")
            .args(["list-sessions"])
            .output()
            .context("Failed to list zellij sessions")?;

        let sessions = String::from_utf8_lossy(&output.stdout);
        let mut cleaned = Vec::new();

        for line in sessions.lines() {
            let session = line.split_whitespace().next().unwrap_or("");
            if !session.starts_with("arbor-") { continue; }

            let is_valid = valid_branches.iter()
                .any(|b| sanitize_session_name(b) == session);

            if !is_valid {
                let _ = self.kill_session(session);
                cleaned.push(session.to_string());
            }
        }
        Ok(cleaned)
    }
}
