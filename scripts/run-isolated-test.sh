#!/usr/bin/env bash
# run-isolated-test.sh - Run a single test in isolation with orphan detection
#
# This script executes a single test in isolation, captures output to a log,
# and verifies no orphaned processes remain after completion.
#
# Usage:
#   ./scripts/run-isolated-test.sh <test-name> [options]
#
# Arguments:
#   test-name          Name of the test to run (can be full path or function name)
#
# Options:
#   --timeout SECONDS  Timeout for test execution (default: 180)
#   --keep-logs        Keep log files even on successful test (default: delete on success)
#   --verbose          Show detailed progress output
#   --help             Show this help message
#
# Exit codes:
#   0 - Test passed, no orphaned processes
#   1 - Test failed or orphaned processes found
#   2 - Error occurred (invalid args, command failed, etc.)
#   124 - Timeout occurred (from timeout command)

set -euo pipefail

# Default options
TIMEOUT=180
KEEP_LOGS=false
VERBOSE=false
LOG_DIR="logs/isolated-runs"

# Parse arguments
TEST_NAME=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --timeout)
            if [[ -n "${2:-}" ]]; then
                TIMEOUT="$2"
                shift 2
            else
                echo "Error: --timeout requires an argument" >&2
                exit 2
            fi
            ;;
        --keep-logs)
            KEEP_LOGS=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 <test-name> [options]"
            echo ""
            echo "Run a single test in isolation with orphan detection."
            echo ""
            echo "Arguments:"
            echo "  test-name          Name of the test to run"
            echo ""
            echo "Options:"
            echo "  --timeout SECONDS  Timeout for test execution (default: 180)"
            echo "  --keep-logs        Keep log files on success (default: delete)"
            echo "  --verbose          Show detailed progress output"
            echo "  --help             Show this help message"
            echo ""
            echo "Exit codes:"
            echo "  0   - Test passed, no orphaned processes"
            echo "  1   - Test failed or orphaned processes found"
            echo "  2   - Error occurred"
            echo "  124 - Timeout occurred"
            exit 0
            ;;
        -*)
            echo "Error: Unknown option: $1" >&2
            exit 2
            ;;
        *)
            TEST_NAME="$1"
            shift
            ;;
    esac
done

# Validate test name
if [[ -z "$TEST_NAME" ]]; then
    echo "Error: Test name is required" >&2
    echo "Usage: $0 <test-name> [options]" >&2
    exit 2
fi

# Validate timeout is numeric
if ! [[ "$TIMEOUT" =~ ^[0-9]+$ ]]; then
    echo "Error: Timeout must be a positive integer" >&2
    exit 2
fi

# Create log directory
mkdir -p "$LOG_DIR"

# Generate timestamp for log file
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="$LOG_DIR/${TEST_NAME}_${TIMESTAMP}.log"

# Function to run orphan check
run_orphan_check() {
    ./scripts/check-orphaned-processes.sh --json
}

# Pre-check for orphans
if [[ "$VERBOSE" == "true" ]]; then
    echo "Checking for pre-existing orphaned processes..."
    ORPHAN_PRE=$(run_orphan_check)
    ORPHAN_STATUS=$(echo "$ORPHAN_PRE" | grep -o '"status":"[^"]*"' | cut -d: -f2 | tr -d '"')
    if [[ "$ORPHAN_STATUS" == "orphaned" ]]; then
        echo "⚠ Warning: Pre-existing orphaned processes detected"
        echo "$ORPHAN_PRE"
    else
        echo "✓ No pre-existing orphaned processes"
    fi
fi

# Run the test with timeout
if [[ "$VERBOSE" == "true" ]]; then
    echo "Running test: $TEST_NAME"
    echo "Timeout: ${TIMEOUT}s"
    echo "Log file: $LOG_FILE"
fi

# Use cargo nextest to run the test
TEST_EXIT_CODE=0
timeout --kill-after=30s "$TIMEOUT" cargo nextest run "$TEST_NAME" > "$LOG_FILE" 2>&1 || TEST_EXIT_CODE=$?

# Handle timeout exit code specifically
if [[ $TEST_EXIT_CODE -eq 124 ]]; then
    if [[ "$VERBOSE" == "true" ]]; then
        echo "⚠ Test timed out after ${TIMEOUT}s"
    fi
    echo "TIMEOUT" >> "$LOG_FILE"
fi

# Check for orphaned processes after test
if [[ "$VERBOSE" == "true" ]]; then
    echo "Checking for orphaned processes after test..."
fi

ORPHAN_POST=$(run_orphan_check)
ORPHAN_STATUS=$(echo "$ORPHAN_POST" | grep -o '"status":"[^"]*"' | cut -d: -f2 | tr -d '"')

if [[ "$ORPHAN_STATUS" == "orphaned" ]]; then
    if [[ "$VERBOSE" == "true" ]]; then
        echo "✗ Orphaned processes detected after test"
        echo "$ORPHAN_POST"
    fi
    TEST_EXIT_CODE=1  # Treat orphan detection as failure
elif [[ "$VERBOSE" == "true" ]]; then
    echo "✓ No orphaned processes detected"
fi

# Determine overall success
TEST_PASSED=false
if [[ $TEST_EXIT_CODE -eq 0 && "$ORPHAN_STATUS" == "clean" ]]; then
    TEST_PASSED=true
    if [[ "$VERBOSE" == "true" ]]; then
        echo "✅ Test passed"
    fi
else
    if [[ "$VERBOSE" == "true" ]]; then
        echo "❌ Test failed or orphans detected"
    fi
fi

# Clean up log on success if requested
if [[ "$TEST_PASSED" == "true" && "$KEEP_LOGS" == "false" ]]; then
    rm -f "$LOG_FILE"
    if [[ "$VERBOSE" == "true" ]]; then
        echo "Log file removed (test passed)"
    fi
elif [[ "$VERBOSE" == "true" ]]; then
    echo "Log file preserved: $LOG_FILE"
fi

# Exit with appropriate code
if [[ "$TEST_PASSED" == "true" ]]; then
    exit 0
elif [[ $TEST_EXIT_CODE -eq 124 ]]; then
    exit 124
elif [[ $TEST_EXIT_CODE -ne 0 ]]; then
    exit 1
else
    exit 1  # Orphan detected
fi
