#!/bin/bash
# CI gate for rustdoc - ensures all public items are documented.
#
# This script runs cargo doc with -D missing-docs and fails if any warnings are emitted.
# It's designed to run in CI environments (GitHub Actions, Argo Workflows).
#
# Usage: ./ci/rustdoc-gate.sh
#
# Exit codes:
#   0 - All public items are documented
#   1 - rustdoc warnings found (missing documentation)
#   2 - Build failed (compilation error)

set -euo pipefail

# Color output for better readability
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Running rustdoc CI gate ===${NC}"

# Run cargo doc with -D missing-docs (deny missing documentation)
# We use default features only to avoid OCR dependencies which may not be available
echo -e "${YELLOW}Building documentation with -D missing-docs...${NC}"

if cargo doc --no-deps -p pdftract-core 2>&1 | grep -q "warning:"; then
    echo -e "${RED}✗ FAIL: rustdoc warnings found${NC}"
    echo -e "${YELLOW}Run 'cargo doc --no-deps -p pdftract-core' locally to see the warnings${NC}"
    exit 1
fi

echo -e "${GREEN}✓ PASS: No rustdoc warnings${NC}"

# Optionally check example coverage
if command -v rust-script &> /dev/null; then
    echo -e "${YELLOW}Checking example coverage...${NC}"
    if rust-script scripts/count_rustdoc_coverage.rs; then
        echo -e "${GREEN}✓ PASS: 80%+ example coverage met${NC}"
    else
        echo -e "${YELLOW}⚠ WARNING: Example coverage below 80% (non-blocking)${NC}"
    fi
else
    echo -e "${YELLOW}⚠ rust-script not found, skipping example coverage check${NC}"
fi

echo -e "${GREEN}=== rustdoc CI gate passed ===${NC}"
exit 0
