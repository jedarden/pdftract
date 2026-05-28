#!/usr/bin/env bash
# Generate document model test fixtures
# Requires: qpdf (via nix-shell)

set -e

FIXTURES_DIR="tests/document_model/fixtures"
BASE_PDF="$FIXTURES_DIR/base_hello.pdf"

# Create a minimal base PDF for encryption
create_base_pdf() {
    cat > "$BASE_PDF" <<'EOF'
%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj
3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/Resources<</Font<</F1 4 0 R>>>/Contents 5 0 R>>endobj
4 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj
5 0 obj<</Length 44>>stream
BT /F1 12 Tf 100 700 Td (Hello World) Tj ET
endstream endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000302 00000 n
0000000377 00000 n
trailer<</Size 6/Root 1 0 R>>
startxref 445
%%EOF
EOF
    echo "Created base PDF: $BASE_PDF"
}

# Generate encrypted fixtures
generate_encrypted() {
    echo "Generating encrypted fixtures..."

    # RC4-40 with password "test" (EC-04)
    nix-shell -p qpdf --run "qpdf --allow-weak-crypto --encrypt test test 40 -- $BASE_PDF $FIXTURES_DIR/encrypted_rc4_test.pdf"

    # AES-128 with password "test" (EC-05)
    nix-shell -p qpdf --run "qpdf --encrypt test test 128 -- $BASE_PDF $FIXTURES_DIR/encrypted_aes128_test.pdf"

    # AES-256 with password "test" (EC-06) - requires PDF 2.0
    nix-shell -p qpdf --run "qpdf --encrypt test test 256 --force-version=2.0 -- $BASE_PDF $FIXTURES_DIR/encrypted_aes256_test.pdf"

    # Empty password (RC4-40)
    nix-shell -p qpdf --run "qpdf --allow-weak-crypto --encrypt '' '' 40 -- $BASE_PDF $FIXTURES_DIR/encrypted_empty_password.pdf"

    echo "Encrypted fixtures generated."
}

# Generate tagged PDF with 3-level outline
generate_tagged_outline() {
    echo "Generating tagged_3_level_outline.pdf..."

    cat > "$FIXTURES_DIR/tagged_3_level_outline.pdf" <<'EOF'
%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R/Outlines 3 0 R>>endobj
2 0 obj<</Type/Pages/Count 2/Kids[4 0 R 5 0 R]>>endobj
3 0 obj<</Type/Outlines/First 6 0 R/Last 7 0 R/Count 2>>endobj
4 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
5 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
6 0 obj<</Title(Chapter 1)/Parent 3 0 R/Next 7 0 R/First 8 0 R/Count 1/Dest[4 0 R /XYZ 0 792 null]>>endobj
7 0 obj<</Title(Chapter 2)/Parent 3 0 R/Prev 6 0 R/Dest[5 0 R /XYZ 0 792 null]>>endobj
8 0 obj<</Title(Section 1.1)/Parent 6 0 R/Dest[4 0 R /XYZ 0 700 null]>>endobj
xref
0 9
0000000000 65535 f
0000000009 00000 n
0000000066 00000 n
0000000133 00000 n
0000000222 00000 n
0000000313 00000 n
0000000404 00000 n
0000000549 00000 n
0000000680 00000 n
trailer<</Size 9/Root 1 0 R>>
startxref 795
%%EOF
EOF
    echo "Generated tagged_3_level_outline.pdf"
}

# Generate OCG with default OFF (EC-16)
generate_ocg_off() {
    echo "Generating ocg_default_off.pdf..."

    cat > "$FIXTURES_DIR/ocg_default_off.pdf" <<'EOF'
%PDF-1.5
1 0 obj<</Type/Catalog/Pages 2 0 R/OCProperties</D</BaseState/OFF/ON[]/OFF[5 0 R]>>>>>endobj
2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj
3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/OCMD 4 0 R>>endobj
4 0 obj<</OCGs 5 0 R/P/ON>>endobj
5 0 obj[/OCG1]endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000157 00000 n
0000000232 00000 n
0000000331 00000 n
0000000424 00000 n
trailer<</Size 6/Root 1 0 R>>
startxref 509
%%EOF
EOF
    echo "Generated ocg_default_off.pdf"
}

# Generate multi-revision PDF (3 revisions)
generate_multi_revision() {
    echo "Generating multi_revision_3.pdf..."

    cat > "$FIXTURES_DIR/multi_revision_3.pdf" <<'EOF'
%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Count 3/Kids[3 0 R 4 0 R 5 0 R]>>endobj
3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
4 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
5 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000125 00000 n
0000000222 00000 n
0000000319 00000 n
trailer<</Size 6/Root 1 0 R>>
startxref 416
%%EOF
EOF
    echo "Generated multi_revision_3.pdf"
}

# Generate inheritance test fixtures
generate_inheritance() {
    echo "Generating inheritance fixtures..."

    cat > "$FIXTURES_DIR/inheritance_grandparent_mediabox.pdf" <<'EOF'
%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]/MediaBox[0 0 612 792]>>endobj
3 0 obj<</Type/Pages/Count 1/Kids[4 0 R]>>endobj
4 0 obj<</Type/Page/Parent 3 0 R>>endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000157 00000 n
0000000240 00000 n
trailer<</Size 5/Root 1 0 R>>
startxref 325
%%EOF
EOF

    cat > "$FIXTURES_DIR/missing_mediabox.pdf" <<'EOF'
%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj
3 0 obj<</Type/Page/Parent 2 0 R>>endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000125 00000 n
trailer<</Size 4/Root 1 0 R>>
startxref 210
%%EOF
EOF

    echo "Generated inheritance fixtures."
}

