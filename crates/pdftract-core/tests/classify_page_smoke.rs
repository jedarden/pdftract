//! Basic smoke test for classify_page integration with PDF fixtures.
//!
//! This test verifies that page classification works correctly through
//! the full extraction pipeline using actual PDF files.
//!
//! Acceptance criteria:
//! - Test uses a simple PDF fixture (vector_pure/source.pdf)
//! - SDK extraction returns Ok() for a valid PDF
//! - Output format is verified (JSON structure, expected fields)
//! - Basic output fields are checked (page_type exists and has valid value)
//! - All tests pass and module compiles without errors

use pdftract_core::options::ExtractionOptions;
use pdftract_core::sdk;
use std::path::PathBuf;

fn get_fixture_path(fixture_name: &str) -> PathBuf {
    // Get the workspace root by going up from the crate's manifest dir
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("../../tests/fixtures/page_class")
        .join(fixture_name)
}

#[test]
fn test_classify_page_smoke_with_pdf_fixture() {
    //! Basic smoke test for page classification using a simple PDF fixture.
    //!
    //! This test verifies:
    //! - classify_page works through the extraction pipeline with a real PDF
    //! - Output format includes expected fields (page_type)
    //! - Function returns Ok() for a valid PDF
    //! - Basic output structure is correct (classification exists, reasonable value)
    //!
    //! Uses vector_pure/source.pdf as a simple text-only PDF fixture.

    let fixture_path = get_fixture_path("vector_pure/source.pdf");

    // Verify fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    // Run SDK extraction (this calls classify_page internally)
    let result = sdk::extract(&fixture_path, &ExtractionOptions::default());

    // Verify extraction succeeded (Ok() for valid PDF)
    assert!(
        result.is_ok(),
        "SDK extract() should succeed for vector_pure PDF: {:?}",
        result.err()
    );

    let extraction_result = result.unwrap();

    // Verify at least one page was extracted
    assert!(
        !extraction_result.pages.is_empty(),
        "At least one page should be extracted from the PDF"
    );

    // Get first page
    let first_page = &extraction_result.pages[0];

    // Verify basic output structure: page_type field exists
    let page_type = first_page.page_type.as_ref()
        .expect("page_type field should exist (Some), got None. Classification may not have run.");

    assert!(
        !page_type.is_empty(),
        "page_type field should not be empty. Got empty string, which suggests \
         classification was not run or failed. Expected a valid page type like 'text', 'scanned', \
         'mixed', or 'broken_vector'."
    );

    // Verify classification exists and has a reasonable value
    // Valid page types: "text", "scanned", "mixed", "broken_vector", "blank", "figure_only"
    let valid_page_types = [
        "text",          // Vector/born-digital text
        "scanned",       // Scanned image-only
        "mixed",         // Hybrid (text + scanned)
        "broken_vector", // Invisible text over scanned image
        "blank",         // Empty page
        "figure_only",   // Image-only page
    ];

    assert!(
        valid_page_types.contains(&page_type.as_str()),
        "page_type should be one of the expected values. Got: '{}'. Expected one of: {:?}. \
         This indicates classification returned an invalid or unexpected value.",
        page_type,
        valid_page_types
    );

    // For vector_pure fixture, we expect "text" (born-digital PDF)
    assert_eq!(
        page_type, "text",
        "vector_pure/source.pdf should classify as 'text' (born-digital PDF). Got: '{}'. \
         This suggests the classification logic misidentified a pure text PDF.",
        page_type
    );

    // Verify the result can be serialized to JSON (output format verification)
    let json_result = serde_json::to_string(&extraction_result);
    assert!(
        json_result.is_ok(),
        "ExtractionResult must be JSON-serializable, got error: {:?}",
        json_result.err()
    );

    // Verify JSON contains expected page_type field
    let json_string = json_result.unwrap();
    assert!(
        json_string.contains("\"page_type\""),
        "JSON output must contain 'page_type' field. Got: {}",
        json_string
    );

    // Verify JSON contains the actual page_type value
    assert!(
        json_string.contains("\"text\""),
        "JSON output should contain the page_type value 'text'. Got: {}",
        json_string
    );

    println!(
        "test_classify_page_smoke_with_pdf_fixture PASSED: \
         page_type={}, pages={}",
        first_page.page_type.as_ref().map(|s| s.as_str()).unwrap_or("None"),
        extraction_result.pages.len()
    );
}

#[test]
fn test_classify_page_smoke_scanned_fixture() {
    //! Smoke test for page classification using a scanned PDF fixture.
    //!
    //! Verifies that scanned PDF (image-only) is correctly classified.

    let fixture_path = get_fixture_path("scanned_single/source.pdf");

    // Verify fixture exists
    assert!(
        fixture_path.exists(),
        "Test fixture should exist at {}",
        fixture_path.display()
    );

    // Run SDK extraction
    let result = sdk::extract(&fixture_path, &ExtractionOptions::default());

    // Verify extraction succeeded
    assert!(
        result.is_ok(),
        "SDK extract() should succeed for scanned_single PDF: {:?}",
        result.err()
    );

    let extraction_result = result.unwrap();

    // Verify at least one page was extracted
    assert!(
        !extraction_result.pages.is_empty(),
        "At least one page should be extracted from the PDF"
    );

    // Get first page
    let first_page = &extraction_result.pages[0];

    // Verify page_type exists and is "scanned"
    let page_type = first_page.page_type.as_ref()
        .expect("page_type field should exist (Some), got None");

    assert!(!page_type.is_empty());

    assert_eq!(
        page_type, "scanned",
        "scanned_single/source.pdf should classify as 'scanned' (image-only PDF). Got: '{}'",
        page_type
    );

    println!(
        "test_classify_page_smoke_scanned_fixture PASSED: \
         page_type={}",
        page_type
    );
}

#[test]
fn test_classify_page_output_format_verification() {
    //! Verify output format includes all expected fields in JSON structure.
    //!
    //! This test ensures the JSON output format matches the schema expectations.

    let fixture_path = get_fixture_path("vector_pure/source.pdf");

    let result = sdk::extract(&fixture_path, &ExtractionOptions::default());
    assert!(result.is_ok(), "Extraction should succeed");

    let extraction_result = result.unwrap();

    // Serialize to JSON
    let json_value = serde_json::to_value(&extraction_result);
    assert!(json_value.is_ok(), "Should serialize to JSON");

    let json = json_value.unwrap();

    // Verify top-level structure
    assert!(
        json.get("pages").is_some(),
        "JSON should contain 'pages' array"
    );

    let pages = json["pages"].as_array().expect("pages should be an array");
    assert!(
        !pages.is_empty(),
        "pages array should contain at least one page"
    );

    // Verify page structure
    let first_page = &pages[0];

    // Check for expected fields
    let expected_fields = [
        "page_type",   // Classification result
        "width",        // Page width
        "height",       // Page height
        "blocks",       // Content blocks
        "spans",        // Text spans
    ];

    for field in &expected_fields {
        assert!(
            first_page.get(*field).is_some(),
            "Page JSON should contain '{}' field. Full page: {}",
            field,
            first_page
        );
    }

    // Verify page_type is a string
    let page_type = first_page["page_type"].as_str();
    assert!(
        page_type.is_some(),
        "page_type should be a string, got: {}",
        first_page["page_type"]
    );

    println!(
        "test_classify_page_output_format_verification PASSED: \
         all expected fields present in JSON output"
    );
}
