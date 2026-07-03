//! Generate unmapped glyph test fixture for comprehensive Unicode recovery testing.
//!
//! This program creates a PDF with 6 unmapped glyphs and 3 mapped glyphs to exercise
//! the 4-level Unicode fallback chain failure path and verify GLYPH_UNMAPPED diagnostic emission.
//!
//! Based on design: notes/bf-68f9i-design.md
//! Glyph selection: notes/bf-68f9i-glyphs.md
//!
//! Usage: cargo run --bin gen_unmapped_comprehensive

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = "tests/fixtures/encoding";
    std::fs::create_dir_all(out_dir)?;

    println!("Generating unmapped-comprehensive fixture...");

    generate_unmapped_comprehensive_pdf(out_dir)?;

    println!("\nFixture generated successfully!");
    println!("PDF: {}/unmapped-comprehensive.pdf", out_dir);
    println!("Ground truth: {}/unmapped-comprehensive.txt", out_dir);
    Ok(())
}

/// Build a complete PDF with proper xref table and trailer.
fn build_pdf(objects: Vec<Vec<u8>>) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    // Track object offsets
    let mut offsets = Vec::new();
    for obj in &objects {
        offsets.push(pdf.len());
        pdf.extend(obj);
        pdf.extend(b"\n");
    }

    let xref_start = pdf.len();

    // Build xref table
    pdf.extend(b"xref\n");
    pdf.extend(format!("0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend(b"0000000000 65535 f \n"); // Free entry for object 0

    for offset in offsets {
        pdf.extend(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    // Build trailer
    pdf.extend(b"trailer\n<<\n/Size ");
    pdf.extend(format!("{}", objects.len() + 1).as_bytes());
    pdf.extend(b"\n/Root 1 0 R\n>>\n");

    // Build startxref
    pdf.extend(b"startxref\n");
    pdf.extend(format!("{}\n", xref_start).as_bytes());
    pdf.extend(b"%%EOF\n");

    pdf
}

fn generate_unmapped_comprehensive_pdf(out_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("  Generating unmapped-comprehensive.pdf...");

    // Content stream with 3 lines:
    // Line 1: Three PUA glyphs (codes 0, 1, 2) -> <000102>
    // Line 2: Custom and orphaned glyphs (codes 3, 4, 5, 6) -> <03040506>
    // Line 3: Mapped AGL glyphs (codes 7, 8, 9) -> <070809>
    let content = b"BT\n/F1 12 Tf\n50 700 Td\n<000102> Tj\n50 680 Td\n<03040506> Tj\n50 660 Td\n<070809> Tj\nET\n";

    let objects = vec![
        // Object 1: Catalog
        b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj".to_vec(),
        // Object 2: Pages node
        b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj".to_vec(),
        // Object 3: Page dictionary
        b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Resources <<\n/Font <<\n/F1 5 0 R\n>>\n>>\n/Contents 4 0 R\n>>\nendobj".to_vec(),
        // Object 4: Content stream
        format!("4 0 obj\n<<\n/Length {}\n>>\nstream\n{}\nendstream\nendobj",
            content.len(),
            std::str::from_utf8(content).unwrap()).into_bytes(),
        // Object 5: Font dictionary with custom encoding
        // Character codes 0-9 map to glyph names:
        // 0->/g001, 1->/g002, 2->/g003 (PUA - unmapped)
        // 3->/CustomA, 4->/CustomB (custom - unmapped)
        // 5->/NotAGlyph (orphaned - unmapped)
        // 6->/glyph_0041 (non-AGL algorithmic - unmapped)
        // 7->/A, 8->/B, 9->/space (AGL - mapped)
        b"5 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /UnmappedTestFont\n/Encoding <<\n/Type /Encoding\n/Differences [0 /g001 /g002 /g003 /CustomA /CustomB /NotAGlyph /glyph_0041 /A /B /space]\n>>\n>>\nendobj".to_vec(),
    ];

    let pdf = build_pdf(objects);

    let pdf_path = format!("{}/unmapped-comprehensive.pdf", out_dir);
    let mut file = File::create(&pdf_path)?;
    file.write_all(&pdf)?;

    // Ground truth: 7 × U+FFFD + "AB "
    // Line 1: ��� (3 U+FFFD for /g001, /g002, /g003)
    // Line 2: ���� (4 U+FFFD for /CustomA, /CustomB, /NotAGlyph, /glyph_0041)
    // Line 3: AB  (U+0041, U+0042, U+0020 for /A, /B, /space)
    let txt_path = format!("{}/unmapped-comprehensive.txt", out_dir);
    let mut txt_file = File::create(&txt_path)?;

    // Write the expected output: 3 lines
    txt_file.write_all(b"\xEF\xBF\xBC\xEF\xBF\xBC\xEF\xBF\xBC\n")?;  // Line 1: ���
    txt_file.write_all(b"\xEF\xBF\xBC\xEF\xBF\xBC\xEF\xBF\xBC\xEF\xBF\xBC\n")?;  // Line 2: ����
    txt_file.write_all(b"AB \n")?;  // Line 3: AB + space

    println!("    -> {}", pdf_path);
    println!("    -> {}", txt_path);

    Ok(())
}
