# UAT Harness Design

Automated exploratory testing for Arbor using a tmux-based harness driven by Claude.

## Problem

Arbor has solid unit/integration tests for the data layer (worktree CRUD, config persistence, GitHub parsing, key dispatch, mouse events) but no end-to-end validation that the TUI actually works as a user would experience it. Rendering, PTY interaction, dialog flows, and visual state transitions are untested.

## Approach

A shell script harness that manages a tmux session running Arbor against a disposable test repo. Claude drives the app interactively via `tmux send-keys` and reads the screen via `tmux capture-pane`. This enables exploratory testing where Claude exercises user flows, reasons about what it sees, and reports findings. Bugs or important behaviors discovered during exploration can later be codified as scripted regression tests.

## Components

### `uat/harness.sh`

A sourceable shell script providing these functions:

| Function | Purpose |
|----------|---------|
| `uat_start [repo_path]` | Build Arbor, create temp git repo (or use provided path), seed test data, launch Arbor in tmux session `arbor-uat` |
| `uat_capture` | Print current screen content via `tmux capture-pane -p -t arbor-uat` |
| `uat_send <keys>` | Send keystrokes via `tmux send-keys -t arbor-uat <keys>` |
| `uat_stop` | Kill tmux session, clean up temp directories |
| `uat_wait [ms]` | Sleep to let TUI render (default 200ms) |

### Test repo seeding

`uat_start` (when no repo path is given) creates a disposable repo in `/tmp/arbor-uat-XXXX/` with:

- An initial commit on `main`
- 2-3 branches with commits (`feature-auth`, `feature-api`)
- One branch set up as an active worktree
- A `.arbor.json` with pre-set workflow statuses for verifying sidebar grouping

Every session starts from a known, reproducible state.

### Claude's UAT workflow

```
source uat/harness.sh && uat_start
uat_capture                          # read initial screen
uat_send "j"                         # press a key
uat_wait && uat_capture              # wait for render, read result
# ... explore flows, reason about output ...
uat_stop                             # clean up
```

## User flows to explore

Not a rigid checklist — Claude uses judgement. Key areas:

**Core navigation:**
- App launches and renders sidebar + terminal
- j/k moves selection, Enter activates a worktree
- Shift+Arrow switches focus between panes
- Terminal accepts input and shows output

**Worktree lifecycle:**
- `n` opens create dialog, name entry + Enter creates worktree
- New worktree appears in sidebar and is selectable
- `a` shows archive confirmation, `y` removes worktree
- Archived branch available for restoration via Tab in create dialog

**Status & display:**
- `s` cycles workflow status, sidebar grouping updates
- Status bar shows correct keybindings for current focus
- PR info bar renders when applicable

**Edge cases:**
- Duplicate worktree name creation
- Archiving the main worktree (should be blocked)
- Rapid key presses, empty states
- Quitting via `q` and `Esc`

## Output

Each UAT session produces a summary: what worked, what broke, what felt off. Findings worth locking down get converted to scripted integration tests (future work).

## Dependencies

- `tmux` (installed via Homebrew)
- Arbor binary (`cargo build` in harness)
- Git (for temp repo creation)

## File structure

```
uat/
└── harness.sh    # The tmux harness script
```
