#!/bin/bash
# Overlay isolation tests for audit-box using Test Anything Protocol (TAP)
# These tests require bwrap to be installed

set -e

# Test counter
TEST_NUM=0

# Path to audit-box binary
AUDIT_BOX="./target/debug/audit-box"

# Cleanup function
cleanup() {
    rm -f "$TEST_SESSION_FILE" 2>/dev/null || true
    rm -rf "$TEST_WORKSPACE" 2>/dev/null || true
    rm -rf /tmp/audit-box-* 2>/dev/null || true

    # Restore backup if it exists
    if [ -f "$TEST_SESSION_FILE.backup" ]; then
        mv "$TEST_SESSION_FILE.backup" "$TEST_SESSION_FILE"
    fi
}

# Setup test environment
setup_test_env() {
    TEST_CONFIG_DIR="$HOME/.config/audit-box"
    TEST_SESSION_FILE="$TEST_CONFIG_DIR/sessions"
    TEST_WORKSPACE="$(mktemp -d /tmp/audit-box-test-XXXXXX)"

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

# Helper function to check file content
check_file_content() {
    local description="$1"
    local file="$2"
    local expected="$3"

    TEST_NUM=$((TEST_NUM + 1))

    if [ -f "$file" ] && [ "$(cat "$file")" = "$expected" ]; then
        echo "ok $TEST_NUM - $description"
        return 0
    else
        echo "not ok $TEST_NUM - $description"
        return 1
    fi
}

# Check if bwrap is available
if ! command -v bwrap >/dev/null 2>&1; then
    echo "TAP version 13"
    echo "1..0 # SKIP bwrap not available"
    exit 0
fi

# Start tests
echo "TAP version 13"

# Build the project first
echo "# Building audit-box..."
cargo build 2>&1 | grep -E "Finished|Compiling" || true

# Setup test environment
setup_test_env
cleanup

# Count total tests
TOTAL_TESTS=8
echo "1..$TOTAL_TESTS"

echo "# Creating test workspace"
mkdir -p "$TEST_WORKSPACE"
echo "original content" > "$TEST_WORKSPACE/test-file.txt"

echo "# Testing overlay isolation"
run_test "create session with test workspace" "$AUDIT_BOX new --base '$TEST_WORKSPACE'"

SESSION_DIR=$(head -1 "$TEST_SESSION_FILE")

echo "# Testing file modifications in overlay"
run_test "run command creates file in overlay" "$AUDIT_BOX run bash -c 'echo \"overlay content\" > $TEST_WORKSPACE/new-file.txt'"
run_test "file does not exist in base filesystem" "! test -f '$TEST_WORKSPACE/new-file.txt'"
run_test "file exists in overlay directory" "test -f '$SESSION_DIR/overlay/new-file.txt'"

echo "# Testing file content isolation"
run_test "modify existing file in overlay" "$AUDIT_BOX run bash -c 'echo \"modified content\" > $TEST_WORKSPACE/test-file.txt'"
check_file_content "base file retains original content" "$TEST_WORKSPACE/test-file.txt" "original content"
check_file_content "overlay file has modified content" "$SESSION_DIR/overlay/test-file.txt" "modified content"

echo "# Testing file visibility from within overlay"
run_test "overlay sees modified file" "$AUDIT_BOX run bash -c 'test \"\$(cat $TEST_WORKSPACE/test-file.txt)\" = \"modified content\"'"

# Cleanup
cleanup

echo "# All overlay tests completed"
