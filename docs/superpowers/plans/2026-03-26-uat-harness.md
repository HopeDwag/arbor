# UAT Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a tmux-based shell harness that lets Claude drive Arbor interactively for exploratory UAT testing.

**Architecture:** A single sourceable shell script (`uat/harness.sh`) that manages the full lifecycle: build Arbor, seed a disposable git repo with worktrees and config, launch Arbor inside a tmux session, and expose helper functions for sending keys and capturing screen output. No Rust code changes needed.

**Tech Stack:** Bash, tmux, git

**Spec:** `docs/superpowers/specs/2026-03-26-uat-harness-design.md`

---

### Task 1: Create `uat/harness.sh` with skeleton and `uat_stop`

**Files:**
- Create: `uat/harness.sh`

Start with the simplest function — cleanup — and the script skeleton with constants.

- [ ] **Step 1: Create the harness script skeleton**

```bash
#!/usr/bin/env bash
# UAT harness for Arbor — source this file, then call uat_start / uat_stop
set -euo pipefail

ARBOR_UAT_SESSION="arbor-uat"
ARBOR_UAT_TMPDIR=""
ARBOR_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

uat_stop() {
    tmux kill-session -t "$ARBOR_UAT_SESSION" 2>/dev/null || true
    if [[ -n "$ARBOR_UAT_TMPDIR" && "$ARBOR_UAT_TMPDIR" == /tmp/arbor-uat-* ]]; then
        rm -rf "$ARBOR_UAT_TMPDIR"
        # Also clean up the sibling worktrees directory
        rm -rf "${ARBOR_UAT_TMPDIR}-worktrees"
        ARBOR_UAT_TMPDIR=""
    fi
    echo "UAT session stopped."
}
```

- [ ] **Step 2: Verify the script is syntactically valid**

Run: `bash -n uat/harness.sh`
Expected: no output (no syntax errors)

- [ ] **Step 3: Commit**

```bash
git add uat/harness.sh
git commit -m "feat(uat): add harness skeleton with uat_stop"
```

---

### Task 2: Add `uat_wait`, `uat_send`, and `uat_capture`

**Files:**
- Modify: `uat/harness.sh`

These are thin wrappers around tmux commands. `uat_send` passes arguments directly to `tmux send-keys` — callers use tmux key syntax (e.g. `Enter`, `Escape`, `C-a`, literal strings).

- [ ] **Step 1: Add the three helper functions**

Append before the closing of the file:

```bash
uat_wait() {
    local ms="${1:-200}"
    sleep "$(echo "scale=3; $ms/1000" | bc)"
}

uat_send() {
    tmux send-keys -t "$ARBOR_UAT_SESSION" "$@"
}

uat_capture() {
    tmux capture-pane -t "$ARBOR_UAT_SESSION" -p
}
```

- [ ] **Step 2: Verify syntax**

Run: `bash -n uat/harness.sh`
Expected: no output

- [ ] **Step 3: Commit**

```bash
git add uat/harness.sh
git commit -m "feat(uat): add uat_wait, uat_send, uat_capture helpers"
```

---

### Task 3: Add test repo seeding function

**Files:**
- Modify: `uat/harness.sh`

A `_uat_seed_repo` internal function that creates a disposable git repo with branches, worktrees, and an `.arbor.json`.

- [ ] **Step 1: Add the seed function**

Add before `uat_stop`:

```bash
_uat_seed_repo() {
    ARBOR_UAT_TMPDIR="$(mktemp -d /tmp/arbor-uat-XXXX)"
    local repo="$ARBOR_UAT_TMPDIR"

    # Initialize repo with a commit on main
    git init "$repo" >/dev/null 2>&1
    git -C "$repo" config user.email "uat@test"
    git -C "$repo" config user.name "UAT"
    git -C "$repo" checkout -b main
    git -C "$repo" commit --allow-empty -m "initial commit"

    # Create branches with commits
    git -C "$repo" checkout -b feature-auth
    git -C "$repo" commit --allow-empty -m "auth: add login endpoint"
    git -C "$repo" checkout main

    git -C "$repo" checkout -b feature-api
    git -C "$repo" commit --allow-empty -m "api: add REST routes"
    git -C "$repo" checkout main

    # Create one active worktree
    local wt_dir="${repo}-worktrees"
    mkdir -p "$wt_dir"
    git -C "$repo" worktree add "$wt_dir/feature-auth" feature-auth

    # Write .arbor.json with pre-set statuses
    cat > "$repo/.arbor.json" <<'ARBORJSON'
{
  "worktrees": {
    "feature-auth": {
      "status": "in_progress"
    },
    "feature-api": {
      "status": "queued"
    }
  }
}
ARBORJSON

    echo "$repo"
}
```

