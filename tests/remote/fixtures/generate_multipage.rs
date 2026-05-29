//! Generate a multi-page PDF fixture for bandwidth testing.
//!
//! This script creates a 100-page PDF with ~10 KB per page (total ~1 MB).
//! Each page contains text content that can be extracted for testing.
//!
//! Usage: cargo run --bin generate_multipage

use std::fs::File;
use std::io::Write;

fn main() -> std::io::Result<()> {
    let page_count = 100;
    let content_per_page = 10000; // ~10 KB per page

    let mut pdf = String::new();

    // PDF Header
    pdf.push_str("%PDF-1.4\n");
    pdf.push_str("% комментариев\n");
    pdf.push_str("1 0 obj\n");
    pdf.push_str("<< /Type /Catalog /Pages 2 0 R >>\n");
    pdf.push_str("endobj\n");

    // Pages object
    pdf.push_str("2 0 obj\n");
    pdf.push_str("<< /Type /Pages /Kids [ ");
    for i in 0..page_count {
        pdf.push_str(&format!("{} 0 R ", 3 + i * 2));
    }
    pdf.push_str(&format!("] /Count {} >>\n", page_count));
    pdf.push_str("endobj\n");

    // Generate pages and content streams
    let mut current_offset = pdf.len();
    let mut xref_entries = vec![(0u64, 65535u16)]; // Entry 0 is always free

    xref_entries.push((current_offset as u64, 0)); // Object 1
    current_offset += pdf.len() - current_offset;
    xref_entries.push((current_offset as u64, 0)); // Object 2

    for i in 0..page_count {
        // Page object
        let page_obj_num = 3 + i * 2;
        let content_obj_num = 4 + i * 2;

        pdf.push_str(&format!("{} 0 obj\n", page_obj_num));
        pdf.push_str("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 1000 0 R >> >> /Contents ");
        pdf.push_str(&format!("{} 0 R ", content_obj_num));
        pdf.push_str(">>\n");
        pdf.push_str("endobj\n");

        xref_entries.push((current_offset as u64, 0));
        current_offset = pdf.len();

        // Content stream object
        pdf.push_str(&format!("{} 0 obj\n", content_obj_num));
        pdf.push_str(&format!("<< /Length {} >>\n", content_per_page));
        pdf.push_str("stream\n");

        // Generate page content
        let content = generate_page_content(i + 1, content_per_page);
        pdf.push_str(&content);
        pdf.push_str("endstream\n");
        pdf.push_str("endobj\n");

        xref_entries.push((current_offset as u64, 0));
        current_offset = pdf.len();
    }

    // Font object
    pdf.push_str("1000 0 obj\n");
    pdf.push_str("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\n");
    pdf.push_str("endobj\n");
    xref_entries.push((current_offset as u64, 0));
    current_offset = pdf.len();

    // xref table
    let xref_offset = current_offset;
    pdf.push_str("xref\n");
    pdf.push_str(&format!("0 {}\n", xref_entries.len()));
    for entry in &xref_entries {
        pdf.push_str(&format!("{:010} {:05} f \n", entry.0, entry.1));
    }

    // Trailer
    pdf.push_str("trailer\n");
    pdf.push_str(&format!("<< /Size {} /Root 1 0 R >>\n", xref_entries.len()));
    pdf.push_str(&format!("startxref\n{}\n", xref_offset));
    pdf.push_str("%%EOF\n");

    // Write to file
    let output_path = "tests/remote/fixtures/multipage-100.pdf";
    let mut file = File::create(output_path)?;
    file.write_all(pdf.as_bytes())?;

    println!("Generated {} with {} pages (~{} bytes)", output_path, page_count, pdf.len());

    Ok(())
}

/// Generate content for a single page.
fn generate_page_content(page_num: usize, target_length: usize) -> String {
    let mut content = String::new();
    content.push_str("BT\n");
    content.push_str("/F1 12 Tf\n");

    let mut y = 700;
    let mut x = 50;

    let text_lines = vec![
        format!("Page {}", page_num),
        "This is a test PDF page for bandwidth testing.".to_string(),
        "Each page contains approximately 10 KB of text content.".to_string(),
        "The purpose is to verify that partial extraction uses Range requests.".to_string(),
        "Only the requested pages should be downloaded from the server.".to_string(),
        "This test validates the HTTP Range source implementation.".to_string(),
        "".to_string(),
    ];

    let mut current_length = content.len();

    while current_length < target_length {
        for line in &text_lines {
            if current_length >= target_length {
                break;
            }

            content.push_str(&format!("{} {} Td ({}) Tj\n", x, y, line));
            y -= 14;

            if y < 50 {
                y = 700;
                x += 200;
            }

            current_length = content.len();
        }
    }

    content.push_str("ET\n");
    content
}
