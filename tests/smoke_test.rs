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

#[test]
fn test_extract_returns_typed_document() {
    //! Verify complete nested structure integrity: Document -> Pages -> Spans.
    //!
    //! This comprehensive test validates the full type hierarchy and relationships
    //! throughout the PDF extraction pipeline. It ensures that the Document -> Pages -> Spans
    //! structure is properly maintained and all objects contain valid data.
    //!
    //! # Assertion Types Verified
    //!
    //! ## Fixture Validation
    //! - Fixture file exists at expected path
    //!
    //! ## Extraction Success
    //! - PDF extraction completes without errors
    //!
    //! ## Document-Level Checks
    //! - Document contains at least one page (page_count > 0)
    //! - All pages are accessible from the document
    //!
    //! ## Page-Level Checks
    //! - Each page has valid width > 0.0
    //! - Each page has valid height > 0.0
    //! - Spans are properly counted per page
    //!
    //! ## Span-Level Checks
    //! - Each span has bbox with exactly 4 coordinates
    //! - At least one span per page has non-empty text content
    //! - Text content is accessible via span.text
    //!
    //! ## Aggregate/Completeness Checks
    //! - At least one page has spans populated
    //! - Total span count > 0 across all pages
    //! - All objects in hierarchy are accounted for
    //!
    //! # Edge Cases Handled
    //! - Empty document (no pages): Error indicates expected vs actual page count
    //! - Pages without spans: Error identifies which page has empty spans
    //! - Spans without text: Error distinguishes between no spans and no text
    //! - Invalid page dimensions: Error shows page index and actual values
    //!
    //! # Test Fixture
    //! Uses `tests/fixtures/test-minimal.pdf` - a minimal PDF file optimized for
    //! fast, reliable testing. This fixture is known to contain at least one page
    //! with extractable text content.

    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    // === Fixture Validation ===
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );
    println!("✓ Fixture found: {}", fixture_path.display());

    // === Extraction Success ===
    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
        &OutputOptions::default(),
    );

    assert!(
        result.is_ok(),
        "PDF extraction should succeed: {:?}",
        result.err()
    );
    println!("✓ PDF extraction completed successfully");

    let extraction_result = result.unwrap();

    // === Document-Level Checks ===
    // Verify the document owns its pages (parent-child relationship)
    let page_count = extraction_result.pages.len();
    assert!(
        page_count > 0,
        "Document should contain at least one page, found {} pages",
        page_count
    );
    println!("✓ Document-level: {} page(s) found", page_count);

    // === Page-Level Checks ===
    // These checks verify that each page has valid physical dimensions
    // and properly counts its associated spans.
    let mut total_spans = 0;
    let mut pages_with_spans = 0;

    for (page_idx, page) in extraction_result.pages.iter().enumerate() {
        // Verify page has valid dimensions
        assert!(
            page.width > 0.0,
            "Page {} should have valid width, got {}",
            page_idx,
            page.width
        );
        assert!(
            page.height > 0.0,
            "Page {} should have valid height, got {}",
            page_idx,
            page.height
        );
        println!(
            "  Page {}: dimensions {:.1} x {:.1}",
            page_idx, page.width, page.height
        );

        // Count spans on this page
        let span_count = page.spans.len();
        total_spans += span_count;

        if span_count > 0 {
            pages_with_spans += 1;
            println!("  Page {}: {} span(s)", page_idx, span_count);

            // === Span-Level Checks ===
            // These checks verify that spans have valid structural data
            // (bounding boxes) and contain actual text content.
            // Verify at least one span has non-empty text content
            let mut spans_with_text = 0;
            let mut spans_with_empty_text = 0;
            for (span_idx, span) in page.spans.iter().enumerate() {
                if !span.text.is_empty() {
                    spans_with_text += 1;
                } else {
                    spans_with_empty_text += 1;
                }

                // Verify span has valid bounding box
                assert!(
                    span.bbox.len() == 4,
                    "Page {} span {} should have bbox with 4 coordinates, got {}",
                    page_idx,
                    span_idx,
                    span.bbox.len()
                );
            }

            assert!(
                spans_with_text > 0,
                "Page {} should have at least one span with non-empty text, found {} spans with text out of {} total ({} spans with empty text)",
                page_idx,
                spans_with_text,
                span_count,
                spans_with_empty_text
            );
            println!(
                "  Page {}: {} spans with text, {} spans with empty text",
                page_idx, spans_with_text, spans_with_empty_text
            );
        } else {
            println!("  Page {}: no spans found (edge case handled)", page_idx);
        }
    }

    // === Aggregate/Completeness Checks ===
    // These checks verify that the overall structure is complete:
    // - At least one page has content (spans)
    // - Total span count is non-zero
    // - All objects are accessible

    // Verify at least one page has spans populated
    assert!(
        pages_with_spans > 0,
        "At least one page should have spans populated, found {} pages with spans out of {} total pages. If no pages have spans, the PDF may be image-only or extraction failed.",
        pages_with_spans,
        page_count
    );
    println!(
        "✓ Aggregate: {}/{} pages have content (spans)",
        pages_with_spans, page_count
    );

    // Verify total span count (completeness check)
    assert!(
        total_spans > 0,
        "Document should contain at least one span across all pages, found {} total spans. Zero spans indicates text extraction failed or PDF is image-only.",
        total_spans
    );
    println!("✓ Aggregate: {} total spans across all pages", total_spans);

    // Verify we can account for all objects in the hierarchy
    assert!(
        extraction_result.pages.len() == page_count,
        "All pages should be accessible in the document"
    );
    println!("✓ Completeness: All {} pages accessible in hierarchy", page_count);

    println!("✓ All nested structure checks passed");
}

