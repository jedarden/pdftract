#!/bin/bash
# Analyze rustdoc coverage for pdftract-core

echo "Analyzing pdftract-core public API documentation coverage..."
echo "================================================================"
echo ""

# Count public items (functions, structs, enums, traits, type aliases, constants)
# Use rustdoc JSON output or simpler: grep for pub fn/pub struct/pub enum/pub trait/pub type/pub const

cd crates/pdftract-core/src

# Count public items
total_pub_items=$(grep -r "^pub " --include="*.rs" | grep -E "pub (fn|struct|enum|trait|type|const|static|mod)" | wc -l)
echo "Total public items found: $total_pub_items"

# Count items with doc comments (/// or //!)
# This is a rough estimate - we'd need a more sophisticated parser for exact counts
echo ""
echo "Note: This is a basic grep-based count. A precise analysis requires:"
echo "1. Rust AST parsing via rust-analyzer or syn crate"
echo "2. Checking for /// doc comments on each public item"
echo "3. Distinguishing between module-level and item-level docs"
echo ""
echo "Key modules to review:"
find . -name "*.rs" -type f | head -20 | while read f; do
    count=$(grep "^pub " "$f" | grep -E "pub (fn|struct|enum|trait|type)" | wc -l)
    if [ "$count" -gt 0 ]; then
        echo "  $f: $count public items"
    fi
done

echo ""
echo "To get precise coverage with examples, run:"
echo "cargo doc -p pdftract-core --no-deps --all-features 2>&1 | grep -i 'missing.*doc'"
