#!/bin/bash
# Measure rustdoc coverage for pdftract-core public API

# Count public items (functions, structs, enums, traits, type aliases)
# Filter for examples by checking for ```rust in doc comments

echo "=== PDFTRACT-CORE DOCUMENTATION COVERAGE ==="
echo ""

# Find all Rust files in pdftract-core/src
CORE_SRC="/home/coding/pdftract/crates/pdftract-core/src"

# Count public items
pub_fn=$(grep -h "^pub fn" $(find $CORE_SRC -name "*.rs") | wc -l)
pub_struct=$(grep -h "^pub struct" $(find $CORE_SRC -name "*.rs") | wc -l)
pub_enum=$(grep -h "^pub enum" $(find $CORE_SRC -name "*.rs") | wc -l)
pub_trait=$(grep -h "^pub trait" $(find $CORE_SRC -name "*.rs") | wc -l)
pub_type=$(grep -h "^pub type" $(find $CORE_SRC -name "*.rs") | wc -l)
pub_mod=$(grep -h "^pub mod" $(find $CORE_SRC -name "*.rs") | wc -l)

total=$((pub_fn + pub_struct + pub_enum + pub_trait + pub_type + pub_mod))

echo "Public item counts:"
echo "  Functions: $pub_fn"
echo "  Structs: $pub_struct"
echo "  Enums: $pub_enum"
echo "  Traits: $pub_trait"
echo "  Type aliases: $pub_type"
echo "  Modules: $pub_mod"
echo "  TOTAL: $total"
echo ""

# Count items with doc comments (/// or //!)
doc_items=$(grep -h "^[[:space:]]*///" $(find $CORE_SRC -name "*.rs") | wc -l)
echo "Total doc comment lines: $doc_items"
echo ""

# Count examples (```rust blocks)
examples=$(grep -h "\`\`\`rust" $(find $CORE_SRC -name "*.rs") | wc -l)
echo "Total example blocks: $examples"
echo ""

echo "=== FILES WITHOUT MODULE-LEVEL DOCS ==="
# Check for files that lack module-level //!
for file in $(find $CORE_SRC -name "*.rs" -not -path "*/mod.rs"); do
    # Skip lib.rs (it has docs)
    if [[ "$file" == *"lib.rs" ]]; then
        continue
    fi
    # Check if file has //! at the beginning
    if ! head -5 "$file" | grep -q "^//!"; then
        echo "  ${file#$CORE_SRC/}"
    fi
done
echo ""

echo "=== COVERAGE METRICS ==="
# Estimate: each well-documented item needs at least one doc comment
# Example coverage: examples / (total items requiring examples)
# Items that should have examples: fn, struct, enum, trait
items_needing_examples=$((pub_fn + pub_struct + pub_enum + pub_trait))

if [[ $items_needing_examples -gt 0 ]]; then
    coverage=$((examples * 100 / items_needing_examples))
    echo "Example coverage: $coverage% ($examples examples / $items_needing_examples items)"
else
    echo "No items needing examples found"
fi

# Target is 80%
target=$((items_needing_examples * 80 / 100))
needed=$((target - examples))
if [[ $needed -gt 0 ]]; then
    echo "Need $needed more example blocks to reach 80% coverage"
else
    echo "✓ 80% coverage target met!"
fi
