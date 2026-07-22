#!/usr/bin/env bash
# check-orphaned-processes.sh - Check for orphaned test processes
#
# This script verifies that no orphaned processes remain after test runs.
# It checks for processes matching known patterns (pdftract mcp, TH-0, TH_0).
#
# Usage:
#   ./scripts/check-orphaned-processes.sh [options]
#
# Options:
#   --json              Output in JSON format for CI parsing
#   --kill              Kill orphaned processes (default: false)
#   --verbose           Show detailed output
#   --pattern PATTERN   Custom process pattern to check (can be repeated)
#
# Exit codes:
#   0 - No orphaned processes found (clean state)
#   1 - Orphaned processes found (and not killed)
#   2 - Error occurred (invalid args, command failed, etc.)

set -euo pipefail

# Default process patterns to check
DEFAULT_PATTERNS=("pdftract mcp" "TH-0" "TH_0")

# Options
JSON_OUTPUT=false
KILL_ORPHANS=false
VERBOSE=false
PATTERNS=("${DEFAULT_PATTERNS[@]}")

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --json)
            JSON_OUTPUT=true
            shift
            ;;
        --kill)
            KILL_ORPHANS=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --pattern)
            if [[ -n "${2:-}" ]]; then
                PATTERNS+=("$2")
                shift 2
            else
                echo "Error: --pattern requires an argument" >&2
                exit 2
            fi
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Check for orphaned test processes."
            echo ""
            echo "Options:"
            echo "  --json              Output in JSON format"
            echo "  --kill              Kill orphaned processes"
            echo "  --verbose           Show detailed output"
            echo "  --pattern PATTERN   Custom process pattern (repeatable)"
            echo ""
            echo "Default patterns: ${DEFAULT_PATTERNS[*]}"
            echo ""
            echo "Exit codes:"
            echo "  0 - No orphaned processes (clean)"
            echo "  1 - Orphaned processes found"
            echo "  2 - Error occurred"
            exit 0
            ;;
        *)
            echo "Error: Unknown option: $1" >&2
            exit 2
            ;;
    esac
done

# Check if pgrep is available
if ! command -v pgrep &> /dev/null; then
    if [[ "$JSON_OUTPUT" == "true" ]]; then
        echo '{"status":"error","error":"pgrep command not found"}'
    else
        echo "Error: pgrep command not found. Please install procps (Debian/Ubuntu) or procps-ng (RHEL/Fedora)." >&2
    fi
    exit 2
fi

# Find processes matching a pattern
# Args:
#   $1 - pattern to search for
# Outputs:
#   List of PIDs (one per line) for matching processes
find_processes() {
    local pattern="$1"
    pgrep -f "$pattern" 2>/dev/null || true
}

# Get process command line
# Args:
#   $1 - PID
# Outputs:
#   Process command line
get_process_command() {
    local pid="$1"
    if command -v ps &> /dev/null; then
        ps -p "$pid" -o args= 2>/dev/null || echo "(unknown)"
    else
        echo "(unknown - ps command not available)"
    fi
}

# Get current script's PID for filtering
SCRIPT_PID=$$
SCRIPT_PARENT_PID=$PPID

# Collect all orphaned processes
declare -a ORPHAN_PIDS=()
declare -A ORPHAN_CMDS=()

for pattern in "${PATTERNS[@]}"; do
    while IFS= read -r pid; do
        if [[ -n "$pid" ]]; then
            # Skip the script's own PID and its parent
            if [[ "$pid" == "$SCRIPT_PID" ]] || [[ "$pid" == "$SCRIPT_PARENT_PID" ]]; then
                continue
            fi
            # Also skip any parent processes in the chain (up to grandparent)
            ppid=$(ps -o ppid= -p "$pid" 2>/dev/null | tr -d ' ')
            if [[ -n "$ppid" ]] && [[ "$ppid" != "0" ]]; then
                if [[ "$ppid" == "$SCRIPT_PID" ]] || [[ "$ppid" == "$SCRIPT_PARENT_PID" ]]; then
                    continue
                fi
            fi
            ORPHAN_PIDS+=("$pid")
            ORPHAN_CMDS["$pid"]="$(get_process_command "$pid")"
        fi
    done < <(find_processes "$pattern")
done

ORPHAN_COUNT=${#ORPHAN_PIDS[@]}

# Output results
if [[ "$JSON_OUTPUT" == "true" ]]; then
    if [[ $ORPHAN_COUNT -eq 0 ]]; then
        echo '{"status":"clean","orphaned_processes":[],"count":0}'
    else
        echo -n '{"status":"orphaned","orphaned_processes":['
        first=true
        for pid in "${ORPHAN_PIDS[@]}"; do
            if [[ "$first" == "true" ]]; then
                first=false
            else
                echo -n ','
            fi
            # Escape special characters in command for JSON
            cmd="${ORPHAN_CMDS[$pid]}"
            cmd="${cmd//\\/\\\\}"  # Escape backslashes
            cmd="${cmd//\"/\\\"}"  # Escape quotes
            printf '{"pid":"%s","command":"%s"}' "$pid" "$cmd"
        done
        echo -n "],"count":$ORPHAN_COUNT}"
    fi
else
    if [[ $ORPHAN_COUNT -eq 0 ]]; then
        if [[ "$VERBOSE" == "true" ]]; then
            echo "✓ No orphaned processes found"
        fi
    else
        echo "✗ Found $ORPHAN_COUNT orphaned process(es):"
        for pid in "${ORPHAN_PIDS[@]}"; do
            echo "  PID $pid: ${ORPHAN_CMDS[$pid]}"
        done
    fi
fi

# Kill orphans if requested
if [[ "$KILL_ORPHANS" == "true" && $ORPHAN_COUNT -gt 0 ]]; then
    if [[ "$VERBOSE" == "true" ]]; then
        echo "Killing $ORPHAN_COUNT orphaned process(es)..."
    fi

    killed=0
    for pid in "${ORPHAN_PIDS[@]}"; do
        if kill "$pid" 2>/dev/null; then
            ((killed++)) || true
            if [[ "$VERBOSE" == "true" ]]; then
                echo "  Killed PID $pid"
            fi
        fi
    done

    if [[ "$JSON_OUTPUT" == "true" ]]; then
        # Output final status after kill attempt
        if [[ $killed -eq $ORPHAN_COUNT ]]; then
            echo '{"status":"cleaned","orphaned_processes":[],"count":0}'
        else
            echo "{\"status\":\"partial_cleanup\",\"killed\":$killed,\"remaining\":$((ORPHAN_COUNT - killed))}"
        fi
    else
        echo "Killed $killed/$ORPHAN_COUNT processes"
    fi
fi

# Exit codes
if [[ $ORPHAN_COUNT -eq 0 ]]; then
    exit 0
elif [[ "$KILL_ORPHANS" == "true" ]]; then
    # Killed orphans, treat as success (but may have partial cleanup)
    exit 0
else
    exit 1
fi
