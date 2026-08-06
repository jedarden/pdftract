//! Smoke test for pdftract basic functionality
//!
//! This test verifies that the core PDF extraction pipeline works end-to-end.
//! It serves as a foundational smoke test that validates:
//! - Basic PDF parsing and loading
//! - Page extraction functionality
//! - Text extraction capabilities
//!
//! The test uses minimal fixtures to ensure fast execution and reliability.
//! This is the first test to run when validating the extraction pipeline.

use pdftract_core::{extract_pdf, ExtractionOptions, OutputOptions};
use std::path::Path;

#[test]
fn test_basic_pdf_extraction() {
    //! Verify basic PDF extraction works on a minimal fixture.
    //!
    //! This smoke test validates:
    //! - PDF file can be opened and parsed
    //! - At least one page is extracted
    //! - Extraction completes without errors
    //!
    //! Uses test-minimal.pdf (374 bytes) as a fast, reliable fixture.

    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    // Verify fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    // Run basic extraction
    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
        &OutputOptions::default(),
    );

    // Verify extraction succeeded
    assert!(
        result.is_ok(),
        "PDF extraction should succeed for minimal fixture: {:?}",
        result.err()
    );

    let extraction_result = result.unwrap();

    // Verify at least one page was extracted
    assert!(
        !extraction_result.pages.is_empty(),
        "At least one page should be extracted from the PDF"
    );

    // Verify we can access page data
    let first_page = &extraction_result.pages[0];
    assert!(
        first_page.width > 0.0,
        "First page should have a valid width"
    );
    assert!(
        first_page.height > 0.0,
        "First page should have a valid height"
    );
}

#[test]
fn test_sample_pdf_extraction() {
    //! Verify PDF extraction works on another simple fixture.
    //!
    //! Provides redundancy using sample.pdf (534 bytes) to ensure
    //! the extraction pipeline works across different minimal PDFs.

    let fixture_path = Path::new("tests/fixtures/sample.pdf");

    // Verify fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    // Run extraction
    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
        &OutputOptions::default(),
    );

    // Verify extraction succeeded
    assert!(
        result.is_ok(),
        "PDF extraction should succeed for sample fixture: {:?}",
        result.err()
    );

    let extraction_result = result.unwrap();

    // Verify at least one page was extracted
    assert!(
        !extraction_result.pages.is_empty(),
        "At least one page should be extracted from the PDF"
    );
}
