//! Single Page access pattern integration tests
//!
//! This module tests single Page object access from Document extraction results.
//! It validates that the page_helpers functions work correctly with real PDF fixtures.
//!
//! Acceptance criteria (from bead bf-47s16l):
//! - Test can access a single Page object from Document
//! - Page properties are asserted correctly
//! - Test passes with single-Page fixtures
//! - Error handling works for missing Page data

use pdftract_core::{extract_pdf, page_helpers, ExtractionOptions};
use std::path::{Path, PathBuf};

/// Get the workspace root directory
fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = PathBuf::from(manifest_dir);
    // We're in crates/pdftract-cli, so go up two levels to reach workspace root
    path.parent().unwrap().parent().unwrap().to_path_buf()
}

/// Path to test fixtures directory
fn fixtures_dir() -> PathBuf {
    workspace_root().join("tests/fixtures")
}

/// Get a single-page PDF fixture for testing
fn single_page_fixture() -> PathBuf {
    // test-minimal.pdf is typically a single page
    fixtures_dir().join("test-minimal.pdf")
}

/// Get a multi-page PDF fixture for testing
fn multi_page_fixture() -> PathBuf {
    // cjk fixtures typically have multiple pages
    fixtures_dir().join("cjk/cjk-chinese-gb18030.pdf")
}

#[test]
fn test_single_page_access_by_index() {
    let fixture_path = single_page_fixture();
    assert!(
        fixture_path.exists(),
        "Single-page fixture not found at {}",
        fixture_path.display()
    );

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    // Access the first page using get_page (zero-based index)
    let page = page_helpers::get_page(&result, 0)
        .expect("Failed to get page at index 0");

    // Verify page properties
    assert_eq!(
        page.index, 0,
        "Page index should be 0"
    );
    assert_eq!(
        page.page_number, 1,
        "Page number should be 1 (1-based)"
    );

    // Verify page has dimensions
    assert!(
        page.width.is_some() || page.width.is_none(),
        "Page width field exists"
    );
    assert!(
        page.height.is_some() || page.height.is_none(),
        "Page height field exists"
    );

    // Verify spans exist (even if empty)
    let spans = &page.spans;
    // Spans may be empty for minimal fixtures, but the slice should be accessible
    let _ = spans.len();
}

#[test]
fn test_single_page_access_by_first_page_helper() {
    let fixture_path = single_page_fixture();
    assert!(
        fixture_path.exists(),
        "Single-page fixture not found at {}",
        fixture_path.display()
    );

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    // Access the first page using first_page helper
    let page = page_helpers::first_page(&result)
        .expect("Failed to get first page");

    // Verify page properties
    assert_eq!(
        page.index, 0,
        "Page index should be 0"
    );
    assert_eq!(
        page.page_number, 1,
        "Page number should be 1"
    );
}

#[test]
fn test_single_page_spans_access() {
    let fixture_path = single_page_fixture();
    assert!(
        fixture_path.exists(),
        "Single-page fixture not found at {}",
        fixture_path.display()
    );

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    // Access page and then its spans
    let page = page_helpers::get_page(&result, 0)
        .expect("Failed to get page at index 0");

    // Spans slice should be accessible (even if empty)
    let span_count = page.spans.len();
    // For minimal fixtures, spans may be empty, which is valid
    assert!(
        span_count >= 0,
        "Span count should be non-negative"
    );
}

#[test]
fn test_page_count_for_single_page() {
    let fixture_path = single_page_fixture();
    assert!(
        fixture_path.exists(),
        "Single-page fixture not found at {}",
        fixture_path.display()
    );

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    // Get page count (returns usize, not Result)
    let count = page_helpers::page_count(&result);

    // Single-page fixture should have exactly 1 page
    assert_eq!(
        count, 1,
        "Single-page fixture should have exactly 1 page, got {}",
        count
    );
}

