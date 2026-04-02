# arbor

A TUI git worktree manager with an embedded terminal per branch.

Split-pane interface: a sidebar listing your worktrees grouped by status, and a full PTY terminal session for each one. Create, switch, and archive worktrees without leaving the app.

## Install

```bash
cargo install --path .
```

Requires `~/.cargo/bin` in your PATH.

## Usage

```bash
arbor                          # Run from any git repo
arbor --repo /path/to/repo     # Run against a specific repo
arbor --worktree feature-auth  # Start with a specific worktree selected
```

Point arbor at a directory containing multiple repos for multi-repo mode — worktrees are prefixed with `repo/branch`.

## How It Works

Worktrees are stored as siblings in `<repo>-worktrees/`. Each gets its own shell session. Status is tracked automatically from runtime state:

| Status | Condition | Icon |
|--------|-----------|------|
| **Root** | Main worktree | `·` |
| **In Progress** | PTY has recent output | spinner |
| **In Review** | Open/draft PR detected via `gh` | `○` |
| **Queued** | PTY exists but idle | `!` |
| **Backlog** | No PTY spawned | `▶` |

Press `s` to park/unpark a worktree to Backlog manually.

## Keybindings

### Terminal focused
| Key | Action |
|-----|--------|
| `Shift+Left` | Switch to sidebar |

### Sidebar focused
| Key | Action |
|-----|--------|
| `Up/Down` | Navigate worktrees |
| `Enter` | Open terminal for selected worktree |
| `n` | Create new worktree |
| `a` | Archive worktree (keeps branch) |
| `s` | Park/unpark to backlog |
| `/` | Filter worktrees |
| `Ctrl+G` | Open PR in browser |
| `q` | Quit |

## Theme

Everforest Dark palette with automatic detection — uses 24-bit RGB in truecolor terminals (iTerm2, Alacritty, Kitty, WezTerm) and falls back to 256-color approximation in basic terminals.

## Requirements

- Rust toolchain
- Git
- `gh` CLI (optional, for PR detection)
- A truecolor terminal for best appearance
