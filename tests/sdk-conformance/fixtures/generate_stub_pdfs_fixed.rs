#!/usr/bin/env rust-script
//! Generate minimal valid PDF files for conformance testing.
//!
//! This script creates stub PDF fixtures with valid xref tables and structure
//! for SDK conformance testing. Each PDF is a minimal but valid PDF document.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn create_minimal_pdf(path: &Path, text: &str, title: &str) -> std::io::Result<()> {
    let content = format!(
        r#"BT
/F1 12 Tf
50 700 Td
({}) Tj
ET
"#,
        text
    );

    // Build the PDF incrementally to calculate offsets correctly
    let mut pdf_data = String::new();

    // Add header
    pdf_data.push_str("%PDF-1.4\n");

    // Catalog (obj 1)
    let catalog_offset = pdf_data.len();
    pdf_data.push_str(&format!(
        "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/Title ({})\n>>\nendobj\n",
        title
    ));

    // Pages tree (obj 2)
    let pages_offset = pdf_data.len();
    pdf_data.push_str(
        "2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1>>\nendobj\n"
    );

    // Page (obj 3)
    let page_offset = pdf_data.len();
    pdf_data.push_str(
        "3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents 4 0 R\n/Resources <<\n/Font <<\n/F1 5 0 R\n>>\n>>\n>>\nendobj\n"
    );

    // Content stream (obj 4)
    let content_offset = pdf_data.len();
    pdf_data.push_str(&format!(
        "4 0 obj\n<<\n/Length {}>>\nstream\n{}\nendstream\nendobj\n",
        content.len(),
        content
    ));

    // Font (obj 5)
    let font_offset = pdf_data.len();
    pdf_data.push_str(
        "5 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n"
    );

    // Xref table
    let xref_offset = pdf_data.len();
    pdf_data.push_str("xref\n");
    pdf_data.push_str("0 6\n");
    pdf_data.push_str("0000000000 65535 f \n");
    pdf_data.push_str(&format!("{:010} 00000 n \n", catalog_offset));
    pdf_data.push_str(&format!("{:010} 00000 n \n", pages_offset));
    pdf_data.push_str(&format!("{:010} 00000 n \n", page_offset));
    pdf_data.push_str(&format!("{:010} 00000 n \n", content_offset));
    pdf_data.push_str(&format!("{:010} 00000 n \n", font_offset));

    // Trailer
    pdf_data.push_str("trailer\n<<\n/Size 6\n/Root 1 0 R\n>>\n");
    pdf_data.push_str(&format!("startxref\n{}\n", xref_offset));
    pdf_data.push_str("%%EOF\n");

    let pdf = pdf_data;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(pdf.as_bytes())?;
    Ok(())
}

fn create_multi_page_pdf(path: &Path, num_pages: usize, title: &str) -> std::io::Result<()> {
    let mut body = String::from("%PDF-1.4\n");
    let mut offsets = std::collections::HashMap::new();
    let mut obj_num = 1;

    // Catalog (obj 1)
    offsets.insert(obj_num, body.len());
    body.push_str(&format!(
        "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/Title ({})\n>>\nendobj\n",
        title
    ));
    obj_num += 1;

    // Pages tree (obj 2)
    let kids: Vec<String> = (0..num_pages).map(|i| format!("{} 0 R", 3 + i * 2)).collect();
    offsets.insert(obj_num, body.len());
    body.push_str(&format!(
        "2 0 obj\n<<\n/Type /Pages\n/Kids [{}]\n/Count {}>>\nendobj\n",
        kids.join(" "),
        num_pages
    ));
    obj_num += 1;

    // Page objects and their contents
    for i in 0..num_pages {
        let content = format!("BT\n/F1 12 Tf\n50 700 Td\n(Page {}) Tj\nET\n", i + 1);

        // Content stream
        offsets.insert(obj_num, body.len());
        body.push_str(&format!(
            "{} 0 obj\n<<\n/Length {}>>\nstream\n{}\nendstream\nendobj\n",
            obj_num,
            content.len(),
            content
        ));
        obj_num += 1;

        // Page object
        let content_obj = obj_num - 1;
        let font_obj = 2 * num_pages + 3;
        offsets.insert(obj_num, body.len());
        body.push_str(&format!(
            "{} 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents {} 0 R\n/Resources <<\n/Font <<\n/F1 {} 0 R\n>>\n>>\n>>\nendobj\n",
            obj_num,
            content_obj,
            font_obj
        ));
        obj_num += 1;
    }

    // Font object
    offsets.insert(obj_num, body.len());
    body.push_str(
        &format!(
            "{} 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n",
            obj_num
        )
    );
    obj_num += 1;

    let xref_start = body.len();

    // Build xref table
    body.push_str("xref\n");
    body.push_str(&format!("0 {}\n", obj_num));
    body.push_str("0000000000 65535 f \n");

    for i in 1..obj_num {
        let offset = offsets.get(&i).unwrap();
        body.push_str(&format!("{:010} 00000 n \n", offset));
    }

    body.push_str(&format!(
        "trailer\n<<\n/Size {}\n/Root 1 0 R\n>>\nstartxref\n{}\n%%EOF\n",
        obj_num,
        xref_start
    ));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(body.as_bytes())?;
    Ok(())
}