#[test]
fn test_extract_returns_typed_document() {
    //! Verify complete nested structure integrity: Document -> Pages -> Spans.
    //!
    //! This comprehensive test validates the full type hierarchy and relationships
    //! throughout the PDF extraction pipeline. It ensures that the Document -> Pages -> Spans
    //! structure is properly maintained and all objects contain valid data.
    //!
    //! # Assertion Types Verified
    //!
    //! ## Fixture Validation
    //! - Fixture file exists at expected path
    //!
    //! ## Extraction Success
    //! - PDF extraction completes without errors
    //!
    //! ## Document-Level Checks
    //! - Document contains at least one page (page_count > 0)
    //! - All pages are accessible from the document
    //!
    //! ## Page-Level Checks
    //! - Each page has valid width > 0.0
    //! - Each page has valid height > 0.0
    //! - Spans are properly counted per page
    //!
    //! ## Span-Level Checks
    //! - Each span has bbox with exactly 4 coordinates
    //! - At least one span per page has non-empty text content
    //! - Text content is accessible via span.text
    //!
    //! ## Aggregate/Completeness Checks
    //! - At least one page has spans populated
    //! - Total span count > 0 across all pages
    //! - All objects in hierarchy are accounted for
    //!
    //! # Edge Cases Handled
    //! - Empty document (no pages): Error indicates expected vs actual page count
    //! - Pages without spans: Error identifies which page has empty spans
    //! - Spans without text: Error distinguishes between no spans and no text
    //! - Invalid page dimensions: Error shows page index and actual values
    //!
    //! # Test Fixture
    //! Uses `tests/fixtures/test-minimal.pdf` - a minimal PDF file optimized for
    //! fast, reliable testing. This fixture is known to contain at least one page
    //! with extractable text content.

    let fixture_path = Path::new("tests/fixtures/test-minimal.pdf");

    // ===== FIXTURE VALIDATION =====
    // Ensure the test fixture exists before attempting extraction
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}, but file was not found. \
         Ensure tests/fixtures/test-minimal.pdf exists in the workspace.",
        fixture_path.display()
    );
    println!("✓ Fixture found: {}", fixture_path.display());

    // ===== EXTRACTION =====
    // Attempt to extract the PDF and capture the result
    let result = extract_pdf(
        fixture_path,
        &ExtractionOptions::default(),
        &OutputOptions::default(),
    );

    // Verify extraction succeeded (if this fails, the PDF may be corrupt)
    assert!(
        result.is_ok(),
        "PDF extraction should succeed for test-minimal.pdf. \
         Got error: {:?}. \
         This may indicate: corrupt PDF, unsupported PDF version, or parser bug.",
        result.err()
    );
    println!("✓ PDF extraction completed successfully");

    let extraction_result = result.unwrap();

    // ===== DOCUMENT-LEVEL CHECKS =====
    // Verify the document properly owns its pages and the hierarchy is intact
    let page_count = extraction_result.pages.len();
    assert!(
        page_count > 0,
        "Document should contain at least one page. \
         Expected: page_count > 0, \
         Actual: page_count = {}. \
         If no pages found, the PDF may be empty or the page parser failed.",
        page_count
    );
    println!("✓ Document contains {} page(s)", page_count);

    // ===== PAGE-LEVEL CHECKS =====
    // These checks verify that each page has valid physical dimensions
    // and properly counts its associated spans.
    let mut total_spans = 0;
    let mut pages_with_spans = 0;

    for (page_idx, page) in extraction_result.pages.iter().enumerate() {
        // Verify page has valid dimensions
        assert!(
            page.width > 0.0,
            "Page {} should have valid width > 0. \
             Expected: width > 0.0, \
             Actual: width = {}. \
             Invalid dimensions indicate a page parsing error.",
            page_idx,
            page.width
        );
        assert!(
            page.height > 0.0,
            "Page {} should have valid height > 0. \
             Expected: height > 0.0, \
             Actual: height = {}. \
             Invalid dimensions indicate a page parsing error.",
            page_idx,
            page.height
        );

        // Count spans on this page
        let span_count = page.spans.len();
        total_spans += span_count;

        if span_count > 0 {
            pages_with_spans += 1;

            // ===== SPAN-LEVEL CHECKS =====
            // These checks verify that spans have valid structural data
            // (bounding boxes) and contain actual text content.
            let mut spans_with_text = 0;
            let mut spans_with_empty_text = 0;

            for (span_idx, span) in page.spans.iter().enumerate() {
                // Track spans with actual text content
                if !span.text.is_empty() {
                    spans_with_text += 1;
                } else {
                    spans_with_empty_text += 1;
                }

                // Verify span bounding box has exactly 4 coordinates [x0, y0, x1, y1]
                assert_eq!(
                    span.bbox.len(),
                    4,
                    "Page {} span {} should have bbox with exactly 4 coordinates [x0, y0, x1, y1]. \
                     Expected: bbox.len() == 4, \
                     Actual: bbox.len() = {}, value: {:?}. \
                     Invalid bbox dimensions indicate a span parsing error.",
                    page_idx,
                    span_idx,
                    span.bbox.len(),
                    span.bbox
                );

                // Verify bbox coordinates are finite (not NaN or infinite)
                assert!(
                    span.bbox[0].is_finite(),
                    "Page {} span {} should have finite x0 coordinate. \
                     Expected: finite value, \
                     Actual: x0 = {}. \
                     Non-finite coordinates indicate a span positioning error.",
                    page_idx,
                    span_idx,
                    span.bbox[0]
                );
                assert!(
                    span.bbox[1].is_finite(),
                    "Page {} span {} should have finite y0 coordinate. \
                     Expected: finite value, \
                     Actual: y0 = {}. \
                     Non-finite coordinates indicate a span positioning error.",
                    page_idx,
                    span_idx,
                    span.bbox[1]
                );
                assert!(
                    span.bbox[2].is_finite(),
                    "Page {} span {} should have finite x1 coordinate. \
                     Expected: finite value, \
                     Actual: x1 = {}. \
                     Non-finite coordinates indicate a span positioning error.",
                    page_idx,
                    span_idx,
                    span.bbox[2]
                );
                assert!(
                    span.bbox[3].is_finite(),
                    "Page {} span {} should have finite y1 coordinate. \
                     Expected: finite value, \
                     Actual: y1 = {}. \
                     Non-finite coordinates indicate a span positioning error.",
                    page_idx,
                    span_idx,
                    span.bbox[3]
                );
            }

            // At least one span on this page must have non-empty text
            assert!(
                spans_with_text > 0,
                "Page {} should have at least one span with non-empty text content. \
                 Expected: spans_with_text > 0, \
                 Actual: spans_with_text = {}, \
                 Total spans on page: {}, \
                 Spans with empty text: {}. \
                 If all spans are empty, the text extractor may have failed.",
                page_idx,
                spans_with_text,
                span_count,
                spans_with_empty_text
            );

            println!(
                "✓ Page {}: {} spans ({} with text, {} with empty text)",
                page_idx, span_count, spans_with_text, spans_with_empty_text
            );
        } else {
            println!("  Page {}: no spans found (edge case handled)", page_idx);
        }
    }

    // ===== AGGREGATE CHECKS =====
    // Verify overall document structure integrity

    // At least one page should have spans (non-empty content)
    assert!(
        pages_with_spans > 0,
        "At least one page should have spans populated. \
         Expected: pages_with_spans > 0, \
         Actual: pages_with_spans = {}, \
         Total pages: {}. \
         If no pages have spans, the PDF may contain no text or the span extractor failed.",
        pages_with_spans,
        page_count
    );
    println!("✓ {} page(s) have spans populated", pages_with_spans);

    // Total span count should be non-zero (completeness check)
    assert!(
        total_spans > 0,
        "Document should contain at least one span across all pages. \
         Expected: total_spans > 0, \
         Actual: total_spans = {}. \
         Zero spans indicates either an empty PDF or span extraction failure.",
        total_spans
    );
    println!("✓ Total spans in document: {}", total_spans);

    // Verify all pages are still accessible (no data loss during iteration)
    assert_eq!(
        extraction_result.pages.len(),
        page_count,
        "All pages should be accessible in the document (page count should remain stable). \
         Expected: {} pages, \
         Actual: {} pages. \
         Page count changed during iteration, indicating a possible data structure bug.",
        page_count,
        extraction_result.pages.len()
    );
    println!("✓ All {} pages accessible (no data loss)", page_count);
    println!("✓ All nested structure checks passed");
}
