//! Generate a linearized PDF fixture for hint stream testing.
//!
//! This script creates a small linearized PDF with a hint stream.
//! The hint stream allows readers to predict page offsets for prefetching.
//!
//! Usage: cargo run --bin generate_linearized

use std::fs::File;
use std::io::Write;

fn main() -> std::io::Result<()> {
    let page_count = 10;

    let mut pdf = String::new();

    // PDF Header
    pdf.push_str("%PDF-1.4\n");
    pdf.push_str("% комментариев\n");

    // Linearized dictionary (object 1)
    // This tells readers the document is linearized and where the first page ends
    let linearized_dict = format!(
        "1 0 obj\n\
        << /Linearized 1 /L {} /E {} /N {} /H [ {} {} {} {} ] /O 2 0 R /T 3 0 R >>\n\
        endobj\n",
        10000, // Total file length (placeholder)
        5000,  // End of first page (placeholder)
        page_count,
        1234, 1234, 1234, 1234 // Hint table offsets (placeholders)
    );
    let linearized_offset = pdf.len();
    pdf.push_str(&linearized_dict);

    // Hint stream (object 2) - contains page offset information
    // In a real linearized PDF, this would have binary data with offset tables
    let hint_stream = format!(
        "2 0 obj\n\
        << /Length {} >>\n\
        stream\n\
        \x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\n\
        \x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\n\
        endstream\n\
        endobj\n",
        32
    );
    let hint_offset = pdf.len();
    pdf.push_str(&hint_stream);

    // Document catalog (object 3)
    pdf.push_str("3 0 obj\n");
    pdf.push_str("<< /Type /Catalog /Pages 4 0 R >>\n");
    pdf.push_str("endobj\n");

    // Pages object
    pdf.push_str("4 0 obj\n");
    pdf.push_str("<< /Type /Pages /Kids [ ");
    for i in 0..page_count {
        pdf.push_str(&format!("{} 0 R ", 5 + i));
    }
    pdf.push_str(&format!("] /Count {} >>\n", page_count));
    pdf.push_str("endobj\n");

    // Generate pages and content streams
    let mut current_offset = pdf.len();
    let mut xref_entries = vec![(0u64, 65535u16)]; // Entry 0 is always free

    xref_entries.push((linearized_offset as u64, 0)); // Object 1
    xref_entries.push((hint_offset as u64, 0));       // Object 2
    xref_entries.push((current_offset as u64, 0));    // Object 3
    current_offset = pdf.len();
    xref_entries.push((current_offset as u64, 0));    // Object 4
    current_offset = pdf.len();

    for i in 0..page_count {
        let page_obj_num = 5 + i;
        let content_obj_num = 5 + page_count + i;

        pdf.push_str(&format!("{} 0 obj\n", page_obj_num));
        pdf.push_str("<< /Type /Page /Parent 4 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 1000 0 R >> >> /Contents ");
        pdf.push_str(&format!("{} 0 R ", content_obj_num));
        pdf.push_str(">>\n");
        pdf.push_str("endobj\n");

        xref_entries.push((current_offset as u64, 0));
        current_offset = pdf.len();

        // Content stream object
        pdf.push_str(&format!("{} 0 obj\n", content_obj_num));
        pdf.push_str("<< /Length 100 >>\n");
        pdf.push_str("stream\n");
        pdf.push_str(&format!("BT\n/F1 12 Tf\n100 {} Td (Page {} content) Tj\nET\n", 700 - (i % 10) * 14, i + 1));
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
    pdf.push_str(&format!("<< /Size {} /Root 3 0 R >>\n", xref_entries.len()));
    pdf.push_str(&format!("startxref\n{}\n", xref_offset));
    pdf.push_str("%%EOF\n");

    // Write to file
    let output_path = "tests/remote/fixtures/linearized-10.pdf";
    let mut file = File::create(output_path)?;
    file.write_all(pdf.as_bytes())?;

    println!("Generated {} with {} pages (~{} bytes)", output_path, page_count, pdf.len());
    println!("Linearized dict at offset: {}", linearized_offset);
    println!("Hint stream at offset: {}", hint_offset);

    Ok(())
}
