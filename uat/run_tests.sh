#!/usr/bin/env bash
# Automated UAT tests for Arbor — runs scenarios via tmux harness
# Usage: ./uat/run_tests.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/harness.sh"

PASS=0
FAIL=0
ERRORS=""

# --- Test helpers ---

assert_screen_contains() {
    local label="$1"
    local pattern="$2"
    local screen
    screen="$(uat_capture)"
    if echo "$screen" | grep -q "$pattern"; then
        return 0
    else
        echo "  ASSERTION FAILED: expected screen to contain '$pattern'"
        echo "  Screen was:"
        echo "$screen" | sed 's/^/    /'
        return 1
    fi
}

assert_screen_not_contains() {
    local label="$1"
    local pattern="$2"
    local screen
    screen="$(uat_capture)"
    if echo "$screen" | grep -q "$pattern"; then
        echo "  ASSERTION FAILED: expected screen NOT to contain '$pattern'"
        echo "  Screen was:"
        echo "$screen" | sed 's/^/    /'
        return 1
    else
        return 0
    fi
}

assert_session_alive() {
    if tmux has-session -t "$ARBOR_UAT_SESSION" 2>/dev/null; then
        return 0
    else
        echo "  ASSERTION FAILED: tmux session '$ARBOR_UAT_SESSION' is not running"
        return 1
    fi
}

run_test() {
    local name="$1"
    local fn="$2"
    echo -n "  $name ... "
    if $fn; then
        echo "PASS"
        PASS=$((PASS + 1))
    else
        echo "FAIL"
        FAIL=$((FAIL + 1))
        ERRORS="$ERRORS\n  - $name"
    fi
}

# --- Tests ---

test_app_launches() {
    uat_start
    uat_wait 500
    assert_session_alive &&
    assert_screen_contains "sidebar header" "arbor" &&
    assert_screen_contains "main worktree" "main" &&
    assert_screen_contains "feature-auth worktree" "feature-auth" &&
    assert_screen_contains "status bar" "sidebar"
    local result=$?
    uat_stop
    return $result
}

test_sidebar_navigation() {
    uat_start
    uat_wait 500
    # Switch to sidebar
    uat_send S-Left
    uat_wait
    assert_screen_contains "sidebar keybindings" "j/k navigate"
    local result=$?
    if [ $result -eq 0 ]; then
        # Navigate down to feature-auth
        uat_send j
        uat_wait
        assert_screen_contains "feature-auth selected" "feature-auth"
        result=$?
    fi
    uat_stop
    return $result
}

test_duplicate_worktree_no_crash() {
    uat_start
    uat_wait 500
    # Switch to sidebar, open create dialog
    uat_send S-Left
    uat_wait
    uat_send n
    uat_wait
    # Type a branch name that already has a worktree
    uat_send "feature-auth"
    uat_send Enter
    uat_wait 500
    # App should still be running
    assert_session_alive &&
    assert_screen_contains "sidebar still visible" "main"
    local result=$?
    uat_stop
    return $result
}

test_short_name_displayed() {
    uat_start
    uat_wait 500
    # Switch to sidebar, create worktree with prefixed branch
    uat_send S-Left
    uat_wait
    uat_send n
    uat_wait
    uat_send "feature/my-short-name"
    uat_send Enter
    uat_wait 1000
    # Switch to sidebar and check auto-derived short name is displayed
    uat_send S-Left
    uat_wait
    # The short name (last segment after /) should appear in the sidebar
    assert_screen_contains "short name in sidebar" "my-short-name"
    local result=$?
    uat_stop
    return $result
}