- [ ] **Step 2: Test the seed function in isolation**

Run: `source uat/harness.sh && _uat_seed_repo && ls "$ARBOR_UAT_TMPDIR" && cat "$ARBOR_UAT_TMPDIR/.arbor.json" && ls "${ARBOR_UAT_TMPDIR}-worktrees/" && uat_stop`
Expected: repo directory listed, `.arbor.json` contents shown, `feature-auth` worktree directory listed, cleanup succeeds

- [ ] **Step 3: Commit**

```bash
git add uat/harness.sh
git commit -m "feat(uat): add test repo seeding with branches and worktrees"
```

---

### Task 4: Add `uat_start` — build, seed, launch

**Files:**
- Modify: `uat/harness.sh`

The main entry point. Builds Arbor, optionally seeds a repo, and launches Arbor in a tmux session.

- [ ] **Step 1: Add uat_start function**

Add after `_uat_seed_repo`:

```bash
uat_start() {
    local repo_path="${1:-}"

    # Kill any existing session
    tmux kill-session -t "$ARBOR_UAT_SESSION" 2>/dev/null || true

    # Build Arbor
    echo "Building Arbor..."
    cargo build --manifest-path "$ARBOR_ROOT/Cargo.toml" 2>&1
    local arbor_bin="$ARBOR_ROOT/target/debug/arbor"

    if [[ ! -x "$arbor_bin" ]]; then
        echo "ERROR: Arbor binary not found at $arbor_bin"
        return 1
    fi

    # Seed repo if no path provided
    if [[ -z "$repo_path" ]]; then
        repo_path="$(_uat_seed_repo)"
        echo "Seeded test repo at $repo_path"
    fi

    # Launch Arbor in a detached tmux session
    tmux new-session -d -s "$ARBOR_UAT_SESSION" -x 120 -y 40 \
        "$arbor_bin --repo $repo_path"

    # Give it a moment to start
    sleep 1

    echo "UAT session started. Use uat_capture, uat_send, uat_stop."
}
```

- [ ] **Step 2: End-to-end test — start, capture, stop**

Run: `source uat/harness.sh && uat_start && uat_capture && uat_stop`
Expected: Arbor builds, tmux session starts, screen capture shows the Arbor TUI (sidebar with worktrees, terminal pane), cleanup succeeds

- [ ] **Step 3: Commit**

```bash
git add uat/harness.sh
git commit -m "feat(uat): add uat_start with build, seed, and tmux launch"
```

---

### Task 5: Smoke test the full workflow

**Files:**
- No changes — validation only

Run through a complete exploratory sequence to verify everything works together.

- [ ] **Step 1: Start a session**

Run: `source uat/harness.sh && uat_start`
Expected: "UAT session started." message

- [ ] **Step 2: Capture and verify initial render**

Run: `uat_capture`
Expected: Screen shows Arbor TUI with sidebar listing `main`, `feature-auth` worktrees. Status groups visible.

- [ ] **Step 3: Navigate and interact**

Run:
```bash
uat_send j && uat_wait && uat_capture    # move selection down
uat_send Enter && uat_wait && uat_capture # select worktree
uat_send S-Right && uat_wait && uat_capture # switch to terminal
```
Expected: Selection moves, worktree activates (terminal shows shell), focus switches to terminal pane

- [ ] **Step 4: Test terminal input**

Run:
```bash
uat_send "echo hello" && uat_send Enter && uat_wait && uat_capture
```
Expected: Terminal shows `echo hello` command and `hello` output

- [ ] **Step 5: Clean up**

Run: `uat_stop`
Expected: "UAT session stopped."

- [ ] **Step 6: Commit (no changes — just verification)**

No commit needed unless fixes were required during smoke testing.
