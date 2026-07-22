#!/usr/bin/env bash
# post-test-check.sh - Post-test verification for CI
#
# This script runs verification checks after test completion to ensure
# no orphaned processes remain. Intended for CI workflow integration.
#
# Usage:
#   ./.ci/scripts/post-test-check.sh [options]
#
# Options:
#   --kill              Kill orphaned processes (default: true)
#   --json              Output in JSON format for CI parsing
#   --strict            Fail if any orphans found (even if killed)
#
# Exit codes:
#   0 - All checks passed
#   1 - Orphaned processes found (with --strict)
#   2 - Error occurred

set -euo pipefail

# Default options
KILL_ORPHANS=true
JSON_OUTPUT=false
STRICT_MODE=false
FAILED=0

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --kill)
            KILL_ORPHANS=true
            shift
            ;;
        --no-kill)
            KILL_ORPHANS=false
            shift
            ;;
        --json)
            JSON_OUTPUT=true
            shift
            ;;
        --strict)
            STRICT_MODE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Post-test verification for CI."
            echo ""
            echo "Options:"
            echo "  --kill              Kill orphaned processes (default: true)"
            echo "  --no-kill           Don't kill orphaned processes (detection only)"
            echo "  --json              Output in JSON format for CI parsing"
            echo "  --strict            Fail if any orphans found (even if killed)"
            echo ""
            echo "Exit codes:"
            echo "  0 - All checks passed"
            echo "  1 - Orphaned processes found (with --strict)"
            echo "  2 - Error occurred"
            exit 0
            ;;
        *)
            echo "Error: Unknown option: $1" >&2
            exit 2
            ;;
    esac
done

# Main verification function
run_verification() {
    local check_args=()

    if [[ "$KILL_ORPHANS" == "true" ]]; then
        check_args+=(--kill)
    fi

    if [[ "$JSON_OUTPUT" == "true" ]]; then
        check_args+=(--json)
    fi

    local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
    local check_script="$script_dir/scripts/check-orphaned-processes.sh"

    if [[ ! -f "$check_script" ]]; then
        echo "Error: Verification script not found: $check_script" >&2
        exit 2
    fi

    # Run the check
    "$check_script" "${check_args[@]}"
    local exit_code=$?

    if [[ $exit_code -eq 0 ]]; then
        # Clean state
        if [[ "$JSON_OUTPUT" == "false" ]]; then
            echo "✓ Post-test verification passed: No orphaned processes"
        fi
        return 0
    elif [[ $exit_code -eq 1 ]]; then
        # Orphans found (may have been killed)
        if [[ "$JSON_OUTPUT" == "false" ]]; then
            if [[ "$STRICT_MODE" == "true" ]]; then
                echo "✗ Post-test verification failed: Orphaned processes detected"
            else
                echo "⚠ Post-test verification warning: Orphaned processes detected and cleaned"
            fi
        fi

        if [[ "$STRICT_MODE" == "true" ]]; then
            return 1
        else
            return 0
        fi
    else
        # Error occurred
        if [[ "$JSON_OUTPUT" == "false" ]]; then
            echo "✗ Post-test verification error: Check script failed (exit code $exit_code)"
        fi
        return 2
    fi
}

# Run verification
run_verification
exit $?
