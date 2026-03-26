#!/usr/bin/env bash
# UAT harness for Arbor — source this file, then call uat_start / uat_stop
set -euo pipefail

ARBOR_UAT_SESSION="arbor-uat"
ARBOR_UAT_TMPDIR=""
ARBOR_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

_uat_seed_repo() {
    ARBOR_UAT_TMPDIR="$(mktemp -d /tmp/arbor-uat-XXXX)"
    local repo="$ARBOR_UAT_TMPDIR"

    # Initialize repo with a commit on main
    git init "$repo" >/dev/null 2>&1
    git -C "$repo" config user.email "uat@test"
    git -C "$repo" config user.name "UAT"
    git -C "$repo" checkout -b main >/dev/null 2>&1
    git -C "$repo" commit --allow-empty -m "initial commit" >/dev/null 2>&1

    # Create branches with commits
    git -C "$repo" checkout -b feature-auth >/dev/null 2>&1
    git -C "$repo" commit --allow-empty -m "auth: add login endpoint" >/dev/null 2>&1
    git -C "$repo" checkout main >/dev/null 2>&1

    git -C "$repo" checkout -b feature-api >/dev/null 2>&1
    git -C "$repo" commit --allow-empty -m "api: add REST routes" >/dev/null 2>&1
    git -C "$repo" checkout main >/dev/null 2>&1

    # Create one active worktree
    local wt_dir="${repo}-worktrees"
    mkdir -p "$wt_dir"
    git -C "$repo" worktree add "$wt_dir/feature-auth" feature-auth >/dev/null 2>&1

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
