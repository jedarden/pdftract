#!/usr/bin/env bash
#
# lint-ruby-visibility.sh
#
# Static lint check for Ruby visibility-keyword bugs in generated SDK output.
#
# This script detects a bug pattern where a bare "private" or "public" keyword
# appears without proper context, which can accidentally make all methods private
# or public. This was the root cause of bug #2 in the pdftract-ruby audit:
# a stray `private` before helper methods made the entire SDK public API private.
#
# Usage:
#   ./lint-ruby-visibility.sh <ruby-file-or-directory>
#
# Exit codes:
#   0 - No issues found
#   1 - Issues found or usage error
#
# Bead: bf-1uhnlv
# Plan: pdftract-ruby/docs/plan/plan.md ADR-001, Alternative 3

set -euo pipefail

# Color output for better visibility
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if target is provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 <ruby-file-or-directory>" >&2
    exit 1
fi

TARGET="$1"
ISSUES_FOUND=0

echo "=== Ruby Visibility Keyword Lint Check ==="
echo "Target: $TARGET"
echo

# Function to check a single Ruby file
check_file() {
    local file="$1"
    local filename=$(basename "$file")

    # Skip test files and non-source files
    if [[ "$filename" =~ _test\.rb$|test_.*\.rb$|spec\.rb$ ]]; then
        return
    fi

    # Read file content
    local content
    content=$(cat "$file" 2>/dev/null || return 0)

    # Check for the specific bug pattern:
    # A bare "private" or "public" keyword that affects method visibility
    # Pattern 1: "private" keyword not followed by "public" before method definitions
    # Pattern 2: "private" keyword appearing after class definition but before contract methods

    # Check for the specific bug pattern using grep
    # Pattern: "private" keyword without "public" keyword appearing before method definitions

    # Get all visibility keyword lines
    local private_count
    private_count=$(grep -c '^[[:space:]]*private[[:space:]]*$' "$file" 2>/dev/null) || true
    private_count=${private_count:-0}

    local public_count
    public_count=$(grep -c '^[[:space:]]*public[[:space:]]*$' "$file" 2>/dev/null) || true
    public_count=${public_count:-0}

    # Get method definition count (excluding accessor methods like attr_reader)
    local method_count
    method_count=$(grep -c '^[[:space:]]*def[[:space:]]' "$file" 2>/dev/null) || true
    method_count=${method_count:-0}

    # If we have private but no public and we have methods, flag it
    if [ "$private_count" -gt 0 ] && [ "$public_count" -eq 0 ] && [ "$method_count" -gt 0 ]; then
        # Get the first private line for reporting
        local private_line
        private_line=$(grep -n '^[[:space:]]*private[[:space:]]*$' "$file" | head -1 | cut -d: -f1)

        local private_text
        private_text=$(grep '^[[:space:]]*private[[:space:]]*$' "$file" | head -1)

        echo -e "${RED}ISSUE FOUND${NC}: $file"
        echo -e "  Line $private_line: ${YELLOW}${private_text}${NC}"
        echo -e "  → 'private' keyword without corresponding 'public' before $method_count method(s)"
        echo
        ((ISSUES_FOUND++))
        return
    fi
}

# Main logic
if [ -f "$TARGET" ]; then
    # Single file
    check_file "$TARGET"
elif [ -d "$TARGET" ]; then
    # Directory - scan all Ruby files
    while IFS= read -r -d '' file; do
        check_file "$file"
    done < <(find "$TARGET" -type f -name "*.rb" -print0)
else
    echo -e "${RED}ERROR${NC}: Target '$TARGET' is neither a file nor a directory" >&2
    exit 1
fi

# Summary
echo "=== Summary ==="
if [ $ISSUES_FOUND -eq 0 ]; then
    echo -e "${GREEN}✓ No visibility keyword issues found${NC}"
    exit 0
else
    echo -e "${RED}✗ Found $ISSUES_FOUND file(s) with potential visibility keyword bugs${NC}"
    echo
    echo "This indicates a possible bug pattern where 'private' or 'public' keywords"
    echo "are not properly balanced, which can accidentally make the entire API private"
    echo "or public. This was the root cause of bug #2 in the pdftract-ruby audit."
    exit 1
fi
