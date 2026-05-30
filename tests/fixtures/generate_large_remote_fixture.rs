//! Generate a 100-page PDF fixture for remote source testing.
//!
//! This creates a multi-page PDF where each page has unique content,
//! allowing us to verify that only specific pages are fetched during
//! Range request testing.

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_path = "tests/fixtures/remote_100page.pdf";

    let mut pdf = String::new();

    // PDF header
    pdf.push_str("%PDF-1.4\n");

    // Track object offsets
    let mut offsets: Vec<u64> = Vec::new();
    let mut current_offset = pdf.len() as u64;

    // Catalog object (1 0 obj)
    offsets.push(current_offset);
    pdf.push_str("1 0 obj\n");
    pdf.push_str("<< /Type /Catalog\n");
    pdf.push_str("   /Pages 2 0 R\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Pages object (2 0 obj) - we'll update this with page count later
    current_offset = pdf.len() as u64;
    offsets.push(current_offset);
    pdf.push_str("2 0 obj\n");
    pdf.push_str("<< /Type /Pages\n");
    pdf.push_str(format!("   /Count {}\n", 100).as_str());
    pdf.push_str("   /Kids [");
    for i in 3..103 {
        pdf.push_str(format!("{} 0 R ", i).as_str());
    }
    pdf.push_str("]\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // Create 100 page objects (3-102)
    // Also create 100 content streams (103-202)
    let page_objects_start = 3u64;
    let content_objects_start = 103u64;

    for page_num in 1..=100 {
        // Page object
        current_offset = pdf.len() as u64;
        offsets.push(current_offset);
        pdf.push_str(format!("{} 0 obj\n", page_objects_start + page_num - 1).as_str());
        pdf.push_str("<< /Type /Page\n");
        pdf.push_str("   /Parent 2 0 R\n");
        pdf.push_str("   /MediaBox [ 0 0 612 792 ]\n");
        pdf.push_str("   /Contents ");
        pdf.push_str(format!("{} 0 R\n", content_objects_start + page_num - 1).as_str());
        pdf.push_str("   /Resources << /Font << /F1 203 0 R >> >>\n");
        pdf.push_str(">>\n");
        pdf.push_str("endobj\n");

        // Content stream with page-specific text
        current_offset = pdf.len() as u64;
        offsets.push(current_offset);
        pdf.push_str(format!("{} 0 obj\n", content_objects_start + page_num - 1).as_str());

        // Create a content stream that's unique per page
        // Each content stream is about 50-100 KB for a total of ~5-10 MB PDF
        let content_lines = 400;  // Fixed size per page for consistency

        pdf.push_str("<< /Length 0 >>\nstream\n");

        // Write some PDF content operations
        pdf.push_str("BT\n");
        pdf.push_str("/F1 8 Tf\n");
        pdf.push_str("50 780 Td\n");
        pdf.push_str(format!("(Page {} of Remote Test PDF - 100 pages for Range request testing) Tj\n", page_num).as_str());

        // Add substantial content to make each page ~50-100 KB
        for line in 1..=content_lines {
            let y = 780 - (line as i32 * 2);
            if y < 50 {  // Prevent negative Y coordinates
                pdf.push_str(format!("50 {} Td\n", 50).as_str());
            } else {
                pdf.push_str(format!("50 {} Td\n", y).as_str());
            }
            // Long text per line - multiple text operations per line
            let long_text = format!(
                "(Line {} page {} Remote Test PDF Range Request Testing Unique Marker Data Content Extraction Partial Fetch Bandwidth Verification {}) Tj\n",
                line, page_num, page_num * 10000 + line
            );
            pdf.push_str(&long_text);
        }

        pdf.push_str("ET\n");
        pdf.push_str("endstream\n");
        pdf.push_str("endobj\n");
    }

    // Font object (203 0 obj)
    current_offset = pdf.len() as u64;
    offsets.push(current_offset);
    pdf.push_str("203 0 obj\n");
    pdf.push_str("<< /Type /Font\n");
    pdf.push_str("   /Subtype /Type1\n");
    pdf.push_str("   /BaseFont /Helvetica\n");
    pdf.push_str(">>\n");
    pdf.push_str("endobj\n");

    // XRef table
    let xref_offset = pdf.len() as u64;
    pdf.push_str("xref\n");
    pdf.push_str("0 204\n");
    pdf.push_str("0000000000 65535 f \n");

    for &offset in &offsets {
        pdf.push_str(format!("{:010} 00000 n \n", offset).as_str());
    }

    // Trailer
    pdf.push_str("trailer\n");
    pdf.push_str("<< /Size 204\n");
    pdf.push_str("   /Root 1 0 R\n");
    pdf.push_str(">>\n");

    // StartXRef
    pdf.push_str(format!("startxref\n{}\n", xref_offset).as_str());
    pdf.push_str("%%EOF\n");

    // Write to file
    let mut file = File::create(output_path)?;
    file.write_all(pdf.as_bytes())?;
    file.flush()?;

    // Get file size
    let metadata = std::fs::metadata(output_path)?;
    let size_kb = metadata.len() / 1024;

    println!("Created {} ({} KB)", output_path, size_kb);

    Ok(())
}
