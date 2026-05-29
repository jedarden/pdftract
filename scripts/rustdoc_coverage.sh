#!/usr/bin/env bash
# Measure rustdoc coverage for pdftract-core public API.
# Reports:
# - Total public items
# - Items with doc comments
# - Items with worked examples (```rust blocks)
# - Coverage percentage

cd "$(dirname "$0")/.."

echo "=== pdftract-core rustdoc coverage ===" >&2
echo "" >&2

# Count public items (count lines, not files)
total=$(find crates/pdftract-core/src -name "*.rs" -exec grep -H "^pub " {} \; | wc -l)
echo "Total public items: $total" >&2

# Count items with doc comments (/// or //!) preceding pub items
with_docs=$(find crates/pdftract-core/src -name "*.rs" -exec grep -B2 "^pub " {} \; 2>/dev/null | grep -c "///\|//!" || echo "0")
echo "Items with doc comments: $with_docs" >&2

# Count items with worked examples (```rust blocks in doc comments)
with_examples=$(grep -r '```rust' crates/pdftract-core/src --include="*.rs" 2>/dev/null | wc -l || echo "0")
echo "Items with worked examples: $with_examples" >&2

# Calculate coverage
if [ "$total" -gt 0 ]; then
    doc_coverage=$((with_docs * 100 / total))
    example_coverage=$((with_examples * 100 / total))
else
    doc_coverage=0
    example_coverage=0
fi

echo "" >&2
echo "=== Coverage ===" >&2
echo "Doc comments: $doc_coverage%" >&2
echo "Worked examples: $example_coverage%" >&2
echo "" >&2

# JSON output for parsing
echo "{\"total\":$total,\"with_docs\":$with_docs,\"with_examples\":$with_examples,\"doc_coverage\":$doc_coverage,\"example_coverage\":$example_coverage}"