test_create_archive_restore() {
    uat_start
    uat_wait 500
    uat_send S-Left
    uat_wait
    # Create a new worktree
    uat_send n
    uat_wait
    uat_send "temp-branch"
    uat_send Enter
    uat_wait 1000
    uat_send S-Left
    uat_wait
    assert_screen_contains "new worktree created" "temp-branch"
    local result=$?
    if [ $result -ne 0 ]; then uat_stop; return $result; fi

    # Archive it
    # Navigate to temp-branch (it should be selected after creation, but
    # we may need to find it)
    uat_send a
    uat_wait
    assert_screen_contains "archive dialog" "Archive worktree"
    result=$?
    if [ $result -ne 0 ]; then uat_stop; return $result; fi

    uat_send y
    uat_wait 500
    assert_screen_not_contains "worktree removed" "temp-branch"
    result=$?
    if [ $result -ne 0 ]; then uat_stop; return $result; fi

    # Restore via create dialog + Tab
    uat_send n
    uat_wait
    uat_send Tab
    uat_wait
    # Look for temp-branch in the restore options
    local screen
    screen="$(uat_capture)"
    local found=0
    # Tab through archived branches to find temp-branch
    for i in 1 2 3 4 5; do
        if echo "$screen" | grep -q "temp-branch"; then
            found=1
            break
        fi
        uat_send Tab
        uat_wait
        screen="$(uat_capture)"
    done
    if [ $found -eq 0 ]; then
        echo "  ASSERTION FAILED: temp-branch not found in archived branches"
        uat_stop
        return 1
    fi
    uat_send Enter
    uat_wait 1000
    uat_send S-Left
    uat_wait
    assert_screen_contains "restored worktree" "temp-branch"
    result=$?
    uat_stop
    return $result
}

test_status_cycling() {
    uat_start
    uat_wait 500
    uat_send S-Left
    uat_wait
    # Navigate to feature-auth
    uat_send j
    uat_wait
    # Cycle status: InProgress -> Done
    uat_send s
    uat_wait
    assert_screen_contains "status changed to done" "DONE"
    local result=$?
    if [ $result -ne 0 ]; then uat_stop; return $result; fi

    # Cycle again: Done -> Queued
    uat_send s
    uat_wait
    assert_screen_contains "status changed to queued" "QUEUED"
    result=$?
    if [ $result -ne 0 ]; then uat_stop; return $result; fi

    # Cycle again: Queued -> InProgress
    uat_send s
    uat_wait
    # feature-auth should be back under IN PROGRESS with main
    assert_screen_contains "status back to in progress" "feature-auth"
    result=$?
    uat_stop
    return $result
}

test_main_cannot_be_archived() {
    uat_start
    uat_wait 500
    uat_send S-Left
    uat_wait
    # main should be selected by default, press a
    uat_send a
    uat_wait
    # No archive dialog should appear
    assert_screen_not_contains "no archive dialog" "Archive worktree" &&
    assert_screen_contains "main still present" "main"
    local result=$?
    uat_stop
    return $result
}

test_terminal_input() {
    uat_start
    uat_wait 500
    # Terminal is focused by default, type a command
    uat_send "echo UAT_MARKER_12345"
    uat_send Enter
    uat_wait 500
    assert_screen_contains "command output" "UAT_MARKER_12345"
    local result=$?
    uat_stop
    return $result
}

test_esc_cancels_dialog() {
    uat_start
    uat_wait 500
    uat_send S-Left
    uat_wait
    uat_send n
    uat_wait
    assert_screen_contains "dialog open" "New worktree"
    local result=$?
    if [ $result -ne 0 ]; then uat_stop; return $result; fi

    uat_send "some-text"
    uat_send Escape
    uat_wait
    assert_screen_not_contains "dialog closed" "New worktree" &&
    assert_session_alive
    result=$?
    uat_stop
    return $result
}

# --- Runner ---

echo ""
echo "Arbor UAT Tests"
echo "================"
echo ""
echo "Building Arbor..."
cargo build --manifest-path "$ARBOR_ROOT/Cargo.toml" 2>&1 | tail -1
echo ""

run_test "App launches and renders correctly" test_app_launches
run_test "Sidebar navigation with j/k" test_sidebar_navigation
run_test "Duplicate worktree creation doesn't crash" test_duplicate_worktree_no_crash
run_test "Short name displayed in sidebar" test_short_name_displayed
run_test "Create, archive, and restore worktree" test_create_archive_restore
run_test "Status cycling (InProgress → Done → Queued → InProgress)" test_status_cycling
run_test "Main worktree cannot be archived" test_main_cannot_be_archived
run_test "Terminal accepts input and shows output" test_terminal_input
run_test "Esc cancels create dialog" test_esc_cancels_dialog

echo ""
echo "Results: $PASS passed, $FAIL failed"
if [ $FAIL -gt 0 ]; then
    echo -e "Failed tests:$ERRORS"
    exit 1
fi