# Generate partial resource override fixture
generate_partial_override() {
    echo "Generating partial_resource_override.pdf..."

    cat > "$FIXTURES_DIR/partial_resource_override.pdf" <<'EOF'
%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Count 2/Kids[3 0 R 4 0 R]/Resources<</Font<</F1 5 0 R/F2 6 0 R>>>>>endobj
3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/Resources<</Font<</F3 7 0 R>>>/Contents 8 0 R>>endobj
4 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj
6 0 obj<</Type/Font/Subtype/Type1/BaseFont/Times-Roman>>endobj
7 0 obj<</Type/Font/Subtype/Type1/BaseFont/Courier>>endobj
8 0 obj<</Length 44>>stream
BT /F3 12 Tf 100 700 Td (Partial override) Tj ET
endstream endobj
xref
0 9
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000245 00000 n
0000000450 00000 n
0000000547 00000 n
0000000636 00000 n
0000000747 00000 n
0000000838 00000 n
trailer<</Size 9/Root 1 0 R>>
startxref 945
%%EOF
EOF
    echo "Generated partial_resource_override.pdf"
}

# Generate JavaScript fixture
generate_js() {
    echo "Generating js_in_openaction.pdf..."

    cat > "$FIXTURES_DIR/js_in_openaction.pdf" <<'EOF'
%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R/OpenAction</S/JavaScript/JS(app.alert('Hello'))>>>>endobj
2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj
3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000176 00000 n
0000000263 00000 n
trailer<</Size 4/Root 1 0 R>>
startxref 348
%%EOF
EOF
    echo "Generated js_in_openaction.pdf"
}

# Generate XFA form fixture
generate_xfa() {
    echo "Generating xfa_form.pdf..."

    cat > "$FIXTURES_DIR/xfa_form.pdf" <<'EOF'
%PDF-1.6
1 0 obj<</Type/Catalog/Pages 2 0 R/AcroForm 3 0 R>>endobj
2 0 obj<</Type/Pages/Count 1/Kids[4 0 R]>>endobj
3 0 obj<</XFA[(xfa.xml)]/Fields[5 0 R]>>endobj
4 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
5 0 obj<</T(Field1)/V(Test value)>>endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000134 00000 n
0000000227 00000 n
0000000330 00000 n
0000000439 00000 n
trailer<</Size 6/Root 1 0 R>>
startxref 528
%%EOF
EOF
    echo "Generated xfa_form.pdf"
}

# Generate PDF/A-1B conformance fixture
generate_pdfa() {
    echo "Generating pdfa_1b_conformance.pdf..."

    cat > "$FIXTURES_DIR/pdfa_1b_conformance.pdf" <<'EOF'
%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R/Metadata 3 0 R>>endobj
2 0 obj<</Type/Pages/Count 1/Kids[4 0 R]>>endobj
3 0 obj<</Type/Metadata/Subtype/XML/Length 220>>stream
<?xpacket begin="utf-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">
<pdfaid:part>1</pdfaid:part>
<pdfaid:conformance>B</pdfaid:conformance>
</rdf:Description>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>
endstream endobj
4 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000134 00000 n
0000000235 00000 n
0000000609 00000 n
trailer<</Size 5/Root 1 0 R>>
startxref 682
%%EOF
EOF
    echo "Generated pdfa_1b_conformance.pdf"
}

# Generate page labels fixture
generate_page_labels() {
    echo "Generating page_labels_roman_arabic.pdf..."

    cat > "$FIXTURES_DIR/page_labels_roman_arabic.pdf" <<'EOF'
%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R/PageLabels 3 0 R>>endobj
2 0 obj<</Type/Pages/Count 6/Kids[4 0 R 5 0 R 6 0 R 7 0 R 8 0 R 9 0 R]>>endobj
3 0 obj<</Nums[0</S/R>>4</S/D>>]>>endobj
4 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
5 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
6 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
7 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
8 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
9 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
xref
0 10
0000000000 65535 f
0000000009 00000 n
0000000134 00000 n
0000000269 00000 n
0000000447 00000 n
0000000554 00000 n
0000000661 00000 n
0000000768 00000 n
0000000875 00000 n
0000000982 00000 n
trailer<</Size 10/Root 1 0 R>>
startxref 1089
%%EOF
EOF
    echo "Generated page_labels_roman_arabic.pdf"
}

# Generate unknown handler fixture
generate_unknown_handler() {
    echo "Generating encrypted_unknown_handler.pdf..."

    cat > "$FIXTURES_DIR/encrypted_unknown_handler.pdf" <<'EOF'
%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj
3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj
4 0 obj<</O(1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef)/U(1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef)/P -1340>>endobj
5 0 obj<</O(1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef)/U(1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef)/P -1340>>endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000125 00000 n
0000000204 00000 n
0000000409 00000 n
trailer<</Size 6/Root 1 0 R/Encrypt</Filter/Adobe.PubSec/V 2/R 2/P -1340/O 4 0 R/U 5 0 R>>/ID[<1234567890abcdef1234567890abcdef><fedcba0987654321fedcba0987654321>]>>
startxref 614
%%EOF
EOF
    echo "Generated encrypted_unknown_handler.pdf"
}

# Main execution
main() {
    echo "Generating document model test fixtures..."

    mkdir -p "$FIXTURES_DIR"

    create_base_pdf
    generate_encrypted
    generate_tagged_outline
    generate_ocg_off
    generate_multi_revision
    generate_inheritance
    generate_partial_override
    generate_js
    generate_xfa
    generate_pdfa
    generate_page_labels
    generate_unknown_handler

    echo "All fixtures generated successfully!"
    echo "Fixtures are in: $FIXTURES_DIR"
}

main "$@"
