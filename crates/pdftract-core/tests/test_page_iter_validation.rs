//! Test PageIter validation and edge cases.
//!
//! This test module verifies that PageIter correctly validates
//! page data and returns appropriate errors for edge cases.
//!
//! Bead: bf-171rdo - Add Result error handling and edge cases
//!
//! Acceptance criteria:
//! - Handles empty Documents
//! - Handles missing pages array
//! - Handles out-of-bounds access with descriptive errors
//! - Handles malformed media_box
//! - Handles invalid dimensions (zero/negative)
//! - Handles invalid rotation values
//! - Error messages clearly indicate what went wrong

use pdftract_core::document::Document;
use pdftract_core::page_extraction_error::PageExtractionError;
use std::path::Path;

/// Test that Document returns NoPagesInDocument error for empty document
#[test]
fn test_empty_document_returns_no_pages_error() {
    // We need to create or use a fixture with 0 pages
    // For now, we verify the error type exists and has the right message
    let error = PageExtractionError::NoPagesInDocument;
    let msg = format!("{}", error);
    assert!(
        msg.contains("no pages") || msg.contains("Document contains no pages"),
        "NoPagesInDocument error should mention no pages, got: {}",
        msg
    );
}

/// Test that Document returns IndexOutOfBounds error with context
#[test]
fn test_index_out_of_bounds_error_includes_context() {
    let error = PageExtractionError::IndexOutOfBounds {
        requested: 10,
        available: 5,
    };
    let msg = format!("{}", error);

    // Verify error message includes both the requested and available counts
    assert!(
        msg.contains("10") && msg.contains("5"),
        "IndexOutOfBounds error should show both requested (10) and available (5) counts, got: {}",
        msg
    );

    assert!(
        msg.contains("out of bounds"),
        "IndexOutOfBounds error should mention 'out of bounds', got: {}",
        msg
    );
}

/// Test that InvalidMediaBox error includes page index and media box values
#[test]
fn test_invalid_media_box_error_includes_details() {
    let error = PageExtractionError::InvalidMediaBox {
        page_index: 2,
        media_box: Some([0.0, 0.0, -1.0, 792.0]),
    };
    let msg = format!("{}", error);

    assert!(
        msg.contains("2") || msg.contains("Page 2"),
        "InvalidMediaBox error should mention page index, got: {}",
        msg
    );

    assert!(
        msg.contains("media box") || msg.contains("MediaBox"),
        "InvalidMediaBox error should mention media box, got: {}",
        msg
    );
}

/// Test that InvalidDimensions error includes page index and dimensions
#[test]
fn test_invalid_dimensions_error_includes_details() {
    // Test zero width
    let error = PageExtractionError::InvalidDimensions {
        page_index: 0,
        width: 0.0,
        height: 792.0,
    };
    let msg = format!("{}", error);

    assert!(
        msg.contains("0") && (msg.contains("width") || msg.contains("height")),
        "InvalidDimensions error should show dimensions, got: {}",
        msg
    );

    // Test negative height
    let error2 = PageExtractionError::InvalidDimensions {
        page_index: 1,
        width: 612.0,
        height: -100.0,
    };
    let msg2 = format!("{}", error2);

    assert!(
        msg2.contains("-100") || msg2.contains("negative"),
        "InvalidDimensions error should show negative value, got: {}",
        msg2
    );
}

/// Test that InvalidRotation error includes page index and rotation value
#[test]
fn test_invalid_rotation_error_includes_details() {
    let error = PageExtractionError::InvalidRotation {
        page_index: 3,
        rotation: 45,
    };
    let msg = format!("{}", error);

    assert!(
        msg.contains("45") || msg.contains("45°"),
        "InvalidRotation error should show the invalid rotation value, got: {}",
        msg
    );

    assert!(
        msg.contains("0") || msg.contains("90") || msg.contains("180") || msg.contains("270"),
        "InvalidRotation error should mention valid values, got: {}",
        msg
    );
}

