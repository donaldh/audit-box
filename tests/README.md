# Audit-Box System Tests

This directory contains system tests for audit-box written in bash shell using the Test Anything Protocol (TAP) format.

## Test Files

- **system-tests.sh** - Core functionality tests including:
  - Help command tests
  - Session creation and management
  - Error handling
  - Command argument validation
  - Basic run command tests

- **overlay-tests.sh** - Overlay filesystem isolation tests including:
  - File creation in overlay
  - File modification isolation
  - Verification that base filesystem remains unchanged
  - Content visibility from within overlay
  - **Note:** Requires `bwrap` to be installed

- **run-all-tests.sh** - Main test runner that executes all test suites

## Running Tests

### Run All Tests
```bash
./tests/run-all-tests.sh
```

### Run Individual Test Suites
```bash
# System tests
./tests/system-tests.sh

# Overlay tests
./tests/overlay-tests.sh
```

## Test Output Format

Tests use TAP (Test Anything Protocol) format:
- `ok N - description` - Test passed
- `not ok N - description` - Test failed
- `# SKIP reason` - Test skipped
- Comments start with `#`

Example output:
```
TAP version 13
1..21
ok 1 - audit-box --help shows help
ok 2 - audit-box new --help shows help
...
```

## Requirements

- Rust toolchain (for building audit-box)
- bash
- Standard Unix utilities (grep, sed, etc.)
- bwrap (required for overlay-tests.sh only)

## Test Coverage

The test suite covers:
- [x] Help commands for all subcommands
- [x] Session creation with default and custom base paths
- [x] Session file creation and format validation
- [x] Session directory structure (overlay, work subdirectories)
- [x] Review command with explicit paths
- [x] Review command using saved session
- [x] Error handling for missing sessions
- [x] Error handling for corrupted session files
- [x] Error handling for invalid command arguments
- [x] Run command execution
- [x] Run command with flags and arguments
- [x] Overlay filesystem isolation
- [x] File creation in overlay vs base filesystem
- [x] File modification isolation

## Adding New Tests

To add new tests, follow this pattern:

```bash
run_test "test description" "command to run"
run_test_should_fail "test that should fail" "command"
run_test_output_contains "test with output check" "command" "expected string"
```

Remember to:
1. Update the test count at the top: `echo "1..N"`
2. Increment TEST_NUM for each test
3. Add cleanup code if needed
4. Use descriptive test names
