//! Page object access tests from Document results
//!
//! This test module verifies that Page objects can be reliably accessed
//! from Document extraction results. It tests:
//! - Accessing Page objects from ExtractionResult
//! - Handling single Page objects correctly
//! - Handling multiple Pages in list/array correctly
//! - Page object type assertions and validation

use pdftract_core::{extract_pdf, ExtractionOptions};
use pdftract_core::document::parse_pdf_file;
use std::path::Path;

/// Test accessing Pages from ExtractionResult with multiple pages
#[test]
fn test_access_pages_from_extraction_result() {
    let fixture_path = Path::new("tests/fixtures/sample.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
    );

    assert!(result.is_ok(), "Extraction should succeed: {:?}", result.err());
    let extraction_result = result.unwrap();

    // Verify we can access the pages list
    let pages = &extraction_result.pages;
    assert!(!pages.is_empty(), "Should have at least one page");

    // Verify we can access individual pages
    let first_page = &pages[0];
    assert!(first_page.width.is_some() && first_page.width.unwrap() > 0.0,
            "First page should have valid width");
    assert!(first_page.height.is_some() && first_page.height.unwrap() > 0.0,
            "First page should have valid height");

    // Verify page structure
    assert_eq!(first_page.index, 0, "First page should have index 0");
}

/// Test accessing Page objects from parse_pdf_file result
#[test]
fn test_access_pages_from_parse_result() {
    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let (_fingerprint, _catalog, pages, _resolver, _dict) =
        parse_pdf_file(fixture_path).expect("Should parse PDF file");

    // Verify we can access the pages vector
    assert!(!pages.is_empty(), "Should have at least one page");

    // Verify we can access individual PageDict objects
    let first_page = &pages[0];
    assert!(first_page.media_box[2] > 0.0, "Page should have valid media box width");
    assert!(first_page.media_box[3] > 0.0, "Page should have valid media box height");

    // Verify page dictionary structure
    assert_eq!(pages.len(), 1, "Should have exactly one page for minimal fixture");
}

/// Test handling single Page object
#[test]
fn test_single_page_access() {
    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
    );

    assert!(result.is_ok(), "Extraction should succeed");
    let extraction_result = result.unwrap();

    // Single page should be accessible
    assert_eq!(extraction_result.pages.len(), 1, "Should have exactly one page");

    let page = &extraction_result.pages[0];
    assert_eq!(page.index, 0, "Single page should have index 0");
    assert!(page.width.is_some() && page.width.unwrap() > 0.0,
            "Page should have valid width");
    assert!(page.height.is_some() && page.height.unwrap() > 0.0,
            "Page should have valid height");
}

/// Test handling multiple Pages in list/array
#[test]
fn test_multiple_pages_access() {
    let fixture_path = Path::new("tests/fixtures/remote_100page.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
    );

    assert!(result.is_ok(), "Extraction should succeed");
    let extraction_result = result.unwrap();

    // Multiple pages should be accessible as a list
    assert!(extraction_result.pages.len() > 1, "Should have multiple pages");

    // Verify each page is accessible
    for (i, page) in extraction_result.pages.iter().enumerate() {
        assert_eq!(page.index, i, "Page {} should have correct index", i);
        assert!(page.width.is_some() && page.width.unwrap() > 0.0,
                "Page {} should have valid width", i);
        assert!(page.height.is_some() && page.height.unwrap() > 0.0,
                "Page {} should have valid height", i);
    }

    // Test accessing specific pages by index
    let first_page = &extraction_result.pages[0];
    let last_page = &extraction_result.pages[extraction_result.pages.len() - 1];

    assert_eq!(first_page.index, 0, "First page index should be 0");
    assert_eq!(last_page.index, extraction_result.pages.len() - 1,
               "Last page index should match");
}

/// Test Page object type assertions
#[test]
fn test_page_type_assertions() {
    let fixture_path = Path::new("tests/fixtures/sample.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
    );

    assert!(result.is_ok(), "Extraction should succeed");
    let extraction_result = result.unwrap();

    // Type assertion: pages should be a vector
    let pages: &Vec<_> = &extraction_result.pages;
    assert!(!pages.is_empty(), "Pages vector should not be empty");

    // Type assertion: each page should have specific fields
    for page in pages.iter() {
        // Type assertion: page index should be usize
        let index: usize = page.index;
        assert!(index < 1000, "Page index should be reasonable");

        // Type assertion: page dimensions should be Option<f32>
        let width: Option<f32> = page.width;
        let height: Option<f32> = page.height;
        assert!(width.is_some() && width.unwrap() > 0.0 && width.unwrap() < 10000.0,
                "Page width should be reasonable");
        assert!(height.is_some() && height.unwrap() > 0.0 && height.unwrap() < 10000.0,
                "Page height should be reasonable");

        // Type assertion: page rotation should be Option<u16>
        let rotation: Option<u16> = page.rotation;
        if let Some(rot) = rotation {
            assert!(rot == 0 || rot == 90 || rot == 180 || rot == 270,
                    "Page rotation should be 0, 90, 180, or 270 degrees");
        }
    }
}

