#!/usr/bin/env rust-script
//! Generate proper PDF fixtures for SDK conformance testing.
//!
//! Creates valid PDF documents with the actual content expected by the conformance tests.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Escape special characters in PDF strings
fn escape_pdf_string(text: &str) -> String {
    text.chars()
        .flat_map(|c| match c {
            '(' => vec!['\\', '('],
            ')' => vec!['\\', ')'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            c => vec![c],
        })
        .collect()
}

/// Create a PDF with multi-line content at specified y positions
fn create_content_pdf(path: &Path, lines: &[(f32, &str)], title: &str) -> std::io::Result<()> {
    let mut pdf = String::new();
    let mut offsets: Vec<usize> = Vec::new();

    // Header
    pdf.push_str("%PDF-1.4\n");

    // Object 1: Catalog
    offsets.push(pdf.len());
    pdf.push_str(&format!(
        "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/Title ({})\n>>\nendobj\n",
        escape_pdf_string(title)
    ));

    // Build content stream with multiple lines
    let mut content = String::new();
    for (y, line) in lines {
        content.push_str(&format!("BT\n/F1 12 Tf\n50 {} Td\n({}) Tj\nET\n", y, escape_pdf_string(line)));
    }

    // Object 2: Pages node
    offsets.push(pdf.len());
    pdf.push_str("2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1>>\nendobj\n");

    // Object 3: Page
    offsets.push(pdf.len());
    pdf.push_str(&format!(
        "3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents 4 0 R\n/Resources <<\n/Font <<\n/F1 5 0 R\n>>\n>>\n>>\nendobj\n"
    ));

    // Object 4: Content stream
    offsets.push(pdf.len());
    pdf.push_str(&format!(
        "4 0 obj\n<<\n/Length {}>>\nstream\n{}\nendstream\nendobj\n",
        content.len(),
        content
    ));

    // Object 5: Font
    offsets.push(pdf.len());
    pdf.push_str("5 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n");

    // Build xref table
    let xref_start = pdf.len();
    pdf.push_str("xref\n");
    pdf.push_str(&format!("0 {}\n", offsets.len() + 1));
    pdf.push_str("0000000000 65535 f \n");

    for &offset in &offsets {
        pdf.push_str(&format!("{:010} 00000 n \n", offset));
    }

    // Trailer
    pdf.push_str(&format!(
        "trailer\n<<\n/Size {}\n/Root 1 0 R\n>>\nstartxref\n{}\n%%EOF\n",
        offsets.len() + 1,
        xref_start
    ));

    // Write to file
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, pdf.as_bytes())?;
    Ok(())
}

/// Create a multi-page PDF
fn create_multi_page_pdf(path: &Path, pages: &[Vec<(f32, String)>], title: &str) -> std::io::Result<()> {
    let mut pdf = String::new();
    let mut offsets: Vec<usize> = Vec::new();

    // Header
    pdf.push_str("%PDF-1.4\n");

    // Object 1: Catalog
    offsets.push(pdf.len());
    pdf.push_str(&format!(
        "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/Title ({})\n>>\nendobj\n",
        escape_pdf_string(title)
    ));

    // Build page objects and content streams
    let mut page_refs = Vec::new();
    let font_obj_num = 3 + pages.len() * 2;

    for (page_idx, page_lines) in pages.iter().enumerate() {
        let content_obj = 3 + page_idx * 2;
        let page_obj = 4 + page_idx * 2;

        // Build content stream for this page
        let mut content = String::new();
        for (y, line) in page_lines {
            content.push_str(&format!("BT\n/F1 12 Tf\n50 {} Td\n({}) Tj\nET\n", y, escape_pdf_string(line)));
        }

        // Content stream
        offsets.push(pdf.len());
        pdf.push_str(&format!(
            "{} 0 obj\n<<\n/Length {}>>\nstream\n{}\nendstream\nendobj\n",
            content_obj,
            content.len(),
            content
        ));

        // Page object
        offsets.push(pdf.len());
        pdf.push_str(&format!(
            "{} 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents {} 0 R\n/Resources <<\n/Font <<\n/F1 {} 0 R\n>>\n>>\n>>\nendobj\n",
            page_obj,
            content_obj,
            font_obj_num
        ));

        page_refs.push(format!("{} 0 R", page_obj));
    }

    // Object 2: Pages node
    offsets.push(pdf.len());
    pdf.push_str(&format!(
        "2 0 obj\n<<\n/Type /Pages\n/Kids [{}]\n/Count {}>>\nendobj\n",
        page_refs.join(" "),
        pages.len()
    ));

    // Font object (last)
    offsets.push(pdf.len());
    pdf.push_str(&format!(
        "{} 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n",
        font_obj_num
    ));

    // Build xref table
    let xref_start = pdf.len();
    pdf.push_str("xref\n");
    pdf.push_str(&format!("0 {}\n", offsets.len() + 1));
    pdf.push_str("0000000000 65535 f \n");

    for &offset in &offsets {
        pdf.push_str(&format!("{:010} 00000 n \n", offset));
    }

    // Trailer
    pdf.push_str(&format!(
        "trailer\n<<\n/Size {}\n/Root 1 0 R\n>>\nstartxref\n{}\n%%EOF\n",
        offsets.len() + 1,
        xref_start
    ));

    // Write to file
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, pdf.as_bytes())?;
    Ok(())
}

