//! Generate unmapped-glyphs.pdf test fixture
//!
//! Creates a PDF with custom glyph names that are not in the Adobe Glyph List
//! and have no ToUnicode mapping, triggering GLYPH_UNMAPPED diagnostics.

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating unmapped-glyphs.pdf...");

    let objects = vec![
        // Catalog
        b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj".to_vec(),

        // Pages
        b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj".to_vec(),

        // Page
        b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Resources <<\n/Font <<\n/F1 5 0 R\n>>\n>>\n/Contents 4 0 R\n>>\nendobj".to_vec(),

        // Content stream: <00010203> Tj (byte codes 0-3 for 4 unmapped glyphs)
        b"4 0 obj\n<<\n/Length 44\n>>\nstream\nBT\n/F1 12 Tf\n50 700 Td\n<00010203> Tj\nET\nendstream\nendobj".to_vec(),

        // Font with custom encoding
        // Uses 4 distinct custom glyph names not in AGL:
        // /CustomAlpha, /CustomBeta, /CustomGamma, /CustomDelta
        b"5 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /UnmappedGlyphs\n/Encoding <<\n/Type /Encoding\n/Differences [0 /CustomAlpha /CustomBeta /CustomGamma /CustomDelta]\n>>\n>>\nendobj".to_vec(),
    ];

    let pdf = build_pdf(objects);

    let mut file = File::create("tests/fixtures/encoding/unmapped-glyphs.pdf")?;
    file.write_all(&pdf)?;

    // Create expected output file (4 U+FFFD replacement characters)
    let mut txt_file = File::create("tests/fixtures/encoding/unmapped-glyphs.txt")?;
    txt_file.write_all("\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}".as_bytes())?;

    println!("✓ unmapped-glyphs.pdf created successfully!");
    println!("  -> Contains 4 unmapped glyphs: /CustomAlpha, /CustomBeta, /CustomGamma, /CustomDelta");
    println!("  -> Expected output: <U+FFFD><U+FFFD><U+FFFD><U+FFFD>");

    Ok(())
}

/// Build a complete PDF with proper xref table and trailer
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
    pdf.extend(b"0000000000 65535 f \n");

    for offset in offsets {
        pdf.extend(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    // Build trailer
    pdf.extend(b"trailer\n");
    pdf.extend(b"<<\n");
    pdf.extend(b"/Size ");
    pdf.extend(format!("{}", objects.len() + 1).as_bytes());
    pdf.extend(b"\n");
    pdf.extend(b"/Root 1 0 R\n");
    pdf.extend(b">>\n");

    // Start of xref
    pdf.extend(b"startxref\n");
    pdf.extend(format!("{}\n", xref_start).as_bytes());

    // EOF marker
    pdf.extend(b"%%EOF\n");

    pdf
}
