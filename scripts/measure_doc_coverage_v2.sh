#!/bin/bash
# Comprehensive rustdoc coverage measurement for pdftract-core

set -o pipefail

CORE_CRATE="/home/coding/pdftract/crates/pdftract-core"
cd "$CORE_CRATE"

echo "=== pdftract-core rustdoc coverage measurement ==="
echo ""

# Run cargo doc with the docs.rs feature set
echo "Running cargo doc with docs.rs feature set..."
if cargo doc --no-deps --features "serde,schemars,receipts,remote,profiles,decrypt,cjk,quick-xml" 2>&1 | tee /tmp/doc_output.txt; then
    DOCS_OK=true
else
    DOCS_OK=false
fi

echo ""
echo "=== Warnings Analysis ==="
WARNINGS=$(grep -c "warning:" /tmp/doc_output.txt 2>/dev/null || echo "0")
echo "Total warnings: $WARNINGS"

# Check for missing_docs warnings specifically
MISSING_DOCS=$(grep -c "missing documentation" /tmp/doc_output.txt 2>/dev/null || echo "0")
echo "Missing doc warnings: $MISSING_DOCS"

# Check for broken links
BROKEN_LINKS=$(grep -c "cannot be resolved" /tmp/doc_output.txt 2>/dev/null || echo "0")
echo "Unresolved links: $BROKEN_LINKS"

echo ""
echo "=== Public API Coverage ==="

# Count public items
TOTAL_PUBLIC=$(find src -name "*.rs" -exec grep -h "^pub " {} \; | wc -l)
echo "Total public items: $TOTAL_PUBLIC"

# Count items with rustdoc comments
WITH_DOC=$(find src -name "*.rs" -exec grep -B1 "^pub " {} \; | grep "///" | wc -l)
echo "Items with /// docs: $WITH_DOC"

# Count items with examples (check for ```rust or similar code blocks in rustdoc)
FILES_WITH_EXAMPLES=$(find src -name "*.rs" -exec grep -l '```rust' {} \; | wc -l)
echo "Files with rust examples: $FILES_WITH_EXAMPLES"

# Count example blocks
EXAMPLE_BLOCKS=$(find src -name "*.rs" -exec grep -c '```rust' {} \; | awk '{s+=$1} END {print s}')
echo "Total example blocks: $EXAMPLE_BLOCKS"

echo ""
echo "=== Coverage Calculation ==="
if [ "$TOTAL_PUBLIC" -gt 0 ]; then
    DOC_COVERAGE=$((WITH_DOC * 100 / TOTAL_PUBLIC))
    echo "Basic doc coverage: $DOC_COVERAGE% ($WITH_DOC / $TOTAL_PUBLIC)"

    if [ "$EXAMPLE_BLOCKS" -gt 0 ]; then
        # Estimate example coverage
        # Each file with examples likely documents multiple items
        EST_EXAMPLE_COV=$((EXAMPLE_BLOCKS * 100 / TOTAL_PUBLIC))
        echo "Estimated example coverage: $EST_EXAMPLE_COV% (based on example block count)"
    fi
fi

echo ""
echo "=== Modules needing module-level docs ==="
for mod_file in $(find src -name "mod.rs" | sort); do
    mod_name=$(echo "$mod_file" | sed 's|src/||' | sed 's|/mod.rs||' | tr '/' '.')
    if ! grep -q "^//! " "$mod_file" 2>/dev/null; then
        echo "- $mod_name (no module doc)"
    fi
done

echo ""
echo "=== Files without any rustdoc ==="
for file in $(find src -name "*.rs" ! -name "test*.rs" | sort); do
    if [ -f "$file" ] && ! grep -q "///" "$file" && ! grep -q "^//!" "$file"; then
        echo "- $file"
    fi
done

echo ""
echo "=== Summary ==="
if [ "$DOCS_OK" = true ] && [ "$WARNINGS" -eq 0 ]; then
    echo "cargo doc: PASS (no warnings)"
else
    echo "cargo doc: FAIL"
fi

if [ "$DOC_COVERAGE" -ge 80 ]; then
    echo "Basic coverage: PASS (≥80%)"
else
    echo "Basic coverage: FAIL (<80%)"
fi

echo ""
echo "Full output saved to /tmp/doc_output.txt"
