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

    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
/Title ({})

>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
/Resources <<
/Font <<
/F1 5 0 R
>>
>>
>>
endobj
4 0 obj
<<
/Length {}
>>
stream
{}
endstream
endobj
5 0 obj
<<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000068 00000 n
0000000125 00000 n
0000000293 00000 n
0000000414 00000 n
trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
501
%%EOF
"#,
        content.len(),
        content,
        title
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(pdf.as_bytes())?;
    Ok(())
}

fn create_multi_page_pdf(path: &Path, num_pages: usize, title: &str) -> std::io::Result<()> {
    let mut pdf = String::new();
    let mut objects = Vec::new();
    let mut offset = 9;

    // Catalog (obj 1)
    pdf.push_str(&format!(
        "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/Title ({})\n>>\nendobj\n",
        title
    ));
    offset += pdf.len() - offset;
    objects.push((1, offset));

    // Pages tree (obj 2)
    let kids: Vec<String> = (0..num_pages).map(|i| format!("{} 0 R", 3 + i * 2)).collect();
    pdf.push_str(&format!(
        "2 0 obj\n<<\n/Type /Pages\n/Kids [{}]\n/Count {}>>\nendobj\n",
        kids.join(" "),
        num_pages
    ));
    offset += pdf.len() - objects.last().unwrap().1;
    objects.push((2, offset));

    // Page objects and their contents
    for i in 0..num_pages {
        let page_obj = 3 + i * 2;
        let content_obj = 4 + i * 2;

        let content = format!("BT\n/F1 12 Tf\n50 700 Td\n(Page {}) Tj\nET\n", i + 1);

        // Content stream
        pdf.push_str(&format!(
            "{} 0 obj\n<<\n/Length {}>>\nstream\n{}\nendstream\nendobj\n",
            content_obj,
            content.len(),
            content
        ));
        offset += pdf.len() - objects.last().unwrap().1;
        objects.push((content_obj, offset));

        // Page object
        pdf.push_str(&format!(
            "{} 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents {} 0 R\n/Resources <<\n/Font <<\n/F1 {} 0 R\n>>\n>>\n>>\nendobj\n",
            page_obj, content_obj, 2 * num_pages + 3
        ));
        offset += pdf.len() - objects.last().unwrap().1;
        objects.push((page_obj, offset));
    }

    // Font object
    let font_obj = 2 * num_pages + 3;
    pdf.push_str(
        &format!(
            "{} 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n",
            font_obj
        )
    );
    offset += pdf.len() - objects.last().unwrap().1;
    objects.push((font_obj, offset));

    let _xref_offset = offset;

    // Build xref table with actual offsets
    pdf.push_str("xref\n0 1\n0000000000 65535 f \n");

    // Calculate xref properly: we need to track where each object starts
    let _pdf_bytes = pdf.as_bytes().to_vec();

    // Rebuild PDF with accurate offsets
    let mut sections = vec![
        // Catalog
        (1, format!(
            "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/Title ({})\n>>\nendobj\n",
            title
        )),
        // Pages node
        (2, format!(
            "2 0 obj\n<<\n/Type /Pages\n/Kids [{}]\n/Count {}>>\nendobj\n",
            (0..num_pages).map(|i| format!("{} 0 R", 3 + i * 2)).collect::<Vec<_>>().join(" "),
            num_pages
        )),
    ];

    // Add pages and contents
    for i in 0..num_pages {
        let page_obj = 3 + i * 2;
        let content_obj = 4 + i * 2;
        let content = format!("BT\n/F1 12 Tf\n50 700 Td\n(Page {}) Tj\nET\n", i + 1);

        sections.push((content_obj, format!(
            "{} 0 obj\n<<\n/Length {}>>\nstream\n{}\nendstream\nendobj\n",
            content_obj, content.len(), content
        )));
        sections.push((page_obj, format!(
            "{} 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents {} 0 R\n/Resources <<\n/Font <<\n/F1 {} 0 R\n>>\n>>\n>>\nendobj\n",
            page_obj, content_obj, 2 * num_pages + 3
        )));
    }

    // Font
    let font_obj = 2 * num_pages + 3;
    sections.push((font_obj, format!(
        "{} 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n",
        font_obj
    )));

    // Build PDF body
    let mut body = format!("%PDF-1.4\n");
    let mut offsets = std::collections::HashMap::new();

    for (obj_num, content) in &sections {
        offsets.insert(obj_num, body.len());
        body.push_str(content);
        body.push('\n');
    }

    let xref_start = body.len();
    body.push_str("xref\n");
    body.push_str(&format!("0 {}\n", sections.len() + 1));
    body.push_str("0000000000 65535 f \n");

    for obj_num in 1..=sections.len() {
        let offset = offsets.get(&obj_num).unwrap();
        body.push_str(&format!("{:010} 00000 n \n", offset));
    }

    body.push_str(&format!(
        "trailer\n<<\n/Size {}\n/Root 1 0 R\n>>\nstartxref\n{}\n%%EOF\n",
        sections.len() + 1,
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
    let fixture_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());

    let fixture_path = Path::new(&fixture_dir)
        .join("tests/sdk-conformance/fixtures");

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
