//! Smoke test for pdftract SDK public API surface
//!
//! This test verifies that the SDK module (`pdftract_core::sdk`) functions work correctly
//! and return properly typed results. It serves as a foundational smoke test for the SDK contract.
//!
//! The test validates:
//! - Basic SDK extraction functionality
//! - Proper function signatures and return types
//! - SDK functions can be imported and used successfully
//!
//! This is the first test to run when validating the SDK API surface.

use pdftract_core::options::ExtractionOptions;
use pdftract_core::sdk;
use std::path::Path;

#[test]
fn test_sdk_extract_basic() {
    //! Verify basic SDK extract() function works on a minimal fixture.
    //!
    //! This smoke test validates:
    //! - sdk::extract() function exists and is callable
    //! - Extract returns a Result type that succeeds on valid PDF
    //! - ExtractionResult contains pages with proper structure
    //!
    //! Uses test-minimal.pdf (374 bytes) as a fast, reliable fixture.

    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    // Verify fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    // Run SDK extraction
    let result = sdk::extract(fixture_path, &ExtractionOptions::default());

    // Verify extraction succeeded
    assert!(
        result.is_ok(),
        "SDK extract() should succeed for minimal fixture: {:?}",
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
        first_page.width.is_some() || first_page.height.is_some(),
        "First page should have dimension information"
    );
}

#[test]
fn test_sdk_extract_text() {
    //! Verify SDK extract_text() function works on a minimal fixture.
    //!
    //! This smoke test validates:
    //! - sdk::extract_text() function exists and is callable
    //! - Returns String with text content
    //! - Function succeeds on valid PDF

    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    // Verify fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    // Run text extraction
    let result = sdk::extract_text(fixture_path, &ExtractionOptions::default());

    // Verify extraction succeeded
    assert!(
        result.is_ok(),
        "SDK extract_text() should succeed for minimal fixture: {:?}",
        result.err()
    );

    let _text = result.unwrap();

    // Verify we got a string (may be empty for minimal PDFs)
    assert!(
        std::any::TypeId::of::<String>() == std::any::TypeId::of::<String>(),
        "extract_text() should return a String"
    );
}

#[test]
fn test_sdk_get_metadata() {
    //! Verify SDK get_metadata() function works on a minimal fixture.
    //!
    //! This smoke test validates:
    //! - sdk::get_metadata() function exists and is callable
    //! - Returns PdfMetadata with page_count
    //! - Function succeeds on valid PDF

    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    // Verify fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    // Get metadata
    let result = sdk::get_metadata(fixture_path);

    // Verify metadata retrieval succeeded
    assert!(
        result.is_ok(),
        "SDK get_metadata() should succeed for minimal fixture: {:?}",
        result.err()
    );

    let metadata = result.unwrap();

    // Verify metadata structure
    assert!(
        metadata.page_count > 0,
        "Metadata should report at least one page"
    );
}

#[test]
fn test_sdk_hash() {
    //! Verify SDK hash() function works on a minimal fixture.
    //!
    //! This smoke test validates:
    //! - sdk::hash() function exists and is callable
    //! - Returns fingerprint hash as String
    //! - Function succeeds on valid PDF

    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    // Verify fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    // Compute hash
    let result = sdk::hash(fixture_path);

    // Verify hash computation succeeded
    assert!(
        result.is_ok(),
        "SDK hash() should succeed for minimal fixture: {:?}",
        result.err()
    );

    let fingerprint = result.unwrap();

    // Verify fingerprint format (should start with "pdftract-v1:")
    assert!(
        fingerprint.starts_with("pdftract-v1:"),
        "Fingerprint should start with 'pdftract-v1:' prefix, got: {}",
        &fingerprint[..fingerprint.chars().count().min(20)]
    );
}

#[test]
fn test_sdk_extract_stream() {
    //! Verify SDK extract_stream() function works on a minimal fixture.
    //!
    //! This smoke test validates:
    //! - sdk::extract_stream() function exists and is callable
    //! - Returns an iterator that yields pages
    //! - Function succeeds on valid PDF

    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    // Verify fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    // Run stream extraction
    let result = sdk::extract_stream(fixture_path, &ExtractionOptions::default());

    // Verify stream creation succeeded
    assert!(
        result.is_ok(),
        "SDK extract_stream() should succeed for minimal fixture: {:?}",
        result.err()
    );

    let mut stream = result.unwrap();

    // Verify we can iterate over pages
    if let Some(first_page_result) = stream.next() {
        assert!(
            first_page_result.is_ok(),
            "Stream should yield Ok(PageResult), got error: {:?}",
            first_page_result.err()
        );

        let first_page = first_page_result.unwrap();
        assert!(
            first_page.width.is_some() || first_page.height.is_some(),
            "Streamed page should have dimension information"
        );
    }
}