/// Test accessing PageDict from parse_pdf_file
#[test]
fn test_pagedict_access_from_parse() {
    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let (_fingerprint, _catalog, pages, _resolver, _dict) =
        parse_pdf_file(fixture_path).expect("Should parse PDF");

    // Type assertion: pages should be Vec<PageDict>
    assert!(!pages.is_empty(), "Should have at least one PageDict");

    // Access first PageDict
    let page_dict = &pages[0];

    // Type assertion: media_box should be [f64; 4]
    let media_box: [f64; 4] = page_dict.media_box;
    assert!(media_box[2] > 0.0, "MediaBox width should be positive");
    assert!(media_box[3] > 0.0, "MediaBox height should be positive");

    // Type assertion: crop_box should be Option<[f64; 4]>
    let crop_box: Option<[f64; 4]> = page_dict.crop_box;
    // crop_box is optional, so we just verify the type matches

    // Type assertion: rotate should be i32
    let rotate: i32 = page_dict.rotate;
    assert!(rotate == 0 || rotate == 90 || rotate == 180 || rotate == 270,
            "Page rotation should be valid");
}

/// Test page iteration patterns
#[test]
fn test_page_iteration_patterns() {
    let fixture_path = Path::new("tests/fixtures/sample.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
    );

    assert!(result.is_ok(), "Extraction should succeed");
    let extraction_result = result.unwrap();

    // Test 1: Iterating over pages
    let mut page_count = 0;
    for page in &extraction_result.pages {
        page_count += 1;
        assert!(page.width.is_some() && page.width.unwrap() > 0.0,
                "Page {} should have valid width", page.index);
        assert!(page.height.is_some() && page.height.unwrap() > 0.0,
                "Page {} should have valid height", page.index);
    }
    assert!(page_count > 0, "Should iterate over at least one page");

    // Test 2: Enumerated iteration
    for (i, page) in extraction_result.pages.iter().enumerate() {
        assert_eq!(i, page.index, "Enumeration index should match page index");
    }

    // Test 3: First and last page access
    if !extraction_result.pages.is_empty() {
        let first = extraction_result.pages.first().unwrap();
        let last = extraction_result.pages.last().unwrap();
        assert_eq!(first.index, 0, "First page should have index 0");
        assert!(last.index >= first.index, "Last page index >= first page index");
    }
}

/// Test empty page list handling
#[test]
fn test_empty_page_handling() {
    // Create a test with empty pages to ensure we handle it gracefully
    let pages: Vec<pdftract_core::extract::PageResult> = vec![];

    assert_eq!(pages.len(), 0, "Empty pages vector should have length 0");
    assert!(pages.is_empty(), "Empty pages vector should be empty");

    // Test first/last on empty vector
    assert!(pages.first().is_none(), "First page on empty vector should be None");
    assert!(pages.last().is_none(), "Last page on empty vector should be None");
}

/// Test page indexing and bounds
#[test]
fn test_page_indexing_bounds() {
    let fixture_path = Path::new("tests/fixtures/sample.pdf");

    if !fixture_path.exists() {
        println!("Skipping test: fixture not found at {}", fixture_path.display());
        return;
    }

    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
    );

    assert!(result.is_ok(), "Extraction should succeed");
    let extraction_result = result.unwrap();

    if !extraction_result.pages.is_empty() {
        let page_count = extraction_result.pages.len();

        // Valid indexing
        let _first_page = &extraction_result.pages[0];
        let _last_page = &extraction_result.pages[page_count - 1];

        // Test get method for safe access
        let valid_page = extraction_result.pages.get(0);
        assert!(valid_page.is_some(), "Should get page at valid index 0");

        let invalid_page = extraction_result.pages.get(page_count);
        assert!(invalid_page.is_none(), "Should not get page at out-of-bounds index");

        // Test contains pattern
        let pages_vec = &extraction_result.pages;
        assert!(pages_vec.len() == page_count, "Pages vector length should match");
    }
}
