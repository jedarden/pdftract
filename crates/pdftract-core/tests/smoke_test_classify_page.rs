//! Smoke test for classify_page functionality
//!
//! This test verifies that the page classification system works end-to-end.
//! It serves as a foundational smoke test that validates:
//! - PageContext construction with basic page data
//! - classify_page function execution
//! - PageClassification output structure and validity
//!
//! The test uses manually constructed PageContext instances to ensure fast
//! execution and reliability. This is the first test to run when validating
//! the classification pipeline.

use pdftract_core::classify::{classify_page, PageClass, PageContext};
use std::path::Path;

#[test]
fn test_classify_basic_vector_page() {
    //! Verify classify_page works for a basic vector PDF page.
    //!
    //! This smoke test validates:
    //! - PageContext can be constructed with valid page data
    //! - classify_page executes successfully (no Result/Ok, direct return)
    //! - Output contains valid PageClassification structure
    //! - Classification returns expected PageClass for vector page
    //!
    //! Uses a simple vector page scenario: text-only, born-digital PDF.

    let mut ctx = PageContext::new();
    // Simple vector page: text-only, born-digital PDF
    ctx.text_op_count = 100;
    ctx.raw_char_count = 500;
    ctx.valid_char_count = 490; // 98% validity
    ctx.replacement_char_count = 10;
    ctx.image_coverage = 0.0; // No images
    ctx.has_full_page_image = false;
    ctx.has_visible_text = true;
    ctx.density_ratio = 0.90; // High character density
    ctx.width = 612.0; // US Letter
    ctx.height = 792.0;
    ctx.rotation = 0;

    // Run classification
    let result = classify_page(&ctx);

    // Verify classification succeeded (direct return, no Result wrapper)
    assert_eq!(
        result.class,
        PageClass::Vector,
        "Simple vector page should classify as Vector"
    );

    // Verify confidence is in valid range [0.0, 1.0]
    assert!(
        result.confidence >= 0.0 && result.confidence <= 1.0,
        "Confidence should be in range [0.0, 1.0], got: {}",
        result.confidence
    );

    // Verify hybrid_cells is None for non-Hybrid classification
    assert!(
        result.hybrid_cells.is_none(),
        "hybrid_cells should be None for Vector classification"
    );

    println!("✓ classify_page returned valid PageClassification: {:?}", result);
}

#[test]
fn test_classify_basic_scanned_page() {
    //! Verify classify_page works for a basic scanned PDF page.
    //!
    //! This smoke test validates classification for image-only pages.
    //! Provides redundancy using a scanned page scenario to ensure
    //! the classification pipeline works across different page types.

    let mut ctx = PageContext::new();
    // Scanned page: image-only, no text
    ctx.text_op_count = 0;
    ctx.raw_char_count = 0;
    ctx.valid_char_count = 0;
    ctx.replacement_char_count = 0;
    ctx.image_coverage = 0.95; // High image coverage
    ctx.image_xobject_areas = vec![50000.0]; // Large image area
    ctx.has_full_page_image = true;
    ctx.has_visible_text = false;
    ctx.density_ratio = 0.0;
    ctx.width = 612.0;
    ctx.height = 792.0;
    ctx.rotation = 0;

    // Run classification
    let result = classify_page(&ctx);

    // Verify classification succeeded
    assert_eq!(
        result.class,
        PageClass::Scanned,
        "Image-only page should classify as Scanned"
    );

    // Verify confidence is in valid range
    assert!(
        result.confidence >= 0.0 && result.confidence <= 1.0,
        "Confidence should be in range [0.0, 1.0], got: {}",
        result.confidence
    );

    println!("✓ classify_page correctly classified scanned page: {:?}", result);
}

#[test]
fn test_classify_page_fixture_exists() {
    //! Verify that the classify_page fixture file exists for future integration tests.
    //!
    //! This test ensures the PDF fixture created in the previous step (bf-32xr9i)
    //! is available for more comprehensive integration tests that will parse
    //! actual PDF files and construct PageContext from real content streams.

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // Go up from crates/pdftract-core to workspace root, then to fixtures
    let fixture_path = format!("{}/../../tests/fixtures/classify_page_simple.pdf", manifest_dir);
    let fixture_path = Path::new(&fixture_path);

    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {} for future integration tests",
        fixture_path.display()
    );

    println!("✓ Fixture file exists: {}", fixture_path.display());

    // Note: Full integration test that loads PDF and constructs PageContext
    // from actual content streams will be added in subsequent work.
    // This smoke test validates the fixture is available for that work.
}
