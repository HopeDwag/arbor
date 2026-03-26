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
