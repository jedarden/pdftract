#!/usr/bin/env bash
# Validate PROVENANCE.md against actual fixture files.
# Ensures every fixture has a provenance entry with matching SHA256.

set -e

FIXTURES_DIR="tests/fixtures"
PROVENANCE_FILE="$FIXTURES_DIR/profiles/PROVENANCE.md"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "Checking fixture provenance..."

# Check if PROVENANCE.md exists
if [[ ! -f "$PROVENANCE_FILE" ]]; then
    echo -e "${RED}ERROR: $PROVENANCE_FILE not found${NC}"
    exit 1
fi

# Find all fixture files (exclude grep-corpus which uses its own manifest.csv)
FIXTURE_COUNT=$(find "$FIXTURES_DIR" -type f \( -name "*.pdf" -o -name "*.yml" -o -name "*.yaml" \) ! -name "PROVENANCE.md" ! -path "*/grep-corpus/*" | wc -l)
echo "Found $FIXTURE_COUNT fixture files"

# Track errors and warnings in temp files for subprocess safety
ERROR_FILE=$(mktemp)
WARN_FILE=$(mktemp)

echo "Validating provenance entries..."

validated=0

# Parse PROVENANCE.md table and validate each entry
while IFS= read -r line; do
    # Skip separator row
    [[ "$line" =~ ^\|\- ]] && continue

    # Remove leading/trailing | and parse fields
    row="${line#\|}"
    row="${row%\|}"

    # Split by | and trim whitespace
    path=$(echo "$row" | cut -d'|' -f1 | xargs)
    sha256=$(echo "$row" | cut -d'|' -f5 | xargs)
    license=$(echo "$row" | cut -d'|' -f3 | xargs)

    # Skip header row and empty paths
    [[ "$path" == "Path" ]] && continue
    [[ -z "$path" ]] && continue

    FULL_PATH="$FIXTURES_DIR/$path"

    # Check if file exists
    if [[ ! -f "$FULL_PATH" ]]; then
        echo "ERROR: Provenance entry references non-existent file: $path" >> "$ERROR_FILE"
        continue
    fi

    # Compute actual SHA256
    ACTUAL_SHA256=$(sha256sum "$FULL_PATH" | cut -d' ' -f1)

    if [[ "$ACTUAL_SHA256" != "$sha256" ]]; then
        echo "ERROR: SHA256 mismatch for $path" >> "$ERROR_FILE"
        echo "  Expected: $sha256" >> "$ERROR_FILE"
        echo "  Actual:   $ACTUAL_SHA256" >> "$ERROR_FILE"
    fi

    ((validated++)) || true
    if [[ $((validated % 50)) -eq 0 ]]; then
        echo -e "${GREEN}✓${NC} Validated $validated entries..."
    fi

    # Validate license is from approved list
    APPROVED_LICENSES="public-domain|CC0-1.0|CC-BY-3.0|CC-BY-4.0|CC-BY-SA-3.0|CC-BY-SA-4.0|US-government|Apache-2.0|MIT|MIT-0"
    if [[ ! "$license" =~ ^($APPROVED_LICENSES)$ ]]; then
        echo "WARN: Unapproved license '$license' for $path" >> "$WARN_FILE"
    fi
done < <(grep -E "^\|" "$PROVENANCE_FILE")

# Check for orphaned files (files without provenance entries)
# Exclude grep-corpus which uses its own manifest.csv system
echo "Checking for orphaned fixture files..."
while read fixture_file; do
    REL_PATH="${fixture_file#$FIXTURES_DIR/}"
    if ! grep -q "| $REL_PATH " "$PROVENANCE_FILE"; then
        echo "ERROR: Fixture file missing from PROVENANCE.md: $REL_PATH" >> "$ERROR_FILE"
    fi
done < <(find "$FIXTURES_DIR" -type f \( -name "*.pdf" -o -name "*.yml" -o -name "*.yaml" \) ! -name "PROVENANCE.md" ! -path "*/grep-corpus/*")

# Count errors and warnings
ERRORS=$(wc -l < "$ERROR_FILE" 2>/dev/null || echo 0)
WARNINGS=$(wc -l < "$WARN_FILE" 2>/dev/null || echo 0)

# Display any errors
if [[ $ERRORS -gt 0 ]]; then
    cat "$ERROR_FILE"
fi

# Display any warnings
if [[ $WARNINGS -gt 0 ]]; then
    cat "$WARN_FILE"
fi

# Clean up temp files
rm -f "$ERROR_FILE" "$WARN_FILE"

# Summary
echo ""
if [[ $ERRORS -eq 0 ]]; then
    echo -e "${GREEN}✓ All fixtures have valid provenance entries${NC}"
    if [[ $WARNINGS -gt 0 ]]; then
        echo -e "${YELLOW}⚠ $WARNINGS warning(s)${NC}"
    fi
    exit 0
else
    echo -e "${RED}✗ Found $ERRORS error(s) in provenance validation${NC}"
    exit 1
fi
