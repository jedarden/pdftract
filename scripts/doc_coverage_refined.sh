#!/bin/bash
# Measure rustdoc example coverage for pdftract-core public API only

cd /home/coding/pdftract

echo "=== Public API Rustdoc Coverage Report ==="
echo ""

# The public API consists of:
# 1. Re-exported types from lib.rs (pub use ...)
# 2. Module-level docs (pub mod ...)

# Focus on key public API modules that are re-exported
api_modules="extract schema options forms markdown table text document page_class source glyph span confidence"

echo "Checking coverage in key public API modules:"
echo ""

total_examples=0

for module in $api_modules; do
    if [ -f "crates/pdftract-core/src/${module}.rs" ]; then
        count=$(grep -cE '```rust(,no_run|,ignore)?' "crates/pdftract-core/src/${module}.rs" 2>/dev/null || echo 0)
        total_examples=$((total_examples + count))
        echo "  ${module}.rs: ${count} examples"
    fi
done

echo ""
echo "Total doc examples in key public API: $total_examples"

# Also count examples in lib.rs (crate-level doc)
lib_examples=$(grep -cE '```rust(,no_run|,ignore)?' crates/pdftract-core/src/lib.rs 2>/dev/null || echo 0)
echo "lib.rs (crate-level): $lib_examples examples"

total_with_lib=$((total_examples + lib_examples))
echo "Total: $total_with_lib examples"

# Count public API items roughly (this is an estimate)
# We count pub fn/struct/enum/type in the key modules
total_pub_items=0
for module in $api_modules; do
    if [ -f "crates/pdftract-core/src/${module}.rs" ]; then
        count=$(find crates/pdftract-core/src -name "${module}.rs" -exec grep -hE "^\s*pub (fn|struct|enum|trait|type)" {} \; | wc -l)
        total_pub_items=$((total_pub_items + count))
    fi
done

echo "Est. public API items: $total_pub_items"

if [ "$total_pub_items" -gt 0 ]; then
    coverage=$(awk "BEGIN {printf \"%.1f\", ($total_with_lib * 100.0 / $total_pub_items)}")
    echo "Coverage (public API only): ${coverage}%"

    target=$(awk "BEGIN {printf \"%d\", ($total_pub_items * 0.8)}")
    needed=$((target - total_with_lib))

    if [ "$total_with_lib" -lt "$target" ]; then
        echo ""
        echo "⚠️  Coverage below 80% target"
        echo "Need $needed more examples to reach 80%"
        exit 1
    else
        echo ""
        echo "✅ Coverage meets 80% target"
    fi
fi
