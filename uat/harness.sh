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
    git init "$repo"
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
