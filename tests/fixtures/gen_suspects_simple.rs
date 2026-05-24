//! Simple Rust-based generator for Suspects test fixtures.
//!
//! Generates minimal valid tagged PDFs with:
//! - /MarkInfo /Suspects flag
//! - StructTree with ParentTree
//! - MCID marked content in content streams

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating Suspects test fixtures...");

    // Fixture 1: Suspects true, 60% coverage (6/10 claimed) -> fallback to XY-cut
    write_fixture("tagged-suspects-true.pdf", true, 6, 10)?;

    // Fixture 2: Suspects false, 50% coverage (5/10 claimed) -> trust StructTree
    write_fixture("tagged-suspects-false.pdf", false, 5, 10)?;

    // Fixture 3: Suspects true, 95% coverage (19/20 claimed) -> trust StructTree
    write_fixture("tagged-suspects-true-high-coverage.pdf", true, 19, 20)?;

    println!("All fixtures generated!");
    Ok(())
}

fn write_fixture(
    path: &str,
    suspects: bool,
    num_claimed: usize,
    num_total: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Build the PDF content
    let mut pdf = String::new();

    // Header
    pdf.push_str("%PDF-1.7\n");

    // Object 1: Catalog
    pdf.push_str("1 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Type /Catalog\n");
    pdf.push_str("/Pages 2 0 R\n");
    pdf.push_str("/MarkInfo <<\n");
    pdf.push_str("  /Marked true\n");
    pdf.push_str(&format!("  /Suspects {}\n", if suspects { "true" } else { "false" }));
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
    pdf.push_str("/Resources <<\n");
    pdf.push_str("/Font <<\n");
    pdf.push_str("/F1 <<\n");
    pdf.push_str("/Type /Font\n");
    pdf.push_str("/Subtype /Type1\n");
    pdf.push_str("/BaseFont /Helvetica\n");
    pdf.push_str(">>\n");
    pdf.push_str(">>\n");
    pdf.push_str(">>\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Object 5: StructElem (paragraph)
    let k_array: String = (0..num_total).map(|i| i.to_string()).collect::<Vec<_>>().join(" ");
    pdf.push_str("5 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Type /StructElem\n");
    pdf.push_str("/S /P\n");
    pdf.push_str(&format!("/K [{}]\n", k_array));
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Object 6: ParentTree
    pdf.push_str("6 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Nums [\n");
    pdf.push_str("0 [");
    for i in 0..num_total {
        if i < num_claimed {
            pdf.push_str("5 0 R");
        } else {
            pdf.push_str("null");
        }
        if i < num_total - 1 {
            pdf.push(' ');
        }
    }
    pdf.push_str("]\n");
    pdf.push_str("]\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Object 7: Content stream with MCID marked content
    let mut content = String::new();
    for i in 0..num_total {
        let y = 700 - i * 15;
        content.push_str(&format!(
            "BT\n/F1 12 Tf\n100 {} Td\n/MCID {} BDC\n(Test{}) Tj\nEMC\nET\n",
            y, i, i
        ));
    }
    let content_bytes = content.as_bytes();
    let content_len = content_bytes.len();

    pdf.push_str("7 0 obj\n");
    pdf.push_str("<<\n");
    pdf.push_str(&format!("/Length {}\n", content_len));
    pdf.push_str(">>\n");
    pdf.push_str("stream\n");
    pdf.push_str(&content);
    pdf.push_str("endstream\n");
    pdf.push_str("endobj\n");

    // Now we have all the content, calculate xref
    let pdf_bytes = pdf.as_bytes();
    let mut offsets = vec![0u64; 8]; // Objects 0-7

    // Find each object's offset by scanning the PDF string
    let pdf_clone = pdf.clone();
    for (obj_num, offset) in find_object_offsets(&pdf_clone) {
        if obj_num < 8 {
            offsets[obj_num] = offset;
        }
    }

    // Build xref table
    let xref_start = pdf_bytes.len() as u64;
    pdf.push_str("xref\n");
    pdf.push_str("0 8\n");
    pdf.push_str("0000000000 65535 f \n");
    for i in 1..=7 {
        pdf.push_str(&format!("{:010} 00000 n \n", offsets[i]));
    }

    // Build trailer
    pdf.push_str("trailer\n");
    pdf.push_str("<<\n");
    pdf.push_str("/Size 8\n");
    pdf.push_str("/Root 1 0 R\n");
    pdf.push_str(">>\n");
    pdf.push_str(&format!("startxref\n{}\n", xref_start));
    pdf.push_str("%%EOF\n");

    // Write to file
    let mut file = File::create(format!("tests/fixtures/{}", path))?;
    file.write_all(pdf.as_bytes())?;

    let coverage = (num_claimed as f64 / num_total as f64) * 100.0;
    println!("Created: {}", path);
    println!("  Suspects: {}, Coverage: {:.0}% ({}/{})",
        suspects, coverage, num_claimed, num_total);

    Ok(())
}

fn parse_obj_number(line: &str) -> Option<usize> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 && parts[1] == "0" && parts.get(2) == Some(&"obj") {
        parts[0].parse().ok()
    } else {
        None
    }
}

fn find_object_offsets(pdf: &str) -> Vec<(usize, u64)> {
    let mut offsets = Vec::new();
    let mut pos = 0u64;

    for line in pdf.lines() {
        if let Some(obj_num) = parse_obj_number(line) {
            offsets.push((obj_num, pos));
        }
        pos += line.len() as u64 + 1; // +1 for newline
    }

    offsets
}
