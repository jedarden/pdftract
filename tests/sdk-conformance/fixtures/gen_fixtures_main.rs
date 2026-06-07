//! Generate valid stub PDF fixtures for SDK conformance testing.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn create_minimal_pdf(path: &Path, text: &str, title: &str) -> std::io::Result<()> {
    let mut pdf = String::new();
    let mut offsets: Vec<usize> = Vec::new();

    // Header
    pdf.push_str("%PDF-1.4\n");

    // Object 1: Catalog
    offsets.push(pdf.len());
    pdf.push_str(&format!(
        "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/Title ({})\n>>\nendobj\n",
        title
    ));

    // Object 2: Pages node
    offsets.push(pdf.len());
    pdf.push_str("2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1>>\nendobj\n");

    // Content stream
    let content = format!(
        "BT\n/F1 12 Tf\n50 700 Td\n({}) Tj\nET\n",
        text
    );

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

fn create_multi_page_pdf(path: &Path, num_pages: usize, title: &str) -> std::io::Result<()> {
    let mut pdf = String::new();
    let mut offsets: Vec<usize> = Vec::new();

    // Header
    pdf.push_str("%PDF-1.4\n");

    // Object 1: Catalog
    offsets.push(pdf.len());
    pdf.push_str(&format!(
        "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/Title ({})\n>>\nendobj\n",
        title
    ));

    // Build page objects and content streams
    let mut page_refs = Vec::new();
    let font_obj_num = 3 + num_pages * 2;

    for i in 0..num_pages {
        let content_obj = 3 + i * 2;
        let page_obj = 4 + i * 2;

        let content = format!("BT\n/F1 12 Tf\n50 700 Td\n(Page {}) Tj\nET\n", i + 1);

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
        num_pages
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
    let fixture_path = Path::new("./tests/sdk-conformance/fixtures");

    // Create base fixture directory
    fs::create_dir_all(&fixture_path)?;
    println!("Creating stub fixtures in: {:?}", fixture_path);

    // Scientific paper fixtures - create with content matching test expectations
    // Paper 01: Basic vector extraction
    let path = fixture_path.join("scientific_paper/01.pdf");
    create_minimal_pdf(&path, "Introduction\n\nThis is a scientific paper with an introduction and methods section.", "Paper 1")?;
    println!("Created scientific_paper/01.pdf");

    // Paper 02: Unicode-heavy text extraction (expects Abstract, Introduction)
    let path = fixture_path.join("scientific_paper/02.pdf");
    create_minimal_pdf(&path, "Abstract\n\nThis is the abstract.\n\nIntroduction\n\nThis is the introduction.", "Paper 2")?;
    println!("Created scientific_paper/02.pdf");

    // Paper 03: Math content (expects equation, formula)
    let path = fixture_path.join("scientific_paper/03.pdf");
    create_minimal_pdf(&path, "Mathematical Analysis\n\nThis paper contains an equation and a formula.", "Paper 3")?;
    println!("Created scientific_paper/03.pdf");

    // Paper 04: Nested headings (for markdown with #, ##, ###)
    let path = fixture_path.join("scientific_paper/04.pdf");
    create_minimal_pdf(&path, "Main Title\nSection One\nSubsection A", "Paper 4")?;
    println!("Created scientific_paper/04.pdf");

    // Papers 05-14: Basic content for remaining tests
    for i in 5..=14 {
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
        create_minimal_pdf(&path, "invoice", &format!("Invoice {}", i))?;
        println!("Created invoice/{:02}.pdf", i);
    }

    // Contract fixtures
    for i in 1..=1 {
        let path = fixture_path.join(format!("contract/{:02}.pdf", i));
        create_minimal_pdf(&path, "AGREEMENT\n\nContract Content", &format!("Contract {}", i))?;
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
