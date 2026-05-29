//! Generate valid minimal PDF fixtures for document model testing.
//!
//! FIXTURE PASSWORDS:
//! - All encrypted fixtures use user password "test" (NOT secret - these are test fixtures)

use std::fs::File;
use std::io::Write;

fn main() {
    println!("Generating document-model test fixtures...");

    generate_all_fixtures();

    println!("\nAll fixtures generated!");
    println!("Note: Encrypted fixtures need to be manually encrypted with qpdf or similar tool.");
}

fn generate_all_fixtures() {
    create_encrypted_rc4_base();
    create_tagged_3_level_outline();
    create_ocg_default_off();
    create_multi_revision_3();
    create_inheritance_grandparent_mediabox();
    create_missing_mediabox();
    create_partial_resource_override();
    create_js_in_openaction();
    create_xfa_form();
    create_pdfa_1b_conformance();
    create_page_labels_roman_arabic();
}

/// Create base PDF for RC4 encryption (will be encrypted later with qpdf)
fn create_encrypted_rc4_base() {
    let pdf = minimal_pdf("Hello Encrypted", "Test content for encrypted PDF");
    write_pdf("tests/document_model/fixtures/_temp_enc_rc4.pdf", &pdf);
    println!("Created _temp_enc_rc4.pdf (encrypt with: qpdf --encrypt test '' 2 -- _temp_enc_rc4.pdf encrypted_rc4_test.pdf)");
}

/// Create a 3-level outline fixture
fn create_tagged_3_level_outline() {
    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
2 0 obj
<</Length 44>>stream
BT /F1 12 Tf 100 700 Td (Chapter 1) Tj ET
endstream
endobj
3 0 obj
<</Length 47>>stream
BT /F1 12 Tf 100 700 Td (Section 1.1) Tj ET
endstream
endobj
4 0 obj
<</Length 56>>stream
BT /F1 12 Tf 100 700 Td (Subsection 1.1.1) Tj ET
endstream
endobj
5 0 obj
<</Type/Pages/Count 3/Kids[6 0 R 7 0 R 8 0 R]/MediaBox[0 0 612 792]/Resources<</Font<</F1 1 0 R>>>>>
endobj
6 0 obj
<</Type/Page/Parent 5 0 R/Contents 2 0 R/MediaBox[0 0 612 792]>>
endobj
7 0 obj
<</Type/Page/Parent 5 0 R/Contents 3 0 R/MediaBox[0 0 612 792]>>
endobj
8 0 obj
<</Type/Page/Parent 5 0 R/Contents 4 0 R/MediaBox[0 0 612 792]>>
endobj
9 0 obj
<</Title(Chapter 1)/Parent 11 0 R/Dest[6 0 R /Fit]>>
endobj
10 0 obj
<</Title(Section 1.1)/Parent 11 0 R/Prev 9 0 R/Dest[7 0 R /Fit]>>
endobj
11 0 obj
<</Title(Subsection 1.1.1)/Parent 11 0 R/Prev 10 0 R/Dest[8 0 R /Fit]>>
endobj
12 0 obj
<</Type/Outlines/First 9 0 R/Last 11 0 R/Count 3>>
endobj
13 0 obj
<</Type/Catalog/Pages 5 0 R/Outlines 12 0 R>>
endobj
xref
0 14
0000000000 65535 f
0000000009 00000 n
0000000062 00000 n
0000000137 00000 n
0000000216 00000 n
0000000295 00000 n
0000000466 00000 n
0000000569 00000 n
0000000672 00000 n
0000000775 00000 n
0000000890 00000 n
0000001005 00000 n
0000001120 00000 n
0000001219 00000 n
trailer
<</Size 14/Root 13 0 R>>
startxref
1318
%%EOF
"#
    );
    write_pdf("tests/document_model/fixtures/tagged_3_level_outline.pdf", &pdf);
    println!("Created tagged_3_level_outline.pdf (3-level outline hierarchy)");
}

/// Create OCG with /BaseState /OFF
fn create_ocg_default_off() {
    let pdf = format!(
        r#"%PDF-1.5
1 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
2 0 obj
<</Length 35>>stream
BT /F1 12 Tf 100 700 Td (Test) Tj ET
endstream
endobj
3 0 obj
<</Type/OCG/Name(Test Layer)>>
endobj
4 0 obj
<</BaseState/OFF/ON[]>>
endobj
5 0 obj
<</OCGs[3 0 R]/D 4 0 R/Present true>>
endobj
6 0 obj
<</Type/Page/MediaBox[0 0 612 792]/Contents 2 0 R/Resources<</Font<</F1 1 0 R>>>>/Parent 7 0 R>>
endobj
7 0 obj
<</Type/Pages/Count 1/Kids[6 0 R]>>
endobj
8 0 obj
<</Type/Catalog/Pages 7 0 R/OCProperties 5 0 R>>
endobj
xref
0 9
0000000000 65535 f
0000000009 00000 n
0000000062 00000 n
0000000137 00000 n
0000000196 00000 n
0000000229 00000 n
0000000310 00000 n
0000000469 00000 n
0000000522 00000 n
trailer
<</Size 9/Root 8 0 R>>
startxref
629
%%EOF
"#
    );
    write_pdf("tests/document_model/fixtures/ocg_default_off.pdf", &pdf);
    println!("Created ocg_default_off.pdf (OCG with /BaseState /OFF)");
}

