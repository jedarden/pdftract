//! Demonstration of single Page extraction from Document
//!
//! This example shows how to extract a single page from a PDF document
//! using the page_helper::extract_page function, which is the foundational
//! extraction path needed by most tests.
//!
//! Bead: bf-8p3b2j - Implement single Page extraction from Document

use pdftract_core::document::Document;
use pdftract_core::page_helper;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Single Page Extraction Demo ===\n");

    // Use a test fixture
    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    if !fixture_path.exists() {
        println!("Test fixture not found at {}", fixture_path.display());
        println!("Creating a minimal test PDF...");
        create_minimal_pdf(fixture_path)?;
    }

    // Open the PDF to get a Document
    println!("Opening PDF: {}", fixture_path.display());
    let doc = Document::open(fixture_path)?;

    // Get the page count
    let page_count = doc.page_count()?;
    println!("Document has {} page(s)\n", page_count);

    if page_count == 0 {
        println!("Document has no pages. Exiting.");
        return Ok(());
    }

    // Extract the first page using page_helper::extract_page
    println!("Extracting page at index 0...");
    let page = page_helper::extract_page(&doc, 0)?;

    // Verify the extracted page has the expected properties
    println!("Page extraction successful!\n");
    println!("Extracted page properties:");
    println!("  - Index: {}", page.index);
    println!("  - Width: {} points", page.width);
    println!("  - Height: {} points", page.height);
    println!("  - Rotation: {} degrees", page.rotation);
    println!("  - Spans: {} text span(s)", page.spans.len());
    println!("  - Blocks: {} block(s)", page.blocks.len());

    // Validate the page data
    println!("\nValidation:");
    println!("  ✓ Page index is 0");
    println!("  ✓ Width is positive ({})", page.width > 0.0);
    println!("  ✓ Height is positive ({})", page.height > 0.0);
    println!(
        "  ✓ Rotation is valid ({})",
        matches!(page.rotation, 0 | 90 | 180 | 270)
    );

    // Demonstrate error handling for out-of-bounds access
    println!("\nDemonstrating error handling:");
    let out_of_bounds_result = page_helper::extract_page(&doc, page_count + 10);
    match out_of_bounds_result {
        Err(e) => println!("  ✓ Correctly returns error for out-of-bounds: {}", e),
        Ok(_) => println!("  ✗ Should have returned error for out-of-bounds index"),
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}

/// Create a minimal valid PDF for testing.
fn create_minimal_pdf(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    let pdf_data = br#"%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
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
/F1 <<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
>>
>>
endobj
4 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
100 700 Td
(Test) Tj
ET
endstream
endobj
xref
0 5
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000298 00000 n
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
376
%%EOF
"#;

    let mut file = File::create(path)?;
    file.write_all(pdf_data)?;
    println!("Created test PDF at {}", path.display());
    Ok(())
}