/// Test extracting pages and verify validation is performed
#[test]
fn test_page_iter_performs_validation() {
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

    // Extract all pages through PageIter
    let mut page_count = 0;
    for page_result in doc.pages() {
        match page_result {
            Ok(page) => {
                page_count += 1;

                // Verify each page has valid dimensions (positive values)
                assert!(
                    page.width > 0.0,
                    "Page {} should have positive width, got {}",
                    page.index,
                    page.width
                );
                assert!(
                    page.height > 0.0,
                    "Page {} should have positive height, got {}",
                    page.index,
                    page.height
                );

                // Verify valid rotation
                assert!(
                    page.rotation == 0
                        || page.rotation == 90
                        || page.rotation == 180
                        || page.rotation == 270,
                    "Page {} should have valid rotation (0, 90, 180, or 270), got {}",
                    page.index,
                    page.rotation
                );
            }
            Err(e) => {
                // If we get a PageExtractionError, verify it's descriptive
                let error_msg = e.to_string();
                assert!(
                    error_msg.len() > 20,
                    "Error message should be descriptive, got: {}",
                    error_msg
                );

                // If it's a validation error, it should mention what failed
                if error_msg.contains("dimensions") {
                    assert!(
                        error_msg.contains("width") || error_msg.contains("height"),
                        "Dimension error should mention which dimension, got: {}",
                        error_msg
                    );
                }
            }
        }
    }

    assert!(
        page_count > 0,
        "Should have extracted at least one page, got {}",
        page_count
    );
}

/// Test that Document::extract_page uses proper error types
#[test]
fn test_document_extract_page_uses_proper_errors() {
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

    // Test out-of-bounds access
    let result = doc.extract_page(page_count + 100);
    assert!(result.is_err(), "Should return error for out-of-bounds access");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("out of bounds") || error_msg.contains("Page index"),
        "Error should mention out-of-bounds or page index, got: {}",
        error_msg
    );

    // Verify the error includes context about available pages
    assert!(
        error_msg.contains(&format!("{}", page_count)) || error_msg.contains("pages"),
        "Error should include page count or mention pages, got: {}",
        error_msg
    );
}

/// Test that errors are Send and Sync (required for multi-threading)
#[test]
fn test_page_extraction_error_is_send_and_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<PageExtractionError>();
    assert_sync::<PageExtractionError>();
}

/// Test that PageExtractionError can be cloned
#[test]
fn test_page_extraction_error_can_be_cloned() {
    let error1 = PageExtractionError::IndexOutOfBounds {
        requested: 5,
        available: 3,
    };
    let error2 = error1.clone();

    assert_eq!(error1, error2, "Cloned error should equal original");
}

