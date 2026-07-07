#!/usr/bin/env bash
# check-orphaned-processes.sh - Verify no orphaned pdftract/TH-0 processes
#
# This script checks for orphaned processes that may have been left behind
# by test runs. Per CLAUDE.md test hygiene rules, no processes should remain
# after tests complete.
#
# Usage:
#   ./scripts/check-orphaned-processes.sh [options]
#
# Options:
#   --kill              Kill any orphaned processes found
#   --verbose           Show detailed output
#   --json              Output in JSON format
#   --pattern PATTERN   Custom process pattern (default: 'pdftract mcp|TH_0|TH-0')
#
# Exit codes:
#   0 - No orphaned processes found
#   1 - Orphaned processes found (and not killed)
#   2 - Error occurred
#
# Examples:
#   ./scripts/check-orphaned-processes.sh
#   ./scripts/check-orphaned-processes.sh --kill
#   ./scripts/check-orphaned-processes.sh --json | jq .

set -euo pipefail

# Default options
KILL=false
VERBOSE=false
JSON=false
PATTERN='pdftract mcp|TH_0|TH-0'
TIMEOUT=2

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --kill)
            KILL=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --json)
            JSON=true
            shift
            ;;
        --pattern)
            PATTERN="$2"
            shift 2
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --kill              Kill any orphaned processes found"
            echo "  --verbose           Show detailed output"
            echo "  --json              Output in JSON format"
            echo "  --pattern PATTERN   Custom process pattern (default: 'pdftract mcp|TH_0|TH-0')"
            echo "  --timeout SECONDS   Timeout for process search (default: 2)"
            echo ""
            echo "Exit codes:"
            echo "  0 - No orphaned processes found"
            echo "  1 - Orphaned processes found (and not killed)"
            echo "  2 - Error occurred"
            exit 0
            ;;
        *)
            echo "Error: Unknown option: $1" >&2
            echo "Run '$0 --help' for usage." >&2
            exit 2
            ;;
    esac
done

# Validate timeout is a positive integer
if ! [[ "$TIMEOUT" =~ ^[0-9]+$ ]]; then
    echo "Error: --timeout must be a positive integer" >&2
    exit 2
fi

# Function to find orphaned processes
find_orphaned_processes() {
    local pattern="$1"
    local current_pid=$$
    # Get PIDs matching pattern, excluding current script
    local pids
    pids=$(timeout "$TIMEOUT" pgrep -f "$pattern" 2>/dev/null | grep -v "^$current_pid$" || true)

    # If no PIDs found, return early
    if [[ -z "$pids" ]]; then
        return
    fi

    # Get full command line for each PID
    while IFS= read -r pid; do
        if [[ -n "$pid" ]]; then
            # Get full command line (suppress errors for processes that exit)
            local cmdline=""
            if [[ "$OSTYPE" == "linux-gnu"* ]]; then
                if [[ -r "/proc/$pid/cmdline" ]]; then
                    cmdline=$(tr '\0' ' ' < "/proc/$pid/cmdline")
                fi
            else
                cmdline=$(ps -p "$pid" -o command= 2>/dev/null) || true
            fi
            if [[ -n "$cmdline" ]]; then
                echo "$pid $cmdline"
            fi
        fi
    done <<< "$pids"
}

# Function to extract process details
extract_process_details() {
    local output="$1"
    local pids=()
    local commands=()

    while IFS= read -r line; do
        if [[ -n "$line" ]]; then
            local pid=$(echo "$line" | awk '{print $1}')
            local cmdline=$(echo "$line" | cut -d' ' -f2-)
            pids+=("$pid")
            commands+=("$cmdline")
        fi
    done <<< "$output"

    # Export as variables for JSON output
    FOUND_PIDS=("${pids[@]}")
    FOUND_COMMANDS=("${commands[@]}")
    FOUND_COUNT=${#pids[@]}
}

# Main logic
main() {
    local orphaned_output
    orphaned_output=$(find_orphaned_processes "$PATTERN")

    if [[ -z "$orphaned_output" ]]; then
        if [[ "$JSON" == "true" ]]; then
            echo '{"status": "clean", "orphaned_processes": [], "count": 0}'
        else
            if [[ "$VERBOSE" == "true" ]]; then
                echo "✓ No orphaned processes found matching pattern: $PATTERN"
            else
                echo "clean"
            fi
        fi
        exit 0
    fi

    # Found orphaned processes
    extract_process_details "$orphaned_output"

    if [[ "$JSON" == "true" ]]; then
        # Build JSON output
        local json_output='{"status": "orphaned", "orphaned_processes": ['
        local first=true
        for i in $(seq 0 $((FOUND_COUNT - 1))); do
            if [[ "$first" == "true" ]]; then
                first=false
            else
                json_output+=','
            fi
            json_output+="{\"pid\": \"${FOUND_PIDS[$i]}\", \"command\": \"$(echo "${FOUND_COMMANDS[$i]}" | sed 's/"/\\"/g')\"}"
        done
        json_output+='], "count": '"$FOUND_COUNT"

        if [[ "$KILL" == "true" ]]; then
            json_output+=', "action": "killed"'
        else
            json_output+=', "action": "detected"'
        fi
        json_output+='}'

        echo "$json_output"
    else
        if [[ "$VERBOSE" == "true" ]]; then
            echo "⚠ Found $FOUND_COUNT orphaned process(es) matching pattern: $PATTERN"
            echo ""
            for i in $(seq 0 $((FOUND_COUNT - 1))); do
                echo "  PID: ${FOUND_PIDS[$i]}"
                echo "  Command: ${FOUND_COMMANDS[$i]}"
                echo ""
            done
        else
            echo "orphaned ($FOUND_COUNT process(es))"
        fi
    fi

    # Kill processes if requested
    if [[ "$KILL" == "true" ]]; then
        if [[ "$VERBOSE" == "true" && "$JSON" == "false" ]]; then
            echo "Killing orphaned processes..."
        fi

        for pid in "${FOUND_PIDS[@]}"; do
            if kill "$pid" 2>/dev/null; then
                if [[ "$VERBOSE" == "true" && "$JSON" == "false" ]]; then
                    echo "  ✓ Killed PID $pid"
                fi
            else
                if [[ "$VERBOSE" == "true" && "$JSON" == "false" ]]; then
                    echo "  ✗ Failed to kill PID $pid"
                fi
            fi
        done

        # Wait briefly and verify cleanup
        sleep 1
        orphaned_output=$(find_orphaned_processes "$PATTERN")
        if [[ -z "$orphaned_output" ]]; then
            if [[ "$VERBOSE" == "true" && "$JSON" == "false" ]]; then
                echo "✓ All orphaned processes cleaned up successfully"
            fi
            if [[ "$JSON" == "true" ]]; then
                echo '{"status": "cleaned", "orphaned_processes": [], "count": 0}'
                exit 0
            else
                echo "cleaned"
                exit 0
            fi
        else
            if [[ "$VERBOSE" == "true" && "$JSON" == "false" ]]; then
                echo "⚠ Warning: Some processes could not be killed"
            fi
            if [[ "$JSON" == "true" ]]; then
                echo '{"status": "partial_cleanup", "message": "Some processes could not be killed"}'
                exit 1
            else
                echo "partial_cleanup"
                exit 1
            fi
        fi
    fi

    # Exit with error if we found orphans and didn't kill them
    exit 1
}

main
