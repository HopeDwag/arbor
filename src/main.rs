use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "arbor", version, about = "Git worktree manager with embedded terminal")]
struct Cli {
    /// Path to git repository (defaults to current directory)
    #[arg(long)]
    repo: Option<PathBuf>,

    /// Worktree branch to select on startup
    #[arg(long)]
    worktree: Option<String>,

    /// Key to toggle focus between sidebar and terminal (default: ctrl-a)
    #[arg(long, default_value = "ctrl-a")]
    toggle_key: String,
}

fn find_repo_root(start: &std::path::Path) -> Result<PathBuf> {
    let repo = git2::Repository::discover(start)
        .context("Not inside a git repository")?;
    let workdir = repo.workdir()
        .or_else(|| repo.path().parent())
        .context("Cannot determine repository root")?;
    Ok(workdir.to_path_buf())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let repo_path = match &cli.repo {
        Some(p) => p.clone(),
        None => std::env::current_dir()?,
    };
    let repo_root = find_repo_root(&repo_path)?;

    let mut app = arbor::app::App::new(&repo_root)?;

    if let Some(ref wt_name) = cli.worktree {
        if let Some(idx) = app.sidebar_state.worktrees.iter()
            .position(|w| w.branch == *wt_name || w.name == *wt_name)
        {
            app.sidebar_state.selected = idx;
        } else {
            eprintln!("arbor: worktree '{}' not found, starting with main", wt_name);
        }
    }

    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    result
}
