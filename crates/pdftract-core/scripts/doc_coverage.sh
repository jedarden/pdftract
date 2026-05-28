#!/bin/bash
# Analyze pdftract-core public API documentation coverage.

set -e

cd "$(dirname "$0")/.."

echo "=== pdftract-core Public API Documentation Coverage ==="
echo ""

# Run cargo doc with missing_docs enabled
echo "Running cargo doc to check for missing_docs warnings..."

# First, check if missing_docs is already enabled
if grep -q "#!\[deny(missing_docs)\]" src/lib.rs; then
    echo "missing_docs already enabled"
else
    echo "Enabling missing_docs lint temporarily..."
    cp src/lib.rs src/lib.rs.bak
    sed -i '1i #![deny(missing_docs)]' src/lib.rs
    trap "mv src/lib.rs.bak src/lib.rs" EXIT
fi

# Run cargo doc and capture warnings
OUTPUT=$(cargo doc --no-deps 2>&1 || true)

# Count missing_docs warnings
MISSING=$(echo "$OUTPUT" | grep -c "missing_docs" || echo 0)
echo "Public items missing documentation: $MISSING"

# Get documented count from cargo doc output
DOCUMENTED=$(echo "$OUTPUT" | grep -oP "documented \K[0-9]+" || echo 0)
echo "Total public items documented: $DOCUMENTED"

# Calculate total items
TOTAL=$((DOCUMENTED + MISSING))
COVERAGE=0
if [ "$TOTAL" -gt 0 ]; then
    COVERAGE=$((DOCUMENTED * 100 / TOTAL))
fi

echo ""
echo "=== Coverage Status ==="
echo "Total public items: $TOTAL"
echo "Coverage: ${COVERAGE}%"

if [ "$COVERAGE" -ge 80 ]; then
    echo "✓ PASS: ${COVERAGE}% coverage meets 80% threshold"
    exit 0
else
    echo "✗ FAIL: ${COVERAGE}% coverage below 80% threshold"
    exit 1
fi
