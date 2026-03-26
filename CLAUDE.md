# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is Arbor

Arbor is a TUI (terminal user interface) git worktree manager written in Rust. It provides a split-pane interface with a sidebar listing worktrees and an embedded terminal (PTY) per worktree. Users can create, select, and archive worktrees without leaving the app. Worktrees are stored in a sibling directory named `<repo>-worktrees/`.

## Commands

```bash
cargo build                    # Build
cargo run                      # Run (must be inside a git repo)
cargo run -- --repo /path      # Run against a specific repo
cargo run -- --worktree NAME   # Start with a specific worktree selected
cargo test                     # Run all tests
cargo test test_create         # Run a single test by name
cargo clippy                   # Lint
```

Integration tests in `tests/worktree_manager.rs` create temporary git repos via `git init` + `git commit --allow-empty`. They test `WorktreeManager` CRUD operations only (no TUI).

### UAT (end-to-end TUI testing)

```bash
./uat/run_tests.sh             # Run all UAT tests (~30s, requires tmux)
```

UAT tests use a tmux-based harness (`uat/harness.sh`) that launches Arbor in a detached tmux session against a disposable git repo, sends keystrokes via `tmux send-keys`, and asserts on screen content via `tmux capture-pane`. For exploratory testing with Claude:

```bash
source uat/harness.sh && uat_start   # Build, seed repo, launch in tmux
uat_capture                          # Read current screen
uat_send j                           # Send keystrokes
uat_wait && uat_capture              # Wait for render, read again
uat_stop                             # Kill session, clean up
```

## Architecture

The app follows a single-threaded event loop pattern using ratatui + crossterm:

- **`app::App`** — owns all state, runs the main `poll → read → dispatch` loop. Handles keyboard/mouse events, manages dialogs (create/archive), and maps focus state to actions.
- **`keys`** — translates raw `KeyEvent` into semantic `Action` variants based on current `Focus` (Sidebar vs Terminal). Shift+Arrow switches panes; Ctrl-a is a toggle fallback.
- **`worktree::WorktreeManager`** — wraps `git2::Repository` for worktree CRUD. Creates worktrees as sibling directories (`<repo>-worktrees/<branch>`). Lists worktrees sorted by main-first then by commit recency. "Archive" means delete the worktree directory but keep the branch.
- **`pty::PtySession`** — spawns a shell in a `portable-pty` pseudo-terminal, feeds output to a `vt100_ctt::Parser` on a background reader thread. The parser is shared via `Arc<Mutex<Parser>>`.
- **`ui::TerminalWidget`** — ratatui widget that reads the vt100 screen buffer cell-by-cell and renders it, translating vt100 colors to ratatui colors. Supports a dimmed mode when sidebar is focused.
- **`ui::sidebar`** — renders the worktree list with status indicators (dirty/clean, commit age) and dialog overlays for create/archive operations.

Key data flow: `crossterm event → keys::handle_key → Action → App::handle_action → (WorktreeManager | PtySession | UI state)`.

## Key Dependencies

- **ratatui** + **crossterm** — TUI rendering and terminal event handling
- **git2** — libgit2 bindings for all git operations (no shelling out to git)
- **portable-pty** — cross-platform PTY allocation
- **vt100-ctt** — terminal emulation/parsing (note: the crate is `vt100-ctt`, not `vt100`)
- **clap** — CLI argument parsing with derive macros
