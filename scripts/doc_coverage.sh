#!/usr/bin/env bash
# Measure rustdoc coverage for pdftract-core
# Counts public items and checks which have worked examples

cd /home/coding/pdftract

echo "=== Analyzing pdftract-core public API documentation coverage ==="
echo ""

# Find all .rs files in pdftract-core/src
RS_FILES=$(find crates/pdftract-core/src -name "*.rs" -type f)

# Total public items (pub fn, pub struct, pub enum, pub trait, pub type, pub mod)
TOTAL_PUB=$(grep -rhE '^pub (fn|struct|enum|trait|type|mod|const|static)' crates/pdftract-core/src | wc -l)

echo "Total public items: $TOTAL_PUB"

# Items with any documentation (/// or //!)
WITH_ANY_DOC=$(grep -rhE '^///|^//!' crates/pdftract-core/src | wc -l)
echo "Items with documentation comments: $WITH_ANY_DOC"

# Items with code examples (containing ```rust)
WITH_EXAMPLES=$(grep -rE '```rust' crates/pdftract-core/src | wc -l)
echo "Items with code examples: $WITH_EXAMPLES"

# Calculate percentage
if [ "$TOTAL_PUB" -gt 0 ]; then
    PERCENT=$((100 * WITH_EXAMPLES / TOTAL_PUB))
    echo "Coverage: ${PERCENT}%"

    if [ "$PERCENT" -ge 80 ]; then
        echo "✓ PASS: Meets 80% threshold"
    else
        echo "✗ FAIL: Below 80% threshold"
    fi
fi

echo ""
echo "=== Detailed breakdown ==="
echo "Public functions: $(grep -rhE '^pub fn' crates/pdftract-core/src | wc -l)"
echo "Public structs: $(grep -rhE '^pub struct' crates/pdftract-core/src | wc -l)"
echo "Public enums: $(grep -rhE '^pub enum' crates/pdftract-core/src | wc -l)"
echo "Public traits: $(grep -rhE '^pub trait' crates/pdftract-core/src | wc -l)"
echo "Public types: $(grep -rhE '^pub type' crates/pdftract-core/src | wc -l)"
echo "Public consts: $(grep -rhE '^pub (const|static)' crates/pdftract-core/src | wc -l)"