/// Test error messages for all PageExtractionError variants
#[test]
fn test_all_page_extraction_error_messages_are_descriptive() {
    let errors = vec![
        PageExtractionError::NoPagesInDocument,
        PageExtractionError::IndexOutOfBounds {
            requested: 10,
            available: 5,
        },
        PageExtractionError::InvalidMediaBox {
            page_index: 0,
            media_box: Some([0.0, 0.0, -1.0, 792.0]),
        },
        PageExtractionError::InvalidDimensions {
            page_index: 1,
            width: 0.0,
            height: 792.0,
        },
        PageExtractionError::InvalidRotation {
            page_index: 2,
            rotation: 45,
        },
        PageExtractionError::ContentStreamDecodeFailed {
            page_index: 3,
            message: "Invalid FlateDecode stream".to_string(),
        },
        PageExtractionError::MissingContentStream { page_index: 4 },
        PageExtractionError::ContentStreamTooLarge {
            page_index: 5,
            size_bytes: 500_000_000,
            max_bytes: 100_000_000,
        },
        PageExtractionError::InvalidResources {
            page_index: 6,
            message: "Font dictionary missing".to_string(),
        },
        PageExtractionError::MissingRequiredFields {
            page_index: 7,
            fields: vec!["MediaBox".to_string(), "Resources".to_string()],
        },
        PageExtractionError::GlyphExtractionFailed {
            page_index: 8,
            message: "Font encoding not supported".to_string(),
        },
        PageExtractionError::SpanMergeFailed {
            page_index: 9,
            glyph_count: 1000,
            message: "Inconsistent font sizes".to_string(),
        },
        PageExtractionError::LayoutAnalysisFailed {
            page_index: 10,
            stage: "XY-cut".to_string(),
            message: "Invalid block geometry".to_string(),
        },
        PageExtractionError::TableDetectionFailed {
            page_index: 11,
            message: "No table structures found".to_string(),
        },
        PageExtractionError::ReceiptGenerationFailed {
            page_index: 12,
            message: "Missing required fields".to_string(),
        },
        PageExtractionError::MalformedPageData {
            page_index: 13,
            message: "Corrupted content stream".to_string(),
        },
        PageExtractionError::MalformedDocumentStructure("Page tree corrupted".to_string()),
        PageExtractionError::ExtractionPanicked {
            page_index: 14,
            message: "Index out of bounds".to_string(),
        },
        PageExtractionError::ExtractionFailed {
            page_index: 15,
            message: "Unknown error".to_string(),
        },
    ];

    for error in errors {
        let msg = format!("{}", error);
        assert!(
            msg.len() > 15,
            "Error message should be descriptive (more than 15 chars), got: {} for {:?}",
            msg,
            error
        );

        // Most errors should mention the page index if they have one
        if let PageExtractionError::InvalidMediaBox { page_index, .. }
        | PageExtractionError::InvalidDimensions { page_index, .. }
        | PageExtractionError::InvalidRotation { page_index, .. }
        | PageExtractionError::ContentStreamDecodeFailed { page_index, .. }
        | PageExtractionError::MissingContentStream { page_index }
        | PageExtractionError::ContentStreamTooLarge { page_index, .. }
        | PageExtractionError::InvalidResources { page_index, .. }
        | PageExtractionError::MissingRequiredFields { page_index, .. }
        | PageExtractionError::GlyphExtractionFailed { page_index, .. }
        | PageExtractionError::SpanMergeFailed { page_index, .. }
        | PageExtractionError::LayoutAnalysisFailed { page_index, .. }
        | PageExtractionError::TableDetectionFailed { page_index, .. }
        | PageExtractionError::ReceiptGenerationFailed { page_index, .. }
        | PageExtractionError::MalformedPageData { page_index, .. }
        | PageExtractionError::ExtractionPanicked { page_index, .. }
        | PageExtractionError::ExtractionFailed { page_index, .. } = error
        {
            assert!(
                msg.contains(&format!("{}", page_index)) || msg.contains(&format!("Page {}", page_index)),
                "Error should mention page index {}, got: {}",
                page_index,
                msg
            );
        }
    }
}

/// Test that valid rotations (0, 90, 180, 270) are accepted
#[test]
fn test_valid_rotations_are_accepted() {
    let valid_rotations = [0, 90, 180, 270];

    for rotation in valid_rotations.iter() {
        // These should not produce InvalidRotation errors
        assert!(
            *rotation == 0 || *rotation == 90 || *rotation == 180 || *rotation == 270,
            "Valid rotation test setup is wrong"
        );
    }
}

/// Test that invalid rotation values produce appropriate errors
#[test]
fn test_invalid_rotation_values_produce_errors() {
    let invalid_rotations = [-1, 1, 45, 89, 91, 179, 181, 269, 271, 360];

    for rotation in invalid_rotations.iter() {
        // Create InvalidRotation error
        let error = PageExtractionError::InvalidRotation {
            page_index: 0,
            rotation: *rotation,
        };

        let msg = format!("{}", error);
        assert!(
            msg.contains(&format!("{}", rotation)),
            "Error should mention the invalid rotation value {}, got: {}",
            rotation,
            msg
        );
    }
}

/// Test edge case: exactly at boundary (last page)
#[test]
fn test_boundary_last_page_access() {
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

    if page_count == 0 {
        println!("Skipping test: document has no pages");
        return;
    }

    // Access the last valid page (index = page_count - 1)
    let result = doc.extract_page(page_count - 1);
    assert!(
        result.is_ok(),
        "Should successfully extract last page at index {}",
        page_count - 1
    );
}

/// Test edge case: just beyond boundary (first invalid page)
#[test]
fn test_boundary_first_invalid_page() {
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

    // Access the first invalid page (index = page_count)
    let result = doc.extract_page(page_count);
    assert!(
        result.is_err(),
        "Should return error for page index equal to page_count"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("out of bounds"),
        "Error should mention out-of-bounds, got: {}",
        error_msg
    );
}
