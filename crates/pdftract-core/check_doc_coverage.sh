#!/usr/bin/env bash
# Measure rustdoc coverage for pdftract-core public API
# Counts: total public items, items with doc comments, items with examples

set -e

CRATE_PATH="crates/pdftract-core/src"

echo "=== pdftract-core Rustdoc Coverage Analysis ==="
echo

# Count all public items (pub fn, pub struct, pub enum, pub trait, pub type, pub mod)
echo "Counting public items..."
TOTAL_ITEMS=$(grep -r "pub fn\|pub struct\|pub enum\|pub trait\|pub type\|pub mod" "$CRATE_PATH" --include="*.rs" | grep -v "pub(crate)" | grep -v "pub use" | wc -l)
echo "Total public items: $TOTAL_ITEMS"

# Count items with doc comments (/// or //!)
echo "Counting items with documentation..."
DOC_ITEMS=$(grep -r "///\|//!" "$CRATE_PATH" --include="*.rs" -A 1 | grep -r "pub fn\|pub struct\|pub enum\|pub trait\|pub type\|pub mod" | grep -v "pub(crate)" | wc -l)
echo "Items with documentation: $DOC_ITEMS"

# Count items with examples (```rust blocks)
echo "Counting items with worked examples..."
EXAMPLE_ITEMS=$(grep -r "///.*\|//!" "$CRATE_PATH" --include="*.rs" -A 5 | grep -r "```rust" | wc -l)
echo "Items with examples: $EXAMPLE_ITEMS"

# Calculate coverage percentages
if [ "$TOTAL_ITEMS" -gt 0 ]; then
    DOC_COVERAGE=$(awk "BEGIN {printf \"%.1f\", ($DOC_ITEMS / $TOTAL_ITEMS) * 100}")
    EXAMPLE_COVERAGE=$(awk "BEGIN {printf \"%.1f\", ($EXAMPLE_ITEMS / $TOTAL_ITEMS) * 100}")
else
    DOC_COVERAGE=0
    EXAMPLE_COVERAGE=0
fi

echo
echo "=== Coverage Summary ==="
echo "Documentation coverage: $DOC_COVERAGE% ($DOC_ITEMS/$TOTAL_ITEMS items)"
echo "Example coverage: $EXAMPLE_COVERAGE% ($EXAMPLE_ITEMS/$TOTAL_ITEMS items)"
echo

# Check if we meet the 80% threshold
if (( $(echo "$EXAMPLE_COVERAGE >= 80.0" | bc -l) )); then
    echo "✓ Meets 80% worked-example threshold"
else
    echo "✗ Below 80% worked-example threshold (need 80%, have $EXAMPLE_COVERAGE%)"
fi

# List items missing documentation
echo
echo "=== Items missing documentation ==="
grep -rn "pub fn\|pub struct\|pub enum\|pub trait\|pub type" "$CRATE_PATH" --include="*.rs" | while IFS=: read -r line_num file line; do
    # Check if the line before has a doc comment
    prev_line=$(sed -n "$((line_num - 1))p" "$file")
    if [[ ! "$prev_line" =~ "///" && ! "$prev_line" =~ "///" && ! "$line" =~ "pub(crate)" && ! "$line" =~ "pub use" ]]; then
        # Check if it's a type alias (skip those)
        if [[ "$line" =~ "pub type" ]]; then
            echo "$file:$line_num: $line"
        else
            echo "$file:$line_num: $line"
        fi
    fi
done | head -20