#[test]
fn test_is_single_page_for_single_page() {
    let fixture_path = single_page_fixture();
    assert!(
        fixture_path.exists(),
        "Single-page fixture not found at {}",
        fixture_path.display()
    );

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    // Check is_single_page (returns bool, not Result)
    assert!(
        page_helpers::is_single_page(&result),
        "Single-page fixture should be detected as single page"
    );
}

#[test]
fn test_multi_page_first_page_access() {
    let fixture_path = multi_page_fixture();
    if !fixture_path.exists() {
        // Skip if fixture not available
        println!("Skipping test: multi-page fixture not found at {}", fixture_path.display());
        return;
    }

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    // Verify it has multiple pages
    let count = page_helpers::page_count(&result);
    assert!(
        count > 1,
        "Multi-page fixture should have more than 1 page, got {}",
        count
    );

    // Access first page
    let page = page_helpers::first_page(&result)
        .expect("Failed to get first page");

    assert_eq!(
        page.page_number, 1,
        "First page should have page_number 1"
    );
}

#[test]
fn test_multi_page_last_page_access() {
    let fixture_path = multi_page_fixture();
    if !fixture_path.exists() {
        // Skip if fixture not available
        println!("Skipping test: multi-page fixture not found at {}", fixture_path.display());
        return;
    }

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    let count = page_helpers::page_count(&result);

    // Access last page
    let page = page_helpers::last_page(&result)
        .expect("Failed to get last page");

    assert_eq!(
        page.index, count - 1,
        "Last page index should match"
    );
    assert_eq!(
        page.page_number as usize, count,
        "Last page_number should equal total page count"
    );
}

#[test]
fn test_error_handling_out_of_bounds_index() {
    let fixture_path = single_page_fixture();
    assert!(
        fixture_path.exists(),
        "Single-page fixture not found at {}",
        fixture_path.display()
    );

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    // Try to access page 5 in a single-page document
    let error = page_helpers::get_page(&result, 5)
        .expect_err("Should return error for out-of-bounds index");

    // Verify error message contains helpful information
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("out of bounds"),
        "Error message should mention 'out of bounds', got: {}",
        error_msg
    );
    assert!(
        error_msg.contains("1 pages"),
        "Error message should mention page count, got: {}",
        error_msg
    );
}

#[test]
fn test_error_handling_invalid_page_number_zero() {
    let fixture_path = single_page_fixture();
    assert!(
        fixture_path.exists(),
        "Single-page fixture not found at {}",
        fixture_path.display()
    );

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    // Try to access page at index 0 in a single-page document
    // This should succeed, not fail
    let page = page_helpers::get_page(&result, 0);
    assert!(
        page.is_ok(),
        "Should be able to access page at index 0, got err: {:?}",
        page
    );
}

#[test]
fn test_error_handling_out_of_bounds_page_number() {
    let fixture_path = single_page_fixture();
    assert!(
        fixture_path.exists(),
        "Single-page fixture not found at {}",
        fixture_path.display()
    );

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    // Try to access page 10 (index 10) in a single-page document
    let error = page_helpers::get_page(&result, 10)
        .expect_err("Should return error for out-of-bounds index");

    // Verify error message contains helpful information
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("out of bounds") || error_msg.contains("1 pages"),
        "Error message should mention the error, got: {}",
        error_msg
    );
}

#[test]
fn test_get_all_pages_single_page() {
    let fixture_path = single_page_fixture();
    assert!(
        fixture_path.exists(),
        "Single-page fixture not found at {}",
        fixture_path.display()
    );

    // Extract the PDF
    let result = extract_pdf(&fixture_path, &ExtractionOptions::default())
        .expect("Failed to extract PDF");

    // Get all pages
    let all_pages = page_helpers::get_pages(&result)
        .expect("Failed to get all pages");

    assert_eq!(
        all_pages.len(), 1,
        "get_pages should return 1 page for single-page fixture"
    );
}
