#!/bin/bash
# Generate a minimal valid PDF for testing
# Usage: ./generate-minimal-pdf.sh <output-file> <page-count>

set -e

OUTPUT_FILE="${1:-test.pdf}"
PAGE_COUNT="${2:-1}"

# Create a minimal PDF with specified page count
# This generates a valid PDF structure with repeated pages

cat > "$OUTPUT_FILE" <<'EOF'
%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [
EOF

# Add page references
for ((i=3; i<3+PAGE_COUNT; i++)); do
    echo "$i 0 R" >> "$OUTPUT_FILE"
done

cat >> "$OUTPUT_FILE" <<'EOF'
]
/Count <<PAGE_COUNT>>
>>
endobj

# Generate pages
PAGE_NUM=3
for ((i=1; i<=PAGE_COUNT; i++)); do
    cat >> "$OUTPUT_FILE" <<PAGEEOF
${PAGE_NUM} 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [ 0 0 612 792 ]
/Contents 4 0 R
/Resources <<
/Font <<
/F1 5 0 R
>>
>>
>>
endobj
PAGEEOF
    PAGE_NUM=$((PAGE_NUM + 1))
done

# Content stream (simple text)
cat >> "$OUTPUT_FILE" <<'EOF'
4 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
50 700 Td
(Test Page) Tj
ET
endstream
endobj
5 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000135 00000 n
0000000265 00000 n
0000000365 00000 n
trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
447
%%EOF
EOF

# Replace page count placeholder
sed -i "s/<<PAGE_COUNT>>/$PAGE_COUNT/" "$OUTPUT_FILE"

echo "Generated $OUTPUT_FILE with $PAGE_COUNT page(s)"