/// Create a 3-page PDF for multi-revision testing (base version)
fn create_multi_revision_3() {
    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
2 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (Rev 1) Tj ET
endstream
endobj
3 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (Rev 2) Tj ET
endstream
endobj
4 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (Rev 3) Tj ET
endstream
endobj
5 0 obj
<</Type/Pages/Count 3/Kids[6 0 R 7 0 R 8 0 R]/MediaBox[0 0 612 792]/Resources<</Font<</F1 1 0 R>>>>>
endobj
6 0 obj
<</Type/Page/Parent 5 0 R/Contents 2 0 R/MediaBox[0 0 612 792]>>
endobj
7 0 obj
<</Type/Page/Parent 5 0 R/Contents 3 0 R/MediaBox[0 0 612 792]>>
endobj
8 0 obj
<</Type/Page/Parent 5 0 R/Contents 4 0 R/MediaBox[0 0 612 792]>>
endobj
9 0 obj
<</Type/Catalog/Pages 5 0 R>>
endobj
xref
0 10
0000000000 65535 f
0000000009 00000 n
0000000062 00000 n
0000000135 00000 n
0000000208 00000 n
0000000281 00000 n
0000000452 00000 n
0000000555 00000 n
0000000658 00000 n
0000000761 00000 n
trailer
<</Size 10/Root 9 0 R>>
startxref
864
%%EOF
"#
    );
    write_pdf("tests/document_model/fixtures/multi_revision_3.pdf", &pdf);
    println!("Created multi_revision_3.pdf (base 3-page PDF)");
}

/// Create MediaBox inheritance from grandparent /Pages node
fn create_inheritance_grandparent_mediabox() {
    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
2 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (Page 1) Tj ET
endstream
endobj
3 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (Page 2) Tj ET
endstream
endobj
4 0 obj
<</Type/Pages/Count 2/Kids[5 0 R]/MediaBox[0 0 612 792]>>
endobj
5 0 obj
<</Type/Pages/Count 2/Kids[6 0 R 7 0 R]/Parent 4 0 R/Resources<</Font<</F1 1 0 R>>>>>
endobj
6 0 obj
<</Type/Page/Parent 5 0 R/Contents 2 0 R>>
endobj
7 0 obj
<</Type/Page/Parent 5 0 R/Contents 3 0 R>>
endobj
8 0 obj
<</Type/Catalog/Pages 4 0 R>>
endobj
xref
0 9
0000000000 65535 f
0000000009 00000 n
0000000062 00000 n
0000000135 00000 n
0000000208 00000 n
0000000289 00000 n
0000000474 00000 n
0000000569 00000 n
0000000664 00000 n
trailer
<</Size 9/Root 8 0 R>>
startxref
767
%%EOF
"#
    );
    write_pdf("tests/document_model/fixtures/inheritance_grandparent_mediabox.pdf", &pdf);
    println!("Created inheritance_grandparent_mediabox.pdf (MediaBox from grandparent)");
}

/// Create PDF with no MediaBox anywhere (should default to US Letter)
fn create_missing_mediabox() {
    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<</Length 40>>stream
BT /F1 12 Tf 100 700 Td (No MediaBox) Tj ET
endstream
endobj
2 0 obj
<</Type/Page/Parent 3 0 R/Contents 1 0 R/Resources<</Font<</F1 4 0 R>>>>>
endobj
3 0 obj
<</Type/Pages/Count 1/Kids[2 0 R]/Resources<</Font<</F1 4 0 R>>>>>
endobj
4 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
5 0 obj
<</Type/Catalog/Pages 3 0 R>>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000071 00000 n
0000000184 00000 n
0000000297 00000 n
0000000370 00000 n
trailer
<</Size 6/Root 5 0 R>>
startxref
473
%%EOF
"#
    );
    write_pdf("tests/document_model/fixtures/missing_mediabox.pdf", &pdf);
    println!("Created missing_mediabox.pdf (no MediaBox, defaults to US Letter)");
}

