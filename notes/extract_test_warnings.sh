#!/bin/bash
# Extract and categorize test file warnings from cargo output

CARGO_OUTPUT="/home/coding/pdftract/notes/bf-5kjp4b-cargo-check-output.txt"
OUTPUT_FILE="/home/coding/pdftract/notes/bf-5kjp4b-child3-categorized.txt"

# Create output header
cat > "$OUTPUT_FILE" << 'EOF'
# Test File Warnings - Categorized Report

Generated: 2026-08-09
Bead: bf-5kjp4b-child-3

This report categorizes all warnings specific to test files from the cargo check output.

---

EOF

echo "Extracting test file warnings..."

# Extract detailed warnings with file paths
echo "## Detailed Test File Warnings" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Pattern 1: Warnings with file paths in test directories
echo "### Warnings with Explicit File Paths" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

grep -E "warning:.*-->.*tests/" "$CARGO_OUTPUT" | while IFS= read -r line; do
    echo "$line" >> "$OUTPUT_FILE"
done

echo "" >> "$OUTPUT_FILE"
echo "---" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Pattern 2: Test-specific warnings (TH-NN, test_*, etc.)
echo "### Security Test Harness (TH-NN) Warnings" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

grep -E "TH-[0-9]+" "$CARGO_OUTPUT" | grep "warning:" | while IFS= read -r line; do
    echo "$line" >> "$OUTPUT_FILE"
done

echo "" >> "$OUTPUT_FILE"
echo "---" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Pattern 3: Property test warnings
echo "### Property Test (proptest) Warnings" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

grep -i "proptest" "$CARGO_OUTPUT" | grep "warning:" | while IFS= read -r line; do
    echo "$line" >> "$OUTPUT_FILE"
done

echo "" >> "$OUTPUT_FILE"
echo "---" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Extract warning type statistics
echo "## Warning Type Statistics" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Count by warning type
echo "| Warning Type | Count |" >> "$OUTPUT_FILE"
echo "|--------------|-------|" >> "$OUTPUT_FILE"

# unused imports
unused_imports=$(grep -c "unused import" "$CARGO_OUTPUT" || echo "0")
echo "| unused_imports | $unused_imports |" >> "$OUTPUT_FILE"

# unused variables
unused_vars=$(grep -c "unused variable" "$CARGO_OUTPUT" || echo "0")
echo "| unused_variables | $unused_vars |" >> "$OUTPUT_FILE"

# dead code
dead_code=$(grep -c "dead_code" "$CARGO_OUTPUT" || echo "0")
echo "| dead_code | $dead_code |" >> "$OUTPUT_FILE"

# unused mut
unused_mut=$(grep -c "unused_mut" "$CARGO_OUTPUT" || echo "0")
echo "| unused_mut | $unused_mut |" >> "$OUTPUT_FILE"

# unused assignments
unused_assign=$(grep -c "unused_assignments" "$CARGO_OUTPUT" || echo "0")
echo "| unused_assignments | $unused_assign |" >> "$OUTPUT_FILE"

# unreachable patterns
unreachable=$(grep -c "unreachable" "$CARGO_OUTPUT" || echo "0")
echo "| unreachable_patterns | $unreachable |" >> "$OUTPUT_FILE"

# doc comments
doc_comments=$(grep -c "unused_doc_comments" "$CARGO_OUTPUT" || echo "0")
echo "| unused_doc_comments | $doc_comments |" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# Test summary
echo "## Test File Warning Summary" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "Individual test files that generated warnings:" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

grep -E 'warning:.*\(test.*".*"\).*generated' "$CARGO_OUTPUT" | while IFS= read -r line; do
    echo "$line" >> "$OUTPUT_FILE"
done

echo "" >> "$OUTPUT_FILE"

# Binary test warnings
echo "### Binary/Example Test Warnings" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

grep -E 'warning:.*\(bin.*test\).*generated' "$CARGO_OUTPUT" | while IFS= read -r line; do
    echo "$line" >> "$OUTPUT_FILE"
done

grep -E 'warning:.*\(example.*\).*generated' "$CARGO_OUTPUT" | while IFS= read -r line; do
    echo "$line" >> "$OUTPUT_FILE"
done

echo "" >> "$OUTPUT_FILE"
echo "---" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

echo "Report generation complete: $OUTPUT_FILE"
wc -l "$OUTPUT_FILE"