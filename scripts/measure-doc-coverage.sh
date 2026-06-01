#!/bin/bash
# Measure rustdoc coverage for pdftract-core
# Counts public items vs. items with worked examples

set -euo pipefail

echo "=== PDFTRACT-CORE DOC COVERAGE MEASUREMENT ==="
echo ""

# Change to project root to ensure correct paths
cd "$(dirname "$0")/.."

# Find all .rs files in pdftract-core
FILES=$(find crates/pdftract-core/src -name '*.rs' 2>/dev/null | wc -l)
echo "Scanning $FILES Rust files..."
echo ""

# Count public items (pub fn, pub struct, pub enum, pub trait, pub type)
# Using ripgrep to match these patterns
PUBLIC_FN=$(rg '^pub fn ' crates/pdftract-core/src --type rust -c | awk -F: '{s+=$2} END {print s+0}')
PUBLIC_STRUCT=$(rg '^pub struct ' crates/pdftract-core/src --type rust -c | awk -F: '{s+=$2} END {print s+0}')
PUBLIC_ENUM=$(rg '^pub enum ' crates/pdftract-core/src --type rust -c | awk -F: '{s+=$2} END {print s+0}')
PUBLIC_TRAIT=$(rg '^pub trait ' crates/pdftract-core/src --type rust -c | awk -F: '{s+=$2} END {print s+0}')
PUBLIC_TYPE=$(rg '^pub type ' crates/pdftract-core/src --type rust -c | awk -F: '{s+=$2} END {print s+0}')

PUBLIC_ITEMS=$((PUBLIC_FN + PUBLIC_STRUCT + PUBLIC_ENUM + PUBLIC_TRAIT + PUBLIC_TYPE))

# Count ```rust blocks (worked examples)
EXAMPLE_BLOCKS=$(rg '```rust' crates/pdftract-core/src --type rust -c | awk -F: '{s+=$2} END {print s+0}')

echo "Public items breakdown:"
echo "  - pub fn: $PUBLIC_FN"
echo "  - pub struct: $PUBLIC_STRUCT"
echo "  - pub enum: $PUBLIC_ENUM"
echo "  - pub trait: $PUBLIC_TRAIT"
echo "  - pub type: $PUBLIC_TYPE"
echo "  Total: $PUBLIC_ITEMS"
echo ""
echo "Example blocks (\`\`\`rust): $EXAMPLE_BLOCKS"
echo ""

if [ "$PUBLIC_ITEMS" -gt 0 ]; then
    COVERAGE=$((EXAMPLE_BLOCKS * 100 / PUBLIC_ITEMS))
    echo "Coverage: $COVERAGE%"
    echo ""
    echo "Target: 80%+"

    if [ "$COVERAGE" -ge 80 ]; then
        echo "✓ PASS: Coverage >= 80%"
    else
        echo "✗ FAIL: Coverage < 80%"
        echo "Need: $((PUBLIC_ITEMS * 80 / 100 - EXAMPLE_BLOCKS + 1)) more examples"
    fi
else
    echo "No public items found"
fi

# List modules that need module-level documentation
echo ""
echo "=== MODULES WITHOUT MODULE-LEVEL DOCS ==="
for f in crates/pdftract-core/src/*.rs; do
    if [ -f "$f" ]; then
        # Check if file has module-level doc (starts with //!)
        if ! head -20 "$f" | grep -q "^//!"; then
            echo "$(basename "$f")"
        fi
    fi
done

# List subdirectories without module docs
for dir in crates/pdftract-core/src/*/; do
    if [ -d "$dir" ]; then
        mod_file="$dir/mod.rs"
        if [ -f "$mod_file" ] && ! head -20 "$mod_file" | grep -q "^//!"; then
            echo "$(basename "$dir")/mod.rs"
        fi
    fi
done

# Sample of public functions without documentation (first 20)
echo ""
echo "=== SAMPLE OF PUBLIC FUNCTIONS WITHOUT DOCS (first 20 lines) ==="
rg '^pub fn ' crates/pdftract-core/src --type rust -n -B2 --multiline --no-ignore 2>/dev/null | grep -B2 '^[0-9]+:pub fn ' | grep -v '///' | head -20 || true
