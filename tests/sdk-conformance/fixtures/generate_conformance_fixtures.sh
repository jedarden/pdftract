#!/bin/bash
# Generate minimal valid PDF fixtures for SDK conformance tests.

set -e

FIXTURE_DIR="$(dirname "$0")"

# Create fixture subdirectories
mkdir -p "$FIXTURE_DIR/scientific_paper"
mkdir -p "$FIXTURE_DIR/misc"
mkdir -p "$FIXTURE_DIR/contract"
mkdir -p "$FIXTURE_DIR/invoice"
mkdir -p "$FIXTURE_DIR/code"
mkdir -p "$FIXTURE_DIR/vertical"
mkdir -p "$FIXTURE_DIR/xmp"
mkdir -p "$FIXTURE_DIR/receipts"

# Function to create a minimal valid single-page PDF
# Usage: create_minimal_pdf <output_path> <text_content>
create_minimal_pdf() {
    local output="$1"
    local text="$2"

    cat > "$output" <<EOF
%PDF-1.4
1 0 obj
<</Type/Catalog/Pages 2 0 R>>
endobj
2 0 obj
<</Type/Pages/Kids[3 0 R]/Count 1>>
endobj
3 0 obj
<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>>>>>/Contents 4 0 R>>
endobj
4 0 obj
<</Length $(echo -n "$text" | wc -c)>>
stream
BT
/F1 12 Tf
50 700 Td
(${text}) Tj
ET
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000296 00000 n
trailer
<</Size 5/Root 1 0 R>>
startxref
$(echo -n "$text" | wc -c | awk '{print 396 + $1}')
%%EOF
EOF
}

# Create scientific paper fixtures (01-14)
for i in {01..14}; do
    create_minimal_pdf "$FIXTURE_DIR/scientific_paper/$i.pdf" "Scientific Paper $i with Abstract and References"
done

# Create misc fixtures
create_minimal_pdf "$FIXTURE_DIR/misc/01.pdf" "Receipt scanned document"
create_minimal_pdf "$FIXTURE_DIR/misc/02.pdf" "Minimal document"
create_minimal_pdf "$FIXTURE_DIR/misc/03.pdf" "Receipt document"

# Create contract fixture
create_minimal_pdf "$FIXTURE_DIR/contract/01.pdf" "AGREEMENT contract with table"

# Create invoice fixture
create_minimal_pdf "$FIXTURE_DIR/invoice/01.pdf" "INVOICE-12345 for services"

# Create code fixture
create_minimal_pdf "$FIXTURE_DIR/code/code.pdf" "function test() { return true; }"

# Create vertical fixture
create_minimal_pdf "$FIXTURE_DIR/vertical/vertical.pdf" "Vertical text"

# Create XMP fixture
create_minimal_pdf "$FIXTURE_DIR/xmp/xmp-metadata.pdf" "XMP metadata document"

# Create receipt fixtures (empty JSONs for now)
echo '{"fingerprint": "test", "page_index": 0, "span_index": 0, "bbox": [0,0,100,100], "content_hash": "test"}' > "$FIXTURE_DIR/receipts/valid-receipt.receipt.json"
echo '{"fingerprint": "test", "page_index": 0, "span_index": 0, "bbox": [0,0,100,100], "content_hash": "tampered"}' > "$FIXTURE_DIR/receipts/tampered-receipt.receipt.json"
create_minimal_pdf "$FIXTURE_DIR/receipts/valid-receipt.pdf" "Valid receipt content"
create_minimal_pdf "$FIXTURE_DIR/receipts/tampered-receipt.pdf" "Tampered receipt content"

echo "Generated conformance fixtures in $FIXTURE_DIR"
ls -la "$FIXTURE_DIR"/scientific_paper/*.pdf | head -5