/// Create partial /Resources override fixture
fn create_partial_resource_override() {
    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
2 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Times-Roman>>
endobj
3 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Courier>>
endobj
4 0 obj
<</Type/XObject/Subtype/Image/Width 100/Height 100>>
endobj
5 0 obj
<</Length 49>>stream
BT /F1 12 Tf 100 700 Td (Test Override) Tj ET
endstream
endobj
6 0 obj
<</Font<</F1 1 0 R/F2 2 0 R>>/XObject<</Im1 4 0 R>>>>
endobj
7 0 obj
<</Font<</F1 3 0 R/F3 1 0 R>>>>
endobj
8 0 obj
<</Type/Page/Parent 9 0 R/Contents 5 0 R/Resources 7 0 R/MediaBox[0 0 612 792]>>
endobj
9 0 obj
<</Type/Pages/Count 1/Kids[8 0 R]/Resources 6 0 R>>
endobj
10 0 obj
<</Type/Catalog/Pages 9 0 R>>
endobj
xref
0 11
0000000000 65535 f
0000000009 00000 n
0000000074 00000 n
0000000157 00000 n
0000000240 00000 n
0000000331 00000 n
0000000412 00000 n
0000000513 00000 n
0000000586 00000 n
0000000729 00000 n
0000000802 00000 n
trailer
<</Size 11/Root 10 0 R>>
startxref
899
%%EOF
"#
    );
    write_pdf("tests/document_model/fixtures/partial_resource_override.pdf", &pdf);
    println!("Created partial_resource_override.pdf (partial /Resources override)");
}

/// Create PDF with /OpenAction /S /JavaScript
fn create_js_in_openaction() {
    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
2 0 obj
<</Length 35>>stream
BT /F1 12 Tf 100 700 Td (JS Test) Tj ET
endstream
endobj
3 0 obj
<</S/JavaScript/JS(app.alert('Hello'))>>
endobj
4 0 obj
<</Type/Page/MediaBox[0 0 612 792]/Contents 2 0 R/Resources<</Font<</F1 1 0 R>>>>/Parent 5 0 R>>
endobj
5 0 obj
<</Type/Pages/Count 1/Kids[4 0 R]>>
endobj
6 0 obj
<</Type/Catalog/Pages 5 0 R/OpenAction 3 0 R>>
endobj
xref
0 7
0000000000 65535 f
0000000009 00000 n
0000000062 00000 n
0000000135 00000 n
0000000246 00000 n
0000000425 00000 n
0000000478 00000 n
trailer
<</Size 7/Root 6 0 R>>
startxref
551
%%EOF
"#
    );
    write_pdf("tests/document_model/fixtures/js_in_openaction.pdf", &pdf);
    println!("Created js_in_openaction.pdf (/OpenAction /S /JavaScript)");
}

/// Create PDF with /AcroForm /XFA
fn create_xfa_form() {
    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
2 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (XFA) Tj ET
endstream
endobj
3 0 obj
<</XFA(template)>>
endobj
4 0 obj
<</Type/Page/MediaBox[0 0 612 792]/Contents 2 0 R/Resources<</Font<</F1 1 0 R>>>>/Parent 5 0 R>>
endobj
5 0 obj
<</Type/Pages/Count 1/Kids[4 0 R]>>
endobj
6 0 obj
<</Type/Catalog/Pages 5 0 R/AcroForm 3 0 R>>
endobj
xref
0 7
0000000000 65535 f
0000000009 00000 n
0000000062 00000 n
0000000127 00000 n
0000000182 00000 n
0000000353 00000 n
0000000406 00000 n
trailer
<</Size 7/Root 6 0 R>>
startxref
479
%%EOF
"#
    );
    write_pdf("tests/document_model/fixtures/xfa_form.pdf", &pdf);
    println!("Created xfa_form.pdf (/AcroForm /XFA present)");
}

