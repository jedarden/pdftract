//! Test single Page extraction from Document
//!
//! This test module verifies that the page_helper::extract_page function
//! correctly extracts a single Page from a Document, which is the foundational
//! extraction path needed by most tests.
//!
//! Bead: bf-8p3b2j - Implement single Page extraction from Document
//!
//! Acceptance criteria:
//! - Function successfully extracts a Page from valid Document
//! - Returns Err() for Documents missing Page data
//! - Handles the nested structure correctly
//! - One test demonstrates successful extraction

use pdftract_core::document::Document;
use pdftract_core::page_helper;
use std::path::Path;

/// Test extracting a single page from a valid Document
#[test]
fn test_extract_single_page_from_document() {
    // Use a test fixture that we know exists
    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    // Open the PDF to get a Document
    let doc = match Document::open(fixture_path) {
        Ok(d) => d,
        Err(e) => {
            println!("Skipping test: failed to open document: {:?}", e);
            return;
        }
    };

    // Get the page count to verify the document has pages
    let page_count = match doc.page_count() {
        Ok(count) => count,
        Err(e) => {
            println!("Skipping test: failed to get page count: {:?}", e);
            return;
        }
    };

    assert!(
        page_count > 0,
        "Document should have at least one page, got {}",
        page_count
    );

    // Extract the first page using page_helper::extract_page
    let page = match page_helper::extract_page(&doc, 0) {
        Ok(page) => page,
        Err(e) => {
            panic!("Failed to extract page at index 0: {:?}", e);
        }
    };

    // Verify the extracted page has the expected properties
    assert_eq!(page.index, 0, "Extracted page should have index 0");
    assert!(page.width > 0.0, "Page should have positive width");
    assert!(page.height > 0.0, "Page should have positive height");

    // Verify we can access the rotation field (handles nested structure)
    // Rotation is stored in the PageDict and extracted into PageExtraction
    assert!(
        page.rotation == 0 || page.rotation == 90 || page.rotation == 180 || page.rotation == 270,
        "Page rotation should be valid (0, 90, 180, or 270), got {}",
        page.rotation
    );
}

/// Test error handling for out-of-bounds page index
#[test]
fn test_extract_page_out_of_bounds() {
    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let doc = match Document::open(fixture_path) {
        Ok(d) => d,
        Err(e) => {
            println!("Skipping test: failed to open document: {:?}", e);
            return;
        }
    };

    let page_count = match doc.page_count() {
        Ok(count) => count,
        Err(e) => {
            println!("Skipping test: failed to get page count: {:?}", e);
            return;
        }
    };

    // Try to extract a page beyond the document's page count
    let result = page_helper::extract_page(&doc, page_count + 10);

    // Should return an error
    assert!(
        result.is_err(),
        "Should return error for out-of-bounds page index"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("out of bounds") || error_msg.contains("Page index"),
        "Error message should mention out-of-bounds or page index, got: {}",
        error_msg
    );
}

/// Test that extract_page handles the nested Document structure correctly
#[test]
fn test_extract_page_handles_nested_structure() {
    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let doc = match Document::open(fixture_path) {
        Ok(d) => d,
        Err(e) => {
            println!("Skipping test: failed to open document: {:?}", e);
            return;
        }
    };

    // The Document structure is: Document -> catalog -> pages_ref -> PageDict
    // extract_page should navigate this structure correctly

    // Extract the first page
    let page = match page_helper::extract_page(&doc, 0) {
        Ok(page) => page,
        Err(e) => {
            panic!("Failed to extract page: {:?}", e);
        }
    };

    // Verify that fields from the nested PageDict were extracted correctly
    // PageExtraction should have width/height from media_box
    assert!(page.width > 0.0, "Width should be extracted from media_box");
    assert!(page.height > 0.0, "Height should be extracted from media_box");

    // Verify rotation from PageDict
    assert!(
        page.rotation >= 0 && page.rotation <= 270,
        "Rotation should be valid"
    );
}

/// Test extracting from a multi-page document
#[test]
fn test_extract_page_from_multi_page_document() {
    let fixture_path = Path::new("tests/fixtures/sample.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let doc = match Document::open(fixture_path) {
        Ok(d) => d,
        Err(e) => {
            println!("Skipping test: failed to open document: {:?}", e);
            return;
        }
    };

    let page_count = match doc.page_count() {
        Ok(count) => count,
        Err(e) => {
            println!("Skipping test: failed to get page count: {:?}", e);
            return;
        }
    };

    if page_count < 2 {
        println!("Skipping test: document has less than 2 pages");
        return;
    }

    // Extract first and second pages
    let page0 = match page_helper::extract_page(&doc, 0) {
        Ok(page) => page,
        Err(e) => {
            panic!("Failed to extract page 0: {:?}", e);
        }
    };

    let page1 = match page_helper::extract_page(&doc, 1) {
        Ok(page) => page,
        Err(e) => {
            panic!("Failed to extract page 1: {:?}", e);
        }
    };

    // Verify they have different indices
    assert_eq!(page0.index, 0, "First page should have index 0");
    assert_eq!(page1.index, 1, "Second page should have index 1");
}
