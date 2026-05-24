//! Generate a minimal valid tagged PDF for testing Phase 7.1.4 coverage check.
//!
//! This creates a PDF with:
//! - /MarkInfo /Suspects true
//! - StructTree with ParentTree
//! - MCID-based content association
//!
//! The PDF is minimal but valid, using manual byte offsets for reliability.

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate fixture 1: Suspects true, low coverage -> XY-cut fallback
    generate_pdf("tests/fixtures/tagged-suspects-true.pdf", true, 6, 10)?;

    // Generate fixture 2: Suspects false, low coverage -> trust StructTree
    generate_pdf("tests/fixtures/tagged-suspects-false.pdf", false, 5, 10)?;

    // Generate fixture 3: Suspects true, high coverage -> trust StructTree
    generate_pdf("tests/fixtures/tagged-suspects-true-high-coverage.pdf", true, 19, 20)?;

    Ok(())
}

fn generate_pdf(path: &str, suspects: bool, num_claimed: usize, num_total: usize) -> Result<(), Box<dyn std::error::Error>> {
    let mut pdf = String::new();

    // PDF header
    pdf.push_str("%PDF-1.7\n");

    // Object 1: Catalog
    pdf.push_str("1 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Type /Catalog\n");
    pdf.push_str("/Pages 2 0 R\n");
    pdf.push_str("/MarkInfo <<\n");
    pdf.push_str("  /Marked true\n");
    pdf.push_str(format!("  /Suspects {}\n", if suspects { "true" } else { "false" }).as_str());
    pdf.push_str(">>\n");
    pdf.push_str("/StructTreeRoot 3 0 R\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Object 2: Pages
    pdf.push_str("2 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Type /Pages\n");
    pdf.push_str("/Kids [4 0 R]\n");
    pdf.push_str("/Count 1\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Object 3: StructTreeRoot
    pdf.push_str("3 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Type /StructTreeRoot\n");
    pdf.push_str("/K [5 0 R]\n");
    pdf.push_str("/ParentTree 6 0 R\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Object 4: Page
    pdf.push_str("4 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Type /Page\n");
    pdf.push_str("/Parent 2 0 R\n");
    pdf.push_str("/MediaBox [0 0 612 792]\n");
    pdf.push_str("/Contents 7 0 R\n");
    pdf.push_str("/StructParents 0\n");
    pdf.push_str("/Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Object 5: StructElem (paragraph)
    pdf.push_str("5 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Type /StructElem\n");
    pdf.push_str("/S /P\n");
    pdf.push_str("/K [");
    for i in 0..num_total {
        pdf.push_str(&format!("{} ", i));
    }
    pdf.push_str("]\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Object 6: ParentTree (number tree with /Nums array)
    pdf.push_str("6 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Nums [\n");
    pdf.push_str("0 [");
    for i in 0..num_total {
        if i < num_claimed {
            pdf.push_str(" 5 0 R");
        } else {
            pdf.push_str(" null");
        }
        if i < num_total - 1 {
            pdf.push(' ');
        }
    }
    pdf.push_str(" ]\n");
    pdf.push_str("]\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Object 7: Content stream
    pdf.push_str("7 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Length 44\n");
    pdf.push_str(">>\n");
    pdf.push_str("stream\n");
    pdf.push_str("BT\n");
    pdf.push_str("/F1 12 Tf\n");
    pdf.push_str("100 700 Td\n");
    pdf.push_str("(Test) Tj\n");
    pdf.push_str("ET\n");
    pdf.push_str("endstream\n");
    pdf.push_str("endobj\n");

    // Calculate xref offset (current position + "xref\n" + start of table)
    let xref_offset = pdf.len() + 5; // +5 for "xref\n"

    // Build xref table
    pdf.push_str("xref\n");
    pdf.push_str("0 8\n");
    pdf.push_str("0000000000 65535 f \n");

    // We need to calculate byte offsets for each object
    // Let's do this by building the PDF first, then computing offsets
    let pdf_bytes = pdf.as_bytes();
    let mut offsets = Vec::new();
    let mut current = 0;

    // Find each object offset by searching for "N 0 obj"
    for n in 1..=7 {
        let pattern = format!("{} 0 obj\n", n);
        if let Some(pos) = pdf.find(&pattern) {
            offsets.push(pos);
        }
    }

    // Add xref entries
    for (i, offset) in offsets.iter().enumerate() {
        pdf.push_str(&format!("{:010} 00000 n \n", offset));
    }

    // Trailer
    pdf.push_str("trailer\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Size 8\n");
    pdf.push_str("/Root 1 0 R\n");
    pdf.push_str(">>\n");

    // startxref
    pdf.push_str(&format!("startxref\n{}\n", xref_offset));

    // EOF
    pdf.push_str("%%EOF\n");

    // Write to file
    let mut file = File::create(path)?;
    file.write_all(pdf.as_bytes())?;

    eprintln!("Created: {}", path);
    eprintln!("  /Suspects: {}", suspects);
    eprintln!("  Coverage: {}/{} MCIDs claimed", num_claimed, num_total);

    Ok(())
}
