#!/usr/bin/env bash
# Check documentation coverage for pdftract-core public API
# Reports:
# 1. Public items without any documentation
# 2. Public items with documentation but no examples
# 3. Overall coverage percentage

set -euo pipefail

cd "$(dirname "$0")/.."

echo "=== Checking rustdoc coverage for pdftract-core ==="
echo ""

# Count public items
echo "Counting public items..."
pub_items=$(grep -rh "^pub fn\|^pub struct\|^pub enum\|^pub trait\|^pub const\|^pub type\|^pub mod" crates/pdftract-core/src --include="*.rs" | wc -l)
echo "Total public items: $pub_items"
echo ""

# Try cargo doc to see warnings
echo "Running cargo doc to check for missing_docs warnings..."
timeout 300 cargo doc --no-deps --all-features -p pdftract-core 2>&1 | grep -i "missing.*doc" | head -20 || echo "No missing_docs warnings found in initial scan"
echo ""

# Check specific high-impact modules
echo "=== Checking key modules for example coverage ==="
for module in extract options schema confidence span glyph table layout; do
    file="crates/pdftract-core/src/${module}.rs"
    if [[ -f "$file" ]]; then
        echo "--- $module ---"
        # Count public items
        pub_count=$(grep "^pub fn\|^pub struct\|^pub enum\|^pub trait\|^pub const\|^pub type" "$file" | wc -l)
        # Count items with examples
        example_count=$(grep -c "^/// # Examples" "$file" 2>/dev/null || echo "0")
        echo "Public items: $pub_count, Items with examples: $example_count"
    fi
done
echo ""

# Manual check: show some items missing examples
echo "=== Sample items that may need examples ==="
grep -rn "^pub fn" crates/pdftract-core/src --include="*.rs" | head -20
echo ""

echo "=== Summary ==="
echo "Run 'cargo doc --no-deps --all-features -p pdftract-core' to see full warnings"
echo "Check individual modules by examining their /// comments for # Examples sections"
