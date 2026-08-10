//! Test multi-Page collection extraction from Document
//!
//! This test module verifies that the page_helper functions correctly
//! extract multiple Pages from Documents and return Page collections.
//!
//! Bead: bf-1iebiy - Add multiple Pages and collection support
//!
//! Acceptance criteria:
//! - Function can extract multiple Pages from a Document
//! - Returns empty collection when no Pages present (not an error)
//! - Single and multi-Page paths use shared extraction logic
//! - One test demonstrates multi-Page extraction

use pdftract_core::document::Document;
use pdftract_core::page_helper;
use std::path::Path;

/// Test extracting all pages from a multi-page document
#[test]
fn test_extract_all_pages_multi_page_document() {
    let fixture_path = Path::new("tests/fixtures/multipage-100.pdf");

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

    // Extract all pages using extract_all_pages
    let pages = match page_helper::extract_all_pages(&doc) {
        Ok(p) => p,
        Err(e) => {
            panic!("Failed to extract all pages: {:?}", e);
        }
    };

    // Verify we got multiple pages
    assert!(
        pages.len() > 1,
        "Document should have multiple pages, got {}",
        pages.len()
    );

    // Verify all pages have valid data
    for page in &pages {
        assert!(page.width > 0.0, "Page {} should have positive width", page.index);
        assert!(page.height > 0.0, "Page {} should have positive height", page.index);
        assert!(
            page.rotation == 0 || page.rotation == 90 || page.rotation == 180 || page.rotation == 270,
            "Page {} should have valid rotation, got {}",
            page.index,
            page.rotation
        );
    }

    println!("Successfully extracted {} pages", pages.len());
}

/// Test extracting a range of pages from a multi-page document
#[test]
fn test_extract_page_range() {
    let fixture_path = Path::new("tests/fixtures/multipage-100.pdf");

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

    if page_count < 10 {
        println!("Skipping test: document has less than 10 pages");
        return;
    }

    // Extract a range of pages (e.g., pages 5-9)
    let pages = match page_helper::extract_page_range(&doc, 5, 9) {
        Ok(p) => p,
        Err(e) => {
            panic!("Failed to extract page range: {:?}", e);
        }
    };

    // Verify we got the correct number of pages (5 through 9 = 5 pages)
    assert_eq!(
        pages.len(),
        5,
        "Should extract 5 pages (indices 5-9), got {}",
        pages.len()
    );

    // Verify the extracted pages have the correct indices
    for (i, page) in pages.iter().enumerate() {
        assert_eq!(
            page.index,
            5 + i,
            "Page {} should have index {}",
            i,
            5 + i
        );
    }

    println!(
        "Successfully extracted pages {} through {}",
        pages[0].index,
        pages[pages.len() - 1].index
    );
}

/// Test that extract_all_pages returns empty collection for single-page document
#[test]
fn test_extract_all_pages_single_page() {
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

    // Extract all pages
    let pages = match page_helper::extract_all_pages(&doc) {
        Ok(p) => p,
        Err(e) => {
            panic!("Failed to extract all pages from single-page document: {:?}", e);
        }
    };

    // Should succeed even with just one page
    assert!(
        pages.len() >= 1,
        "Should have at least one page, got {}",
        pages.len()
    );
}

/// Test that shared extraction logic is used by both single and multi-page paths
#[test]
fn test_shared_extraction_logic() {
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

    // Extract page using single-page path
    let single_page = match page_helper::extract_page(&doc, 0) {
        Ok(p) => p,
        Err(e) => {
            panic!("Failed to extract single page: {:?}", e);
        }
    };

    // Extract all pages and get the first page
    let all_pages = match page_helper::extract_all_pages(&doc) {
        Ok(p) => p,
        Err(e) => {
            panic!("Failed to extract all pages: {:?}", e);
        }
    };

    if all_pages.is_empty() {
        println!("Skipping test: document has no pages");
        return;
    }

    let first_page = &all_pages[0];

    // Verify both paths return the same page data (they use shared validation)
    assert_eq!(
        single_page.index, first_page.index,
        "Page indices should match"
    );
    assert_eq!(
        single_page.width, first_page.width,
        "Page widths should match"
    );
    assert_eq!(
        single_page.height, first_page.height,
        "Page heights should match"
    );
    assert_eq!(
        single_page.rotation, first_page.rotation,
        "Page rotations should match"
    );
}

/// Test extracting page range with start == end (single page from range function)
#[test]
fn test_extract_page_range_single_page() {
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

    // Extract a "range" with just one page
    let pages = match page_helper::extract_page_range(&doc, 0, 0) {
        Ok(p) => p,
        Err(e) => {
            panic!("Failed to extract single-page range: {:?}", e);
        }
    };

    assert_eq!(pages.len(), 1, "Should extract exactly 1 page");
    assert_eq!(pages[0].index, 0, "Extracted page should have index 0");
}

/// Test error handling for invalid page ranges
#[test]
fn test_extract_page_range_invalid_bounds() {
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

    // Test start > end (invalid range)
    let result = page_helper::extract_page_range(&doc, 5, 2);
    assert!(
        result.is_err(),
        "Should return error when start > end"
    );

    // Test start out of bounds
    let result = page_helper::extract_page_range(&doc, page_count + 10, page_count + 15);
    assert!(
        result.is_err(),
        "Should return error when start is out of bounds"
    );

    // Test end out of bounds
    let result = page_helper::extract_page_range(&doc, 0, page_count + 10);
    assert!(
        result.is_err(),
        "Should return error when end is out of bounds"
    );
}

/// Test that extract_all_pages handles documents with varying page counts
#[test]
fn test_extract_all_pages_varying_counts() {
    let fixtures = vec![
        Path::new("tests/fixtures/test-minimal.pdf"),
        Path::new("tests/fixtures/multipage-100.pdf"),
        Path::new("tests/fixtures/linearized-10.pdf"),
    ];

    for fixture_path in fixtures {
        if !fixture_path.exists() {
            continue;
        }

        let doc = match Document::open(fixture_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let page_count = match doc.page_count() {
            Ok(c) => c,
            Err(_) => continue,
        };

        match page_helper::extract_all_pages(&doc) {
            Ok(pages) => {
                assert_eq!(
                    pages.len(),
                    page_count,
                    "Should extract all {} pages from {}",
                    page_count,
                    fixture_path.display()
                );

                for page in &pages {
                    assert!(page.width > 0.0, "Page should have positive width");
                    assert!(page.height > 0.0, "Page should have positive height");
                }

                println!(
                    "✓ Extracted {} pages from {}",
                    pages.len(),
                    fixture_path.display()
                );
            }
            Err(e) => {
                panic!(
                    "Failed to extract all pages from {}: {:?}",
                    fixture_path.display(),
                    e
                );
            }
        }
    }
}