/// Create a receipt JSON file
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
    // Try to find the workspace root by looking for Cargo.toml
    let mut fixture_path = Path::new("tests/sdk-conformance/fixtures");

    // If running from within fixtures directory, adjust path
    if std::env::current_dir().unwrap().ends_with("fixtures") {
        fixture_path = Path::new(".");
    } else if std::env::current_dir().unwrap().ends_with("sdk-conformance") {
        fixture_path = Path::new("fixtures");
    } else if std::env::current_dir().unwrap().ends_with("tests") {
        fixture_path = Path::new("sdk-conformance/fixtures");
    }

    println!("Creating proper fixtures in: {:?}", fixture_path);

    println!("Creating proper fixtures in: {:?}", fixture_path);

    // Scientific paper fixtures with actual academic content
    let scientific_papers: Vec<(usize, Vec<(f32, String)>)> = vec![
        (1, vec![
            (700.0, "Title: Analysis of Machine Learning Algorithms".to_string()),
            (680.0, "Abstract".to_string()),
            (660.0, "This paper presents a comprehensive analysis of various".to_string()),
            (640.0, "machine learning algorithms and their performance.".to_string()),
            (620.0, "Introduction".to_string()),
            (600.0, "Machine learning has become an essential tool in modern".to_string()),
            (580.0, "data science and artificial intelligence research.".to_string()),
        ]),
        (2, vec![
            (700.0, "Title: Unicode Character Analysis: α, β, γ, 中文, العربية".to_string()),
            (680.0, "Abstract".to_string()),
            (660.0, "This study examines Unicode character handling in text extraction.".to_string()),
            (640.0, "Introduction".to_string()),
            (620.0, "Unicode support is critical for international document processing.".to_string()),
        ]),
        (3, vec![
            (700.0, "Title: Mathematical Formula Recognition".to_string()),
            (680.0, "Abstract".to_string()),
            (660.0, "We propose a method for extracting and rendering mathematical".to_string()),
            (640.0, "equations and formulas from scientific documents.".to_string()),
            (620.0, "Introduction".to_string()),
            (600.0, "Mathematical notation presents unique challenges in PDF parsing.".to_string()),
        ]),
        (4, vec![
            (700.0, "Title: Document Structure Analysis".to_string()),
            (680.0, "# Introduction".to_string()),
            (660.0, "This paper examines document structure.".to_string()),
            (640.0, "## Methods".to_string()),
            (620.0, "We employ novel techniques.".to_string()),
            (600.0, "### Results".to_string()),
            (580.0, "Our approach shows significant improvements.".to_string()),
        ]),
        (5, vec![
            (700.0, "# Header Content".to_string()),
            (660.0, "Main document body content.".to_string()),
            (620.0, "## Subsection".to_string()),
            (580.0, "Additional content here.".to_string()),
            (540.0, "# Footer Information".to_string()),
        ]),
        (6, vec![
            (700.0, "Title: Streaming Document Processing".to_string()),
            (680.0, "Schema Version: 1.0".to_string()),
            (660.0, "Total Pages: 5".to_string()),
            (640.0, "Page 1 content begins here.".to_string()),
        ]),
        (7, vec![
            (700.0, "Abstract".to_string()),
            (660.0, "This is the abstract of the paper.".to_string()),
            (620.0, "Introduction".to_string()),
            (580.0, "This is the introduction section.".to_string()),
        ]),
        (8, vec![
            (700.0, "Title: Year Analysis".to_string()),
            (660.0, "Published in 2024, with references to 2023 and 2025.".to_string()),
            (620.0, "The study spanned from 2020 to 2024.".to_string()),
        ]),
        (9, vec![
            (700.0, "Title: Test Document".to_string()),
            (660.0, "This document contains no special patterns.".to_string()),
        ]),
        (10, vec![
            (700.0, "Title: Author Analysis".to_string()),
            (680.0, "Author: Jane Doe".to_string()),
            (660.0, "Creator: Research Institute".to_string()),
            (640.0, "Abstract: A study of document metadata.".to_string()),
        ]),
        (11, vec![
            (700.0, "Title: Hash Test Document".to_string()),
            (660.0, "This document is used for hash testing.".to_string()),
        ]),
        (12, vec![
            (700.0, "Title: Content Stability Test".to_string()),
            (660.0, "Verifies that extraction produces consistent results.".to_string()),
        ]),
        (13, vec![
            (700.0, "Title: Academic Paper Classification".to_string()),
            (680.0, "Abstract".to_string()),
            (660.0, "This paper presents novel research on classification.".to_string()),
            (640.0, "Introduction".to_string()),
            (620.0, "Our method uses advanced machine learning techniques.".to_string()),
            (600.0, "Methods".to_string()),
            (580.0, "We evaluated on multiple datasets.".to_string()),
            (560.0, "Results".to_string()),
            (540.0, "The proposed approach achieves 95% accuracy.".to_string()),
            (520.0, "References".to_string()),
        ]),
        (14, vec![
            (700.0, "Title: Scientific Paper Classification".to_string()),
            (680.0, "Abstract".to_string()),
            (660.0, "This paper examines scientific document analysis.".to_string()),
            (640.0, "Introduction".to_string()),
            (620.0, "Scientific papers have distinctive structure.".to_string()),
            (600.0, "Methods".to_string()),
            (580.0, "We developed a new classification method.".to_string()),
            (560.0, "Results".to_string()),
            (540.0, "Results show strong performance.".to_string()),
            (520.0, "References".to_string()),
        ]),
    ];

    for (i, content) in scientific_papers {
        let path = fixture_path.join(format!("scientific_paper/{:02}.pdf", i));
        let content_refs: Vec<(f32, &str)> = content.iter().map(|(y, s)| (*y, s.as_str())).collect();
        create_content_pdf(&path, &content_refs, &format!("Scientific Paper {}", i))?;
        println!("Created scientific_paper/{:02}.pdf", i);
    }

    // Misc fixtures
    for i in 1..=3 {
        let content = vec![
            (700.0, format!("Misc Document {}", i)),
            (660.0, "Some sample content here.".to_string()),
        ];
        let content_refs: Vec<(f32, &str)> = content.iter().map(|(y, s)| (*y, s.as_str())).collect();
        let path = fixture_path.join(format!("misc/{:02}.pdf", i));
        create_content_pdf(&path, &content_refs, &format!("Misc {}", i))?;
        println!("Created misc/{:02}.pdf", i);
    }

    // Invoice fixtures
    let invoice_content = vec![
        (700.0, "INVOICE".to_string()),
        (660.0, "Invoice Number: 12345".to_string()),
        (620.0, "Total Amount: $100.00".to_string()),
    ];
    let invoice_refs: Vec<(f32, &str)> = invoice_content.iter().map(|(y, s)| (*y, s.as_str())).collect();
    let path = fixture_path.join("invoice/01.pdf");
    create_content_pdf(&path, &invoice_refs, "Invoice")?;
    println!("Created invoice/01.pdf");

    // Contract fixtures
    let contract_content = vec![
        (700.0, "SERVICE AGREEMENT".to_string()),
        (660.0, "This Agreement is made between parties.".to_string()),
        (620.0, "| Term | Description |".to_string()),
        (580.0, "|-------|-------------|".to_string()),
        (540.0, "| Price | $500        |".to_string()),
    ];
    let contract_refs: Vec<(f32, &str)> = contract_content.iter().map(|(y, s)| (*y, s.as_str())).collect();
    let path = fixture_path.join("contract/01.pdf");
    create_content_pdf(&path, &contract_refs, "Contract")?;
    println!("Created contract/01.pdf");

    // Encrypted PDF (simulated - the PDF itself isn't actually encrypted)
    let encrypted_content = vec![
        (700.0, "Encrypted Document".to_string()),
        (660.0, "This document requires password protection.".to_string()),
    ];
    let encrypted_refs: Vec<(f32, &str)> = encrypted_content.iter().map(|(y, s)| (*y, s.as_str())).collect();
    let path = fixture_path.join("encrypted/encrypted.pdf");
    create_content_pdf(&path, &encrypted_refs, "Encrypted PDF")?;
    println!("Created encrypted/encrypted.pdf");

    // Fillable form
    let form_content = vec![
        (700.0, "Application Form".to_string()),
        (660.0, "Name: ______________".to_string()),
        (620.0, "Email: ______________".to_string()),
    ];
    let form_refs: Vec<(f32, &str)> = form_content.iter().map(|(y, s)| (*y, s.as_str())).collect();
    let path = fixture_path.join("fillable-form/form.pdf");
    create_content_pdf(&path, &form_refs, "Fillable Form")?;
    println!("Created fillable-form/form.pdf");

    // Mixed content (multi-page)
    let mixed_pages: Vec<Vec<(f32, String)>> = vec![
        vec![(700.0, "Page 1: Vector Content".to_string()), (660.0, "This is vector text.".to_string())],
        vec![(700.0, "Page 2: Mixed Content".to_string()), (660.0, "Simulated scanned content.".to_string())],
    ];
    let path = fixture_path.join("mixed/mixed.pdf");
    create_multi_page_pdf(&path, &mixed_pages, "Mixed Content Document")?;
    println!("Created mixed/mixed.pdf");

    // Large documents
    for pages in [50, 100] {
        let large_pages: Vec<Vec<(f32, String)>> = (0..pages)
            .map(|i| vec![(700.0, format!("Page {}: Document Content", i + 1))])
            .collect();
        let path = fixture_path.join(format!("large/{}pages.pdf", pages));
        create_multi_page_pdf(&path, &large_pages, &format!("{} Page Document", pages))?;
        println!("Created large/{}pages.pdf", pages);
    }

    // Vertical writing
    let vertical_content = vec![
        (700.0, "Vertical Text".to_string()),
        (660.0, "Japanese text 日本語".to_string()),
    ];
    let vertical_refs: Vec<(f32, &str)> = vertical_content.iter().map(|(y, s)| (*y, s.as_str())).collect();
    let path = fixture_path.join("vertical/vertical.pdf");
    create_content_pdf(&path, &vertical_refs, "Vertical Text Document")?;
    println!("Created vertical/vertical.pdf");

    // Code
    let code_content = vec![
        (700.0, "Code Example".to_string()),
        (660.0, "```".to_string()),
        (620.0, "function test() {".to_string()),
        (580.0, "  return true;".to_string()),
        (540.0, "}".to_string()),
        (500.0, "```".to_string()),
    ];
    let code_refs: Vec<(f32, &str)> = code_content.iter().map(|(y, s)| (*y, s.as_str())).collect();
    let path = fixture_path.join("code/code.pdf");
    create_content_pdf(&path, &code_refs, "Code Sample")?;
    println!("Created code/code.pdf");

    // XMP metadata
    let xmp_content = vec![
        (700.0, "XMP Metadata Document".to_string()),
        (660.0, "This document contains XMP metadata.".to_string()),
    ];
    let xmp_refs: Vec<(f32, &str)> = xmp_content.iter().map(|(y, s)| (*y, s.as_str())).collect();
    let path = fixture_path.join("xmp/xmp-metadata.pdf");
    create_content_pdf(&path, &xmp_refs, "XMP Metadata Document")?;
    println!("Created xmp/xmp-metadata.pdf");

    // Receipts
    create_receipt_json(&fixture_path.join("receipts/valid-receipt.receipt.json"), true)?;
    create_receipt_json(&fixture_path.join("receipts/tampered-receipt.receipt.json"), false)?;

    let receipt_content = vec![
        (700.0, "RECEIPT".to_string()),
        (660.0, "Store: Example Store".to_string()),
        (620.0, "Total: $50.00".to_string()),
    ];
    let receipt_refs: Vec<(f32, &str)> = receipt_content.iter().map(|(y, s)| (*y, s.as_str())).collect();
    create_content_pdf(&fixture_path.join("receipts/valid-receipt.pdf"), &receipt_refs, "Valid Receipt")?;

    let tampered_content = vec![
        (700.0, "RECEIPT".to_string()),
        (660.0, "Store: Different Store".to_string()),
        (620.0, "Total: $999.00".to_string()),
    ];
    let tampered_refs: Vec<(f32, &str)> = tampered_content.iter().map(|(y, s)| (*y, s.as_str())).collect();
    create_content_pdf(&fixture_path.join("receipts/tampered-receipt.pdf"), &tampered_refs, "Tampered Receipt")?;
    println!("Created receipt fixtures");

    // Broken/corrupt PDF
    let path = fixture_path.join("broken/corrupt.pdf");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(&path)?;
    file.write_all(b"%PDF-1.4\nThis is intentionally broken\n%%EOF")?;
    println!("Created broken/corrupt.pdf");

    println!("\nAll proper fixtures created successfully!");
    Ok(())
}
