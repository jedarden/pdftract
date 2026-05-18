#!/bin/bash
# CI check for unauthorized expose_secret() call sites.
#
# Per pdftract-5l9m, the only legitimate uses of expose_secret() are:
# - SecretFingerprint::from_secret() (crates/pdftract-core/src/parser/secrets.rs)
# - Test code (crates/pdftract-core/src/parser/stream.rs deserialization test)
#
# This script should be run in CI to catch any new unauthorized uses.

set -e

echo "Checking for unauthorized expose_secret() call sites..."

# Find all expose_secret() calls
RESULTS=$(rg "expose_secret\(\)" crates/ --type rust -n || true)

if [ -z "$RESULTS" ]; then
    echo "✓ No expose_secret() calls found"
    exit 0
fi

# Check for unauthorized calls
# Authorized locations:
# 1. crates/pdftract-core/src/parser/secrets.rs:37 - SecretFingerprint::from_secret()
# 2. crates/pdftract-core/src/parser/stream.rs:2161 - test deserialization

UNAUTHORIZED=""
while IFS= read -r line; do
    # Extract file and line number
    FILE_LINE=$(echo "$line" | cut -d: -f1-2)

    # Check if this is an authorized location
    if [[ "$FILE_LINE" == *"secrets.rs:37"* ]]; then
        continue
    fi
    if [[ "$FILE_LINE" == *"stream.rs:2161"* ]]; then
        continue
    fi
    # Skip comment lines (contain "//!")
    if [[ "$line" == *"//!"* ]]; then
        continue
    fi
    UNAUTHORIZED="$UNAUTHORIZED$line"$'\n'
done <<< "$RESULTS"

if [ -n "$UNAUTHORIZED" ]; then
    echo "❌ Found unauthorized expose_secret() call sites:"
    echo "$UNAUTHORIZED"
    echo ""
    echo "The only authorized uses of expose_secret() are:"
    echo "  - crates/pdftract-core/src/parser/secrets.rs:SecretFingerprint::from_secret()"
    echo "  - crates/pdftract-core/src/parser/stream.rs:2161 (test code)"
    exit 1
fi

echo "✓ All expose_secret() calls are authorized"
