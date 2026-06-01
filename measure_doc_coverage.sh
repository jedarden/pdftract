#!/usr/bin/env bash
# Measure documentation example coverage for pdftract-core

cd /home/coding/pdftract/crates/pdftract-core

# Count public items and those with examples
total_items=0
items_with_examples=0

# Find all .rs files in src/
find src -name "*.rs" -type f | while read -r file; do
    # Extract public items (pub fn, pub struct, pub enum, pub trait, pub type, pub mod)
    # and check if they have doc comments with examples

    # We'll use a simple grep-based approach to find pub items
    # and check preceding lines for ```rust examples

    grep -n "^pub " "$file" | while IFS=: read -r line_num _; do
        ((total_items++))

        # Look back up to 50 lines for ```rust example
        if sed -n "$((line_num - 50)),${line_num}p" "$file" | grep -q '```rust'; then
            ((items_with_examples++))
        fi
    done
done

echo "Total items with examples: $items_with_examples / $total_items"
if [ "$total_items" -gt 0 ]; then
    coverage=$(echo "scale=1; 100 * $items_with_examples / $total_items" | bc)
    echo "Coverage: $coverage%"
fi
