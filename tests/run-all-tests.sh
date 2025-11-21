#!/bin/bash
# Run all audit-box system tests

set -e

echo "========================================="
echo "Running audit-box system tests"
echo "========================================="
echo ""

# Track overall test status
ALL_PASSED=0

# Run system tests
echo "Running system tests..."
if ./tests/system-tests.sh; then
    echo "✓ System tests passed"
else
    echo "✗ System tests failed"
    ALL_PASSED=1
fi

echo ""

# Run overlay tests
echo "Running overlay isolation tests..."
if ./tests/overlay-tests.sh; then
    echo "✓ Overlay tests passed"
else
    echo "✗ Overlay tests failed"
    ALL_PASSED=1
fi

echo ""
echo "========================================="
if [ $ALL_PASSED -eq 0 ]; then
    echo "All tests passed!"
    exit 0
else
    echo "Some tests failed"
    exit 1
fi
