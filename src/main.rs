use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "arbor", version, about = "Git worktree manager with embedded terminal")]
struct Cli {
    /// Path to git repository or parent directory (defaults to current directory)
    #[arg(long)]
    repo: Option<PathBuf>,

    /// Worktree to select on startup (e.g. "feature-auth" or "Enablis/arbor/feature-auth")
    #[arg(long)]
    worktree: Option<String>,

    /// Key to toggle focus between sidebar and terminal (default: ctrl-a)
    #[arg(long, default_value = "ctrl-a")]
    toggle_key: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let path = match &cli.repo {
        Some(p) => p.clone(),
        None => std::env::current_dir()?,
    };

    let mut app = arbor::app::App::new(&path)?;

    if let Some(ref wt_name) = cli.worktree {
        if let Some(idx) = app.sidebar_state.worktrees.iter()
            .position(|w| {
                // Try repo/branch match first (e.g. "Enablis/arbor/feature-auth")
                if let Some(ref rn) = w.repo_name {
                    if format!("{}/{}", rn, w.branch) == *wt_name {
                        return true;
                    }
                }
                w.branch == *wt_name || w.name == *wt_name
            })
        {
            app.sidebar_state.selected = idx;
        } else {
            eprintln!("arbor: worktree '{}' not found, starting with default", wt_name);
        }
    }

    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableMouseCapture,
        crossterm::event::EnableBracketedPaste,
        crossterm::cursor::SetCursorStyle::BlinkingBar,
    )?;
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste,
        crossterm::cursor::SetCursorStyle::DefaultUserShape,
    )?;
    result
}
