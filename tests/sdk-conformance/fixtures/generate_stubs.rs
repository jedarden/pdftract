//! Generate minimal valid PDF files for conformance testing.
//!
//! Compile and run: rustc generate_stubs.rs && ./generate_stubs

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn create_minimal_pdf(path: &Path, text: &str, _title: &str) -> std::io::Result<()> {
    // Proper PDF structure with correct object ordering
    let pdf_data = format!(
        "%PDF-1.4\n\
         1 0 obj\n\
         <<\n\
         /Type /Catalog\n\
         /Pages 2 0 R\n\
         >>\n\
         endobj\n\
         2 0 obj\n\
         <<\n\
         /Type /Pages\n\
         /Kids [3 0 R]\n\
         /Count 1\n\
         >>\n\
         endobj\n\
         3 0 obj\n\
         <<\n\
         /Type /Page\n\
         /Parent 2 0 R\n\
         /MediaBox [0 0 612 792]\n\
         /Contents 4 0 R\n\
         /Resources <<\n\
         /Font <<\n\
         /F1 5 0 R\n\
         >>\n\
         >>\n\
         >>\n\
         endobj\n\
         4 0 obj\n\
         <<\n\
         /Length {}\n\
         >>\n\
         stream\n\
         BT\n\
         /F1 12 Tf\n\
         50 700 Td\n\
         ({}) Tj\n\
         ET\n\
         endstream\n\
         endobj\n\
         5 0 obj\n\
         <<\n\
         /Type /Font\n\
         /Subtype /Type1\n\
         /BaseFont /Helvetica\n\
         >>\n\
         endobj\n\
         xref\n\
         0 6\n\
         0000000000 65535 f \n\
         0000000009 00000 n \n\
         0000000058 00000 n \n\
         0000000115 00000 n \n\
         0000000264 00000 n \n\
         0000000339 00000 n \n\
         trailer\n\
         <<\n\
         /Size 6\n\
         /Root 1 0 R\n\
         >>\n\
         startxref\n\
         402\n\
         %%EOF\n",
        text.len(),
        text
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(pdf_data.as_bytes())?;
    Ok(())
}

fn create_multi_page_pdf(path: &Path, num_pages: usize, _title: &str) -> std::io::Result<()> {
    // For simplicity, create minimal multi-page PDFs
    // Each page will be a minimal reference
    let mut pdf = format!("%PDF-1.4\n");
    let mut offset = pdf.len();

    // Object 1: Catalog
    pdf.push_str("1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n");

    // Build pages array
    let kids: Vec<String> = (0..num_pages).map(|i| format!("{} 0 R", 3 + i * 2)).collect();
    let kids_str = kids.join(" ");

    // Object 2: Pages
    pdf.push_str(&format!(
        "2 0 obj\n<<\n/Type /Pages\n/Kids [{}]\n/Count {}>>\nendobj\n",
        kids_str, num_pages
    ));

    // Page objects
    for i in 0..num_pages {
        let content = format!("BT\n/F1 12 Tf\n50 700 Td\n(Page {}) Tj\nET\n", i + 1);
        pdf.push_str(&format!(
            "{} 0 obj\n<<\n/Length {}>>\nstream\n{}\nendstream\nendobj\n",
            3 + i * 2,
            content.len(),
            content
        ));
        pdf.push_str(&format!(
            "{} 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents {} 0 R\n/Resources <<\n/Font <<\n/F1 {} 0 R\n>>\n>>\n>>\nendobj\n",
            4 + i * 2,
            3 + i * 2,
            3 + num_pages * 2
        ));
    }

    // Font object
    let font_obj = 3 + num_pages * 2;
    pdf.push_str(&format!(
        "{} 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n",
        font_obj
    ));

    let xref_offset = pdf.len();
    pdf.push_str("xref\n0 1\n0000000000 65535 f \n");

    // Simple xref for this case - we'll build it properly
    let xref_start = pdf.len();
    pdf.push_str(&format!("0 {}\n", font_obj + 2));
    pdf.push_str("0000000000 65535 f \n");

    // For simplicity, use a simpler approach for multi-page
    // Rebuild with simpler structure
    let simple_pdf = format!(
        "%PDF-1.4\n\
         1 0 obj\n\
         <<\n\
         /Type /Catalog\n\
         /Pages 2 0 R\n\
         >>\n\
         endobj\n\
         2 0 obj\n\
         <<\n\
         /Type /Pages\n\
         /Kids [{}]\n\
         /Count {}\n\
         >>\n\
         endobj\n",
         (0..num_pages).map(|i| format!("{} 0 R", 3 + i)).collect::<Vec<_>>().join(" "),
         num_pages
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(simple_pdf.as_bytes())?;

    // Write page objects
    for i in 0..num_pages {
        let page_obj = format!(
            "{} 0 obj\n\
             <<\n\
             /Type /Page\n\
             /Parent 2 0 R\n\
             /MediaBox [0 0 612 792]\n\
             /Contents 4 0 R\n\
             /Resources <<\n\
             /Font <<\n\
             /F1 5 0 R\n\
             >>\n\
             >>\n\
             >>\n\
             endobj\n",
            3 + i
        );
        file.write_all(page_obj.as_bytes())?;
    }

    // Content and font
    let content = "BT\n/F1 12 Tf\n50 700 Td\n(Test) Tj\nET\n";
    file.write_all(
        format!(
            "4 0 obj\n\
             <<\n\
             /Length {}\n\
             >>\n\
             stream\n\
             {}\n\
             endstream\n\
             endobj\n",
            content.len(),
            content
        )
        .as_bytes(),
    );
    file.write_all(b"5 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n");

    // xref and trailer
    file.write_all(b"xref\n0 6\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n");
    file.write_all(b"0000000115 00000 n \n0000000264 00000 n \n0000000339 00000 n \n");
    file.write_all(b"trailer\n<<\n/Size 6\n/Root 1 0 R\n>>\nstartxref\n402\n%%EOF\n");

    Ok(())
}

fn main() -> std::io::Result<()> {
    let fixture_dir = Path::new("tests/sdk-conformance/fixtures");

    println!("Creating stub fixtures in: {:?}", fixture_dir);

    // Scientific paper fixtures (01-14)
    for i in 1..=14 {
        let path = fixture_dir.join(format!("scientific_paper/{:02}.pdf", i));
        let text = match i {
            1 => "Abstract\nIntroduction\nMethods\nResults",
            7 => "Abstract\nThis is the abstract text.\n\nIntroduction",
            8 => "2024\n2023\n2022\n2021",
            9 => "Some content",
            10 => "Title\nAuthor Name\nCreator Tool",
            11 => "Hash Test PDF",
            12 => "Content Stability Test",
            13 => "Abstract\nMethods\nResults\nReferences",
            14 => "Methods\nResults\nDiscussion",
            _ => "Scientific paper content",
        };
        create_minimal_pdf(&path, text, &format!("Paper {}", i))?;
        println!("Created scientific_paper/{:02}.pdf", i);
    }

    // Misc fixtures
    for i in 1..=3 {
        let path = fixture_dir.join(format!("misc/{:02}.pdf", i));
        let text = match i {
            1 => "Receipt content",
            2 => "Minimal content",
            3 => "Another misc file",
            _ => "Misc content",
        };
        create_minimal_pdf(&path, text, &format!("Misc {}", i))?;
        println!("Created misc/{:02}.pdf", i);
    }

    // Invoice fixtures
    let path = fixture_dir.join("invoice/01.pdf");
    create_minimal_pdf(&path, "INVOICE\nInvoice Details", "Invoice")?;
    println!("Created invoice/01.pdf");

    // Contract fixtures
    let path = fixture_dir.join("contract/01.pdf");
    create_minimal_pdf(&path, "AGREEMENT\n| Header |\n| Row 1 |", "Contract")?;
    println!("Created contract/01.pdf");

    // Encrypted PDF
    let path = fixture_dir.join("encrypted/encrypted.pdf");
    create_minimal_pdf(&path, "Encrypted Content", "Encrypted PDF")?;
    println!("Created encrypted/encrypted.pdf");

    // Fillable form
    let path = fixture_dir.join("fillable-form/form.pdf");
    create_minimal_pdf(&path, "Form Content", "Fillable Form")?;
    println!("Created fillable-form/form.pdf");

    // Mixed content
    let path = fixture_dir.join("mixed/mixed.pdf");
    create_multi_page_pdf(&path, 2, "Mixed Content Document")?;
    println!("Created mixed/mixed.pdf");

    // Large documents
    for pages in [50, 100] {
        let path = fixture_dir.join(format!("large/{}pages.pdf", pages));
        create_multi_page_pdf(&path, pages, &format!("{} Page Document", pages))?;
        println!("Created large/{}pages.pdf", pages);
    }

    // Vertical writing
    let path = fixture_dir.join("vertical/vertical.pdf");
    create_minimal_pdf(&path, "Vertical", "Vertical Text Document")?;
    println!("Created vertical/vertical.pdf");

    // Code
    let path = fixture_dir.join("code/code.pdf");
    create_minimal_pdf(&path, "function test() { return true; }", "Code Sample")?;
    println!("Created code/code.pdf");

    // XMP metadata
    let path = fixture_dir.join("xmp/xmp-metadata.pdf");
    create_minimal_pdf(&path, "XMP Document", "XMP Metadata Document")?;
    println!("Created xmp/xmp-metadata.pdf");

    // Receipts
    let receipt_dir = fixture_dir.join("receipts");
    fs::create_dir_all(&receipt_dir)?;

    let mut file = File::create(receipt_dir.join("valid-receipt.receipt.json"))?;
    file.write_all(br#"{"fingerprint": "stub-valid", "signature": "valid-signature"}"#)?;

    let mut file = File::create(receipt_dir.join("tampered-receipt.receipt.json"))?;
    file.write_all(br#"{"fingerprint": "stub-tampered", "signature": "invalid-signature"}"#)?;

    create_minimal_pdf(&receipt_dir.join("valid-receipt.pdf"), "Valid Receipt", "Valid Receipt")?;
    create_minimal_pdf(&receipt_dir.join("tampered-receipt.pdf"), "Tampered Receipt", "Tampered Receipt")?;
    println!("Created receipt fixtures");

    // Broken/corrupt PDF
    let path = fixture_dir.join("broken/corrupt.pdf");
    fs::create_dir_all(path.parent().unwrap())?;
    let mut file = File::create(&path)?;
    file.write_all(b"%PDF-1.4\nThis is intentionally broken\n%%EOF")?;
    println!("Created broken/corrupt.pdf");

    println!("\nAll stub fixtures created successfully!");
    Ok(())
}
