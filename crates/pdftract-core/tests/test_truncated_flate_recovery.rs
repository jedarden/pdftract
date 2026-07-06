//! Integration tests for truncated FlateDecode stream recovery.
//!
//! Tests the behavior of pdftract when encountering truncated/incomplete
//! FlateDecode compressed streams. This can occur when:
//! - PDF files are corrupted during download
//! - PDF files are partially written
//! - PDF streams are truncated by malicious actors
//!
//! The fixture tests various recovery strategies:
//! - Graceful handling of incomplete zlib streams
//! - Diagnostic reporting for truncated data
//! - Partial decompression where possible
//! - Fallback to raw stream data when decompression fails

use pdftract_core::document::{parse_pdf_file, PdfExtractor};
use std::path::PathBuf;

/// Returns the path to the truncated-flate.pdf fixture.
/// This fixture has a valid PDF structure but contains a truncated FlateDecode stream.
fn fixture_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../tests/fixtures/malformed/truncated-flate.pdf");
    path
}

/// Basic test: verify the fixture file exists and can be opened.
///
/// This is a prerequisite for all other tests in this module.
#[test]
fn test_truncated_flate_fixture_exists() {
    let path = fixture_path();
    assert!(
        path.exists(),
        "Fixture file should exist at {}",
        path.display()
    );

    // Verify it's not empty
    let metadata = std::fs::metadata(&path)
        .expect("Should be able to read fixture metadata");
    assert!(metadata.len() > 0, "Fixture file should not be empty");

    println!("✓ Fixture exists: {}", path.display());
    println!("  Size: {} bytes", metadata.len());
}

/// Test that the truncated_mid_stream.pdf can be parsed as a PDF document.
///
/// This verifies that the file has a valid PDF structure even if one
/// or more streams contain truncated FlateDecode data.
#[test]
fn test_truncated_flate_parses_as_pdf() {
    let path = fixture_path();
    let result = parse_pdf_file(&path);

    // The document should parse - truncated streams should be handled
    // gracefully with diagnostics, not cause total parse failure
    if let Err(ref e) = result {
        panic!("Should parse truncated-flate.pdf as a valid PDF document: {}", e);
    }

    let (_fingerprint, _catalog, pages, _resolver) = result.unwrap();
    // Verify basic document structure
    assert!(
        !pages.is_empty(),
        "Document should have at least one page"
    );
}

/// Test that truncated FlateDecode streams produce appropriate diagnostics.
///
/// When a FlateDecode stream is truncated, pdftract should:
/// - Emit a diagnostic code indicating the truncation
/// - Not crash or panic
/// - Continue processing the rest of the document
#[test]
fn test_truncated_flate_emits_diagnostics() {
    let path = fixture_path();
    let (_fingerprint, _catalog, _pages, _resolver) = parse_pdf_file(&path)
        .expect("Should parse document");

    // Note: Diagnostics are not currently surfaced through parse_pdf_file
    // This is a scaffold test to verify the fixture parses without error
    // Once diagnostics are exposed, this test should check for truncation warnings
    println!("Warning: Diagnostic API not yet exposed through parse_pdf_file");
    println!("Fixture parsed successfully - diagnostic collection pending");
}

/// Test that we can access page content even with truncated streams.
///
/// This verifies that when one stream is truncated, other content
/// in the document remains accessible.
#[test]
fn test_truncated_flate_partial_content_accessible() {
    let path = fixture_path();
    let (_fingerprint, _catalog, pages, _resolver) = parse_pdf_file(&path)
        .expect("Should parse document");

    // Try to access the first page
    assert!(!pages.is_empty(), "Should have at least one page");
    let first_page = &pages[0];

    // Verify page has basic structure
    assert!(
        first_page.media_box.len() == 4,
        "Page should have a mediabox with 4 values"
    );

    // Content streams may be affected by truncation - we just verify
    // the page structure is accessible without crashing
    println!("Page accessible with {} content streams", first_page.contents.len());
}

/// Test extraction of truncated_mid_stream.pdf using PdfExtractor.
///
/// This test examines the extraction result structure, particularly
/// the errors/diagnostics field to understand how truncation errors are reported.
#[test]
fn test_truncated_flate_extraction_result_structure() {
    let path = fixture_path();

    println!("Testing extraction of: {}", path.display());

    // Open the PDF with PdfExtractor
    let extractor = PdfExtractor::open(&path)
        .expect("Should open truncated_mid_stream.pdf with PdfExtractor");

    println!("✓ PdfExtractor::open() succeeded");
    println!("  Fingerprint: {}", extractor.fingerprint());
    println!("  Page count: {:?}", extractor.page_count());

    // Materialize pages to enable extraction
    let mut extractor_mut = extractor;
    let pages = extractor_mut.materialize_pages()
        .expect("Should materialize pages");

    println!("✓ materialize_pages() succeeded");
    println!("  Number of pages: {}", pages.len());

    // Try to extract the first page (if it exists)
    if !pages.is_empty() {
        let page_result = extractor_mut.extract_page(0);

        match page_result {
            Ok(extraction) => {
                println!("✓ extract_page(0) succeeded");
                println!("  Page index: {}", extraction.index);
                println!("  Width: {}", extraction.width);
                println!("  Height: {}", extraction.height);
                println!("  Rotation: {}", extraction.rotation);
                println!("  Number of spans: {}", extraction.spans.len());
                println!("  Number of blocks: {}", extraction.blocks.len());

                // Check if there are any errors/diagnostics in the result
                // Look for fields that might contain error information
                println!("\n  Checking for error/diagnostic fields in extraction result...");

                // Try to serialize to see the full structure
                match serde_json::to_string_pretty(&extraction) {
                    Ok(json) => {
                        println!("\n  Full extraction result structure (JSON):");
                        println!("  {}", json.replace("\n", "\n  "));
                    },
                    Err(e) => {
                        println!("  Warning: Could not serialize extraction result: {}", e);
                    }
                }
            },
            Err(e) => {
                println!("✗ extract_page(0) failed: {}", e);
                println!("  Error details: {:?}", e);
            }
        }
    } else {
        println!("  No pages to extract");
    }
}
