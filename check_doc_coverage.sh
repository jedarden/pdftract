#!/bin/bash
# Comprehensive rustdoc coverage analysis for pdftract-core

set -e

CORE_SRC="crates/pdftract-core/src"

echo "=== pdftract-core rustdoc coverage analysis ==="
echo

# Count public items by type (excluding pub(crate))
echo "Public API item counts:"
echo "======================"
pub_structs=$(grep -r "^pub struct" "$CORE_SRC" --include="*.rs" | wc -l)
pub_enums=$(grep -r "^pub enum" "$CORE_SRC" --include="*.rs" | wc -l)
pub_traits=$(grep -r "^pub trait" "$CORE_SRC" --include="*.rs" | wc -l)
pub_fns=$(grep -r "^pub fn" "$CORE_SRC" --include="*.rs" | wc -l)
pub_types=$(grep -r "^pub type" "$CORE_SRC" --include="*.rs" | wc -l)
pub_consts=$(grep -r "^pub const" "$CORE_SRC" --include="*.rs" | wc -l)
pub_mods=$(grep -r "^pub mod" "$CORE_SRC" --include="*.rs" | wc -l)

total_pub=$((pub_structs + pub_enums + pub_traits + pub_fns + pub_types + pub_consts))
echo "pub structs: $pub_structs"
echo "pub enums: $pub_enums"
echo "pub traits: $pub_traits"
echo "pub functions: $pub_fns"
echo "pub types: $pub_types"
echo "pub consts: $pub_consts"
echo "---"
echo "Total public API items: $total_pub (excluding modules)"

# Count module-level docs
echo
echo "Module documentation:"
echo "===================="
mod_files=$(find "$CORE_SRC" -name "mod.rs" -o -name "*.rs" | grep -v "/mod.rs$" | head -50)
mods_with_doc=0
mods_total=0
for file in $mod_files; do
    # Check if it declares a module (has pub mod inside) or is lib.rs
    if grep -q "pub mod\|^fn main\|^#\[cfg(test)" "$file" 2>/dev/null || [[ "$file" == *"lib.rs" ]]; then
        mods_total=$((mods_total + 1))
        if grep -q "^//!" "$file"; then
            mods_with_doc=$((mods_with_doc + 1))
        else
            echo "Missing module doc: $file"
        fi
    fi
done
echo "Modules with docs: $mods_with_doc / $mods_total"

# Check for worked examples in public items
echo
echo "Items with worked examples:"
echo "==========================="
# Count doc comments with ```rust or ```no_run blocks
items_with_examples=0
for file in $(find "$CORE_SRC" -name "*.rs"); do
    # Find pub items and check if they have doc with code examples
    in_pub_block=0
    in_doc=0
    has_example=0
    while IFS= read -r line; do
        if [[ "$line" =~ ^pub[[:space:]](fn|struct|enum|trait|type|const)[[:space:]] ]]; then
            in_pub_block=1
            in_doc=0
            has_example=0
        elif [[ "$line" =~ ^pub\(crate\) ]] || [[ "$line" =~ ^pub[[:space:]]mod ]] || [[ "$line" =~ ^pub[[:space:]]use ]]; then
            in_pub_block=0
        elif [[ "$line" =~ ^///[[:space:]] ]]; then
            in_doc=1
        elif [[ "$line" =~ '```rust'[[:space:]] || "$line" =~ '```no_run' || "$line" =~ '```ignore' ]]; then
            if [ $in_doc -eq 1 ]; then
                has_example=1
            fi
        elif [[ "$line" =~ ^pub ]] && [ $in_pub_block -eq 1 ] && [[ ! "$line" =~ ^pub\(crate\) ]]; then
            # New pub item, check if previous had example
            if [ $has_example -eq 1 ]; then
                items_with_examples=$((items_with_examples + 1))
            fi
            in_pub_block=1
            in_doc=0
            has_example=0
        fi
    done < "$file"
    # Check last item
    if [ $has_example -eq 1 ]; then
        items_with_examples=$((items_with_examples + 1))
    fi
done

echo "Public items with worked examples: $items_with_examples / $total_pub"
percent=$((items_with_examples * 100 / total_pub))
echo "Coverage: $percent%"

if [ $percent -ge 80 ]; then
    echo "✓ Meets 80% threshold"
else
    echo "✗ Below 80% threshold (need $((80 - percent))% more)"
fi

echo
echo "Checking cargo doc with missing_docs lint..."
echo "============================================="
RUSTDOCFLAGS="-D missing-docs" cargo doc --no-deps -p pdftract-core 2>&1 | tail -20
exit_code=${PIPESTATUS[0]}
if [ $exit_code -eq 0 ]; then
    echo "✓ cargo doc passed"
else
    echo "✗ cargo doc failed with warnings"
fi