fn create_receipt_json(path: &Path, valid: bool) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    let content = if valid {
        r#"{"fingerprint": "stub-valid", "signature": "valid-signature"}"#
    } else {
        r#"{"fingerprint": "stub-tampered", "signature": "invalid-signature"}"#
    };
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let fixture_path = Path::new("tests/sdk-conformance/fixtures");

    println!("Creating stub fixtures in: {:?}", fixture_path);

    // Scientific paper fixtures
    for i in 1..=14 {
        let path = fixture_path.join(format!("scientific_paper/{:02}.pdf", i));
        create_minimal_pdf(&path, &format!("Scientific Paper {}", i), &format!("Paper {}", i))?;
        println!("Created scientific_paper/{:02}.pdf", i);
    }

    // Misc fixtures
    for i in 1..=3 {
        let path = fixture_path.join(format!("misc/{:02}.pdf", i));
        create_minimal_pdf(&path, &format!("Misc {}", i), &format!("Misc {}", i))?;
        println!("Created misc/{:02}.pdf", i);
    }

    // Invoice fixtures
    for i in 1..=1 {
        let path = fixture_path.join(format!("invoice/{:02}.pdf", i));
        create_minimal_pdf(&path, &format!("Invoice {}", i), &format!("Invoice {}", i))?;
        println!("Created invoice/{:02}.pdf", i);
    }

    // Contract fixtures
    for i in 1..=1 {
        let path = fixture_path.join(format!("contract/{:02}.pdf", i));
        create_minimal_pdf(&path, &format!("AGREEMENT\n\nContract {}", i), &format!("Contract {}", i))?;
        println!("Created contract/{:02}.pdf", i);
    }

    // Encrypted PDF
    let path = fixture_path.join("encrypted/encrypted.pdf");
    create_minimal_pdf(&path, "Encrypted Content", "Encrypted PDF")?;
    println!("Created encrypted/encrypted.pdf");

    // Fillable form
    let path = fixture_path.join("fillable-form/form.pdf");
    create_minimal_pdf(&path, "Form Content", "Fillable Form")?;
    println!("Created fillable-form/form.pdf");

    // Mixed content
    let path = fixture_path.join("mixed/mixed.pdf");
    create_multi_page_pdf(&path, 2, "Mixed Content Document")?;
    println!("Created mixed/mixed.pdf");

    // Large documents
    for pages in [50, 100] {
        let path = fixture_path.join(format!("large/{}pages.pdf", pages));
        create_multi_page_pdf(&path, pages, &format!("{} Page Document", pages))?;
        println!("Created large/{}pages.pdf", pages);
    }

    // Vertical writing
    let path = fixture_path.join("vertical/vertical.pdf");
    create_minimal_pdf(&path, "Vertical", "Vertical Text Document")?;
    println!("Created vertical/vertical.pdf");

    // Code
    let path = fixture_path.join("code/code.pdf");
    create_minimal_pdf(&path, "function test() {\n  return true;\n}", "Code Sample")?;
    println!("Created code/code.pdf");

    // XMP metadata
    let path = fixture_path.join("xmp/xmp-metadata.pdf");
    create_minimal_pdf(&path, "XMP Document", "XMP Metadata Document")?;
    println!("Created xmp/xmp-metadata.pdf");

    // Receipts
    create_receipt_json(&fixture_path.join("receipts/valid-receipt.receipt.json"), true)?;
    create_receipt_json(&fixture_path.join("receipts/tampered-receipt.receipt.json"), false)?;
    create_minimal_pdf(&fixture_path.join("receipts/valid-receipt.pdf"), "Valid Receipt", "Valid Receipt")?;
    create_minimal_pdf(&fixture_path.join("receipts/tampered-receipt.pdf"), "Tampered Receipt", "Tampered Receipt")?;
    println!("Created receipt fixtures");

    // Broken/corrupt PDF
    let path = fixture_path.join("broken/corrupt.pdf");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(&path)?;
    file.write_all(b"%PDF-1.4\nThis is intentionally broken\n%%EOF")?;
    println!("Created broken/corrupt.pdf");

    println!("\nAll stub fixtures created successfully!");
    Ok(())
}
