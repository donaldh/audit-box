#!/bin/bash
# System tests for audit-box using Test Anything Protocol (TAP)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Test counter
TEST_NUM=0

# Path to audit-box binary
AUDIT_BOX="./target/debug/audit-box"

# Cleanup function
cleanup() {
    rm -f "$TEST_SESSION_FILE" 2>/dev/null || true
    rm -rf /tmp/bwrap-overlay-* 2>/dev/null || true

    # Restore backup if it exists
    if [ -f "$TEST_SESSION_FILE.backup" ]; then
        mv "$TEST_SESSION_FILE.backup" "$TEST_SESSION_FILE"
    fi
}

# Setup test environment
setup_test_env() {
    # Use actual config directory but with a test suffix
    TEST_CONFIG_DIR="$HOME/.config/audit-box"
    TEST_SESSION_FILE="$TEST_CONFIG_DIR/sessions"

    # Backup existing session file if it exists
    if [ -f "$TEST_SESSION_FILE" ]; then
        mv "$TEST_SESSION_FILE" "$TEST_SESSION_FILE.backup"
    fi
}

# Helper function to run a test
run_test() {
    local description="$1"
    local command="$2"

    TEST_NUM=$((TEST_NUM + 1))

    if eval "$command" >/dev/null 2>&1; then
        echo "ok $TEST_NUM - $description"
        return 0
    else
        echo "not ok $TEST_NUM - $description"
        return 1
    fi
}

# Helper function to run a test that should fail
run_test_should_fail() {
    local description="$1"
    local command="$2"

    TEST_NUM=$((TEST_NUM + 1))

    if eval "$command" >/dev/null 2>&1; then
        echo "not ok $TEST_NUM - $description (expected failure but succeeded)"
        return 1
    else
        echo "ok $TEST_NUM - $description"
        return 0
    fi
}

# Helper function to check output contains string
run_test_output_contains() {
    local description="$1"
    local command="$2"
    local expected="$3"

    TEST_NUM=$((TEST_NUM + 1))

    local output
    # Use timeout to prevent hanging
    output=$(timeout 5 bash -c "$command" 2>&1 || true)

    if echo "$output" | grep -q "$expected"; then
        echo "ok $TEST_NUM - $description"
        return 0
    else
        echo "not ok $TEST_NUM - $description"
        echo "# Expected output to contain: $expected"
        echo "# Got: $output"
        return 1
    fi
}

# Start tests
echo "TAP version 13"

# Build the project first
echo "# Building audit-box..."
cargo build 2>&1 | grep -E "Finished|Compiling" || true

# Setup test environment
setup_test_env
cleanup

# Count total tests
TOTAL_TESTS=20
echo "1..$TOTAL_TESTS"

echo "# Testing help commands"
run_test "audit-box --help shows help" "$AUDIT_BOX --help"
run_test "audit-box new --help shows help" "$AUDIT_BOX new --help"
run_test "audit-box run --help shows help" "$AUDIT_BOX run --help"
run_test "audit-box review --help shows help" "$AUDIT_BOX review --help"

echo "# Testing new command"
run_test_should_fail "review fails when no session exists" "$AUDIT_BOX review"
run_test "new command creates session" "$AUDIT_BOX new --base $HOME"
run_test "session file exists after new" "test -f $TEST_SESSION_FILE"

# Get session directory
SESSION_DIR=$(head -1 "$TEST_SESSION_FILE")
BASE_PATH=$(sed -n '2p' "$TEST_SESSION_FILE")

run_test "session directory exists" "test -d '$SESSION_DIR'"
run_test "overlay directory exists" "test -d '$SESSION_DIR/overlay'"
run_test "work directory exists" "test -d '$SESSION_DIR/work'"
run_test "session file contains base path" "test '$BASE_PATH' = '$HOME'"

echo "# Testing session file validation"
run_test_output_contains "new command output mentions session directory" "$AUDIT_BOX new --base $HOME" "Session directory:"
run_test_output_contains "new command output mentions overlay directory" "$AUDIT_BOX new --base $HOME" "Overlay directory:"

echo "# Testing review command with session"
run_test_should_fail "review fails with only --overlay" "$AUDIT_BOX review --overlay /tmp/test"
run_test_should_fail "review fails with only --base" "$AUDIT_BOX review --base /tmp/test"
run_test "review accepts both --overlay and --base" "$AUDIT_BOX review --overlay '$SESSION_DIR/overlay' --base '$BASE_PATH' < /dev/null & sleep 0.5; pkill -f 'audit-box review' || true"

echo "# Testing error handling"
echo "/tmp/nonexistent-session-dir" > "$TEST_SESSION_FILE"
echo "$HOME" >> "$TEST_SESSION_FILE"
run_test_output_contains "review shows error for missing session directory" "$AUDIT_BOX review" "no longer exists"

rm -f "$TEST_SESSION_FILE"
run_test_output_contains "review shows error when no session file exists" "$AUDIT_BOX review" "No active session found"

echo "# Testing run command"
if command -v bwrap >/dev/null 2>&1; then
    # Create a fresh session for run tests
    $AUDIT_BOX new --base "$HOME" >/dev/null 2>&1

    run_test "run command executes echo" "$AUDIT_BOX run echo 'test' | grep -q 'test'"
    run_test "run command can use flags" "$AUDIT_BOX run ls -la / >/dev/null 2>&1"
else
    echo "ok $((TEST_NUM + 1)) - run command executes echo # SKIP bwrap not available"
    echo "ok $((TEST_NUM + 2)) - run command can use flags # SKIP bwrap not available"
    TEST_NUM=$((TEST_NUM + 2))
fi

# Cleanup
cleanup

echo "# All tests completed"