/// Create PDF/A-1B conformance with XMP metadata
fn create_pdfa_1b_conformance() {
    let xmp = r#"<?xpacket begin="?" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core 5.6-c140 79.160451">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">
      <pdfaid:part>1</pdfaid:part>
      <pdfaid:conformance>B</pdfaid:conformance>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

    let xmp_bytes = xmp.as_bytes();
    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
2 0 obj
<</Length 37>>stream
BT /F1 12 Tf 100 700 Td (PDF/A-1B) Tj ET
endstream
endobj
3 0 obj
<</Type/Metadata/Subtype/XML/Length {}>>
stream
{}
endstream
endobj
4 0 obj
<</Type/Page/MediaBox[0 0 612 792]/Contents 2 0 R/Resources<</Font<</F1 1 0 R>>>>/Parent 5 0 R>>
endobj
5 0 obj
<</Type/Pages/Count 1/Kids[4 0 R]>>
endobj
6 0 obj
<</Type/Catalog/Pages 5 0 R/Metadata 3 0 R>>
endobj
xref
0 7
0000000000 65535 f
0000000009 00000 n
0000000062 00000 n
0000000131 00000 n
000000{:04} 00000 n
000000{:04} 00000 n
000000{:04} 00000 n
trailer
<</Size 7/Root 6 0 R>>
startxref
{:04}
%%EOF
"#,
        xmp_bytes.len(),
        xmp,
        xmp_bytes.len() + 179,
        xmp_bytes.len() + 336,
        xmp_bytes.len() + 425,
        xmp_bytes.len() + 518
    );

    write_pdf("tests/document_model/fixtures/pdfa_1b_conformance.pdf", &pdf);
    println!("Created pdfa_1b_conformance.pdf (XMP PDF/A-1B metadata)");
}

/// Create page labels: pages 0-3 roman, pages 4+ arabic
fn create_page_labels_roman_arabic() {
    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
2 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (Page i) Tj ET
endstream
endobj
3 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (Page ii) Tj ET
endstream
endobj
4 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (Page iii) Tj ET
endstream
endobj
5 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (Page iv) Tj ET
endstream
endobj
6 0 obj
<</Length 33>>stream
BT /F1 12 Tf 100 700 Td (Page 1) Tj ET
endstream
endobj
7 0 obj
<</Type/Pages/Count 5/Kids[8 0 R 9 0 R 10 0 R 11 0 R 12 0 R]/MediaBox[0 0 612 792]/Resources<</Font<</F1 1 0 R>>>>>
endobj
8 0 obj
<</Type/Page/Parent 7 0 R/Contents 2 0 R/MediaBox[0 0 612 792]>>
endobj
9 0 obj
<</Type/Page/Parent 7 0 R/Contents 3 0 R/MediaBox[0 0 612 792]>>
endobj
10 0 obj
<</Type/Page/Parent 7 0 R/Contents 4 0 R/MediaBox[0 0 612 792]>>
endobj
11 0 obj
<</Type/Page/Parent 7 0 R/Contents 5 0 R/MediaBox[0 0 612 792]>>
endobj
12 0 obj
<</Type/Page/Parent 7 0 R/Contents 6 0 R/MediaBox[0 0 612 792]>>
endobj
13 0 obj
<</Nums[0 14 0 R 4 15 0 R]>>
endobj
14 0 obj
<</S/r/St 1>>
endobj
15 0 obj
<</S/D/St 1>>
endobj
16 0 obj
<</Type/Catalog/Pages 7 0 R/PageLabels 13 0 R>>
endobj
xref
0 17
0000000000 65535 f
0000000009 00000 n
0000000062 00000 n
0000000135 00000 n
0000000208 00000 n
0000000281 00000 n
0000000354 00000 n
0000000427 00000 n
0000000600 00000 n
0000000703 00000 n
0000000806 00000 n
0000000909 00000 n
0000001012 00000 n
0000001115 00000 n
0000001150 00000 n
0000001175 00000 n
0000001200 00000 n
trailer
<</Size 17/Root 16 0 R>>
startxref
1283
%%EOF
"#
    );
    write_pdf("tests/document_model/fixtures/page_labels_roman_arabic.pdf", &pdf);
    println!("Created page_labels_roman_arabic.pdf (roman 0-3, arabic 4+)");
}

/// Create a minimal valid PDF document
fn minimal_pdf(title: &str, content: &str) -> String {
    format!(
        r#"%PDF-1.4
1 0 obj
<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>
endobj
2 0 obj
<</Length {}>>stream
BT /F1 12 Tf 100 700 Td ({}) Tj ET
endstream
endobj
3 0 obj
<</Type/Page/MediaBox[0 0 612 792]/Contents 2 0 R/Resources<</Font<</F1 1 0 R>>>>/Parent 4 0 R>>
endobj
4 0 obj
<</Type/Pages/Count 1/Kids[3 0 R]>>
endobj
5 0 obj
<</Type/Catalog/Pages 4 0 R>>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000062 00000 n
000000{:04} 00000 n
000000{:04} 00000 n
000000{:04} 00000 n
trailer
<</Size 6/Root 5 0 R>>
startxref
{:04}
%%EOF
"#,
        content.len() + 30,
        content,
        content.len() + 135,
        content.len() + 264,
        content.len() + 357,
        content.len() + 446
    )
}

/// Write PDF content to a file
fn write_pdf(path: &str, content: &str) {
    let mut file = File::create(path).expect("Failed to create PDF file");
    file.write_all(content.as_bytes()).expect("Failed to write PDF content");
}
