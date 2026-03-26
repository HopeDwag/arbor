use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 3;

const SKIP_DIRS: &[&str] = &[
    "node_modules", "vendor", "target", "__pycache__", "build", "dist",
];

pub struct DiscoveredRepo {
    pub name: String,
    pub path: PathBuf,
}

pub fn discover_repos(root: &Path) -> Result<Vec<DiscoveredRepo>> {
    let mut repos = Vec::new();
    scan_dir(root, root, 0, &mut repos)?;

    if repos.is_empty() {
        bail!("No git repositories found in {}", root.display());
    }

    repos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(repos)
}

fn scan_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    repos: &mut Vec<DiscoveredRepo>,
) -> Result<()> {
    if depth > MAX_DEPTH {
        return Ok(());
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if ft.is_symlink() {
            continue;
        }
        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        if dir_name.starts_with('.') {
            continue;
        }

        if SKIP_DIRS.contains(&dir_name.as_str()) {
            continue;
        }

        if dir_name.ends_with("-worktrees") {
            continue;
        }

        if path.join(".git").exists() {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let name = rel.to_string_lossy().replace('\\', "/");
            repos.push(DiscoveredRepo { name, path });
            continue;
        }

        scan_dir(root, &path, depth + 1, repos)?;
    }

    Ok(())
}
