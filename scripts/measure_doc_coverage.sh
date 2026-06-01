#!/bin/sh
# Measure rustdoc coverage for pdftract-core

echo "Measuring rustdoc coverage for pdftract-core..."
echo ""

cd crates/pdftract-core

# Count public items
public_items=$(grep -r "^pub " src/ --include="*.rs" | wc -l)

# Count items with documentation
doc_items=$(grep -r "^///\|^//!" src/ --include="*.rs" | wc -l)

# Count items with worked examples
example_items=$(grep -r "^\`\\\`\\\`rust" src/ --include="*.rs" | wc -l)

echo "Public items found: $public_items"
echo "Items with docs: $doc_items"
echo "Items with examples: $example_items"
echo ""

# Count examples more accurately (looking for ```rust anywhere in doc comments)
example_items_total=$(grep -r "rust" src/ --include="*.rs" | grep -c "\`\`\`" || echo 0)
echo "Approximate example count (contains ```): $example_items_total"
echo ""

cd ../..
