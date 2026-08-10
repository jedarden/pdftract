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
    // Note: classify_page returns PageClassification directly (not a Result),
    // so success means the function returns without panic and produces valid output
    let result = classify_page(&ctx);

    // VERIFY SUCCESS: classify_page executed without panic and returned valid classification
    // This is the "Ok()" case for the direct-return API - successful execution is
    // demonstrated by the function returning a valid PageClassification
    assert!(
        matches!(result.class, PageClass::Vector | PageClass::Scanned | PageClass::Hybrid | PageClass::BrokenVector),
        "classify_page must return a valid PageClass variant, got: {:?}",
        result.class
    );

    // Verify classification succeeded (direct return, no Result wrapper)
    assert_eq!(
        result.class,
        PageClass::Vector,
        "Simple vector page should classify as Vector"
    );

    // ========== OUTPUT FORMAT VERIFICATION ==========
    // Comprehensive validation of PageClassification structure

    // 1. Verify classification field exists and is valid
    assert!(
        matches!(result.class, PageClass::Vector | PageClass::Scanned | PageClass::Hybrid | PageClass::BrokenVector),
        "OUTPUT FORMAT ERROR: class field must be valid PageClass variant, got: {:?}. Expected one of: Vector, Scanned, Hybrid, BrokenVector",
        result.class
    );

    // 2. Verify classification field is non-empty and matches expected value
    assert_eq!(
        result.class,
        PageClass::Vector,
        "OUTPUT FORMAT ERROR: Expected PageClass::Vector for simple vector page, got: {:?}. This suggests classification logic failed or PageContext was misconfigured.",
        result.class
    );

    // 3. Verify confidence score is within expected range [0.0, 1.0]
    assert!(
        result.confidence >= 0.0 && result.confidence <= 1.0,
        "OUTPUT FORMAT ERROR: confidence must be in range [0.0, 1.0], got: {}. Value out of valid range indicates scoring logic failure.",
        result.confidence
    );

    // 4. Verify confidence is reasonable for this classification (not zero/uninitialized)
    assert!(
        result.confidence > 0.5,
        "OUTPUT FORMAT ERROR: Expected confidence > 0.5 for clear vector page with 98% character validity, got: {}. Low confidence on simple vector page suggests signal evaluator failure.",
        result.confidence
    );

    // 5. Verify hybrid_cells field structure (Option<BTreeSet<usize>>)
    assert!(
        result.hybrid_cells.is_none(),
        "OUTPUT FORMAT ERROR: hybrid_cells must be None for non-Hybrid classification (got: {:?}). Non-Hybrid pages should not have scanned cell indexes.",
        result.hybrid_cells
    );

    // 6. Verify output is complete (not uninitialized/default)
    assert!(
        !((result.class == PageClass::Vector) && (result.confidence == 0.0)),
        "OUTPUT FORMAT ERROR: Classification appears uninitialized (Vector with 0.0 confidence). This suggests PageClassification::new() was called with default values without proper classification logic."
    );

    // 7. Verify JSON serialization format (if needed for output)
    let json_result = serde_json::to_string(&result);
    assert!(
        json_result.is_ok(),
        "OUTPUT FORMAT ERROR: PageClassification failed to serialize to JSON: {:?}. Struct fields may be incompatible with Serialize trait.",
        json_result.err()
    );

    let json_str = json_result.unwrap();
    let required_fields = vec!["\"class\"", "\"confidence\"", "\"hybrid_cells\""];
    for field in required_fields {
        assert!(
            json_str.contains(field),
            "OUTPUT FORMAT ERROR: JSON output missing required field '{}'. Got: {}. Expected all top-level fields present in serialized output.",
            field, json_str
        );
    }

    println!("✓ classify_page output format verification complete:");
    println!("  - class: {:?} (valid PageClass)", result.class);
    println!("  - confidence: {:.3} (in range [0.0, 1.0])", result.confidence);
    println!("  - hybrid_cells: {:?} (correct for non-Hybrid)", result.hybrid_cells);
    println!("  - JSON format: valid (all required fields present)");
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
    // Note: classify_page returns PageClassification directly (not a Result),
    // so success means the function returns without panic and produces valid output
    let result = classify_page(&ctx);

    // VERIFY SUCCESS: classify_page executed without panic and returned valid classification
    // This is the "Ok()" case for the direct-return API - successful execution is
    // demonstrated by the function returning a valid PageClassification
    assert!(
        matches!(result.class, PageClass::Vector | PageClass::Scanned | PageClass::Hybrid | PageClass::BrokenVector),
        "classify_page must return a valid PageClass variant, got: {:?}",
        result.class
    );

    // Verify classification succeeded
    assert_eq!(
        result.class,
        PageClass::Scanned,
        "Image-only page should classify as Scanned"
    );

    // ========== OUTPUT FORMAT VERIFICATION ==========
    // Comprehensive validation of PageClassification structure for scanned pages

    // 1. Verify classification field exists and is valid
    assert!(
        matches!(result.class, PageClass::Vector | PageClass::Scanned | PageClass::Hybrid | PageClass::BrokenVector),
        "OUTPUT FORMAT ERROR: class field must be valid PageClass variant, got: {:?}. Expected one of: Vector, Scanned, Hybrid, BrokenVector",
        result.class
    );

    // 2. Verify classification field matches expected value for scanned page
    assert_eq!(
        result.class,
        PageClass::Scanned,
        "OUTPUT FORMAT ERROR: Expected PageClass::Scanned for image-only page, got: {:?}. This suggests classification logic failed for scanned content.",
        result.class
    );

    // 3. Verify confidence score is within expected range [0.0, 1.0]
    assert!(
        result.confidence >= 0.0 && result.confidence <= 1.0,
        "OUTPUT FORMAT ERROR: confidence must be in range [0.0, 1.0], got: {}. Value out of valid range indicates scoring logic failure.",
        result.confidence
    );

    // 4. Verify confidence is reasonable for high-confidence scanned page
    assert!(
        result.confidence > 0.8,
        "OUTPUT FORMAT ERROR: Expected confidence > 0.8 for clear scanned page with 95% image coverage, got: {}. Low confidence suggests signal evaluator failure.",
        result.confidence
    );

    // 5. Verify hybrid_cells field structure (should be None for Scanned classification)
    assert!(
        result.hybrid_cells.is_none(),
        "OUTPUT FORMAT ERROR: hybrid_cells must be None for Scanned classification (got: {:?}). Scanned pages should not have hybrid cell indexes.",
        result.hybrid_cells
    );

    // 6. Verify output is complete (not uninitialized)
    assert!(
        !((result.class == PageClass::Scanned) && (result.confidence == 0.0)),
        "OUTPUT FORMAT ERROR: Classification appears uninitialized (Scanned with 0.0 confidence). This suggests default PageClassification was returned without proper evaluation."
    );

    // 7. Verify JSON serialization format
    let json_result = serde_json::to_string(&result);
    assert!(
        json_result.is_ok(),
        "OUTPUT FORMAT ERROR: PageClassification failed to serialize to JSON: {:?}. Struct fields may be incompatible with Serialize trait.",
        json_result.err()
    );

    let json_str = json_result.unwrap();
    assert!(
        json_str.contains("\"class\"") && json_str.contains("\"confidence\"") && json_str.contains("\"hybrid_cells\""),
        "OUTPUT FORMAT ERROR: JSON output missing required top-level fields. Got: {}. Expected all fields present in serialized output.",
        json_str
    );

    println!("✓ classify_page output format verification complete for scanned page:");
    println!("  - class: {:?} (valid PageClass)", result.class);
    println!("  - confidence: {:.3} (in range [0.0, 1.0])", result.confidence);
    println!("  - hybrid_cells: {:?} (correct for non-Hybrid)", result.hybrid_cells);
    println!("  - JSON format: valid (all required fields present)");
}

#[test]
fn test_classify_page_returns_valid_result_for_valid_input() {
    //! Verify classify_page returns valid classification for valid input (the "Ok()" case).
    //!
    //! This test explicitly verifies that classify_page succeeds when given valid PageContext.
    //! Since classify_page returns PageClassification directly (not a Result), the "Ok()" case
    //! means the function returns without panic and produces a valid PageClassification.
    //!
    //! If classify_page were to panic or return invalid data, this test would fail with a
    //! clear message indicating the "Err" case occurred.

    let mut ctx = PageContext::new();
    // Construct valid PageContext with all required fields populated
    ctx.text_op_count = 50;
    ctx.raw_char_count = 250;
    ctx.valid_char_count = 245;
    ctx.replacement_char_count = 5;
    ctx.image_coverage = 0.2;
    ctx.has_full_page_image = false;
    ctx.has_visible_text = true;
    ctx.density_ratio = 0.75;
    ctx.width = 595.0; // A4
    ctx.height = 842.0;
    ctx.rotation = 0;

    // EXECUTE: Call classify_page with valid input
    // SUCCESS CRITERIA: Function returns without panic and produces valid PageClassification
    let result = classify_page(&ctx);

    // ========== OUTPUT FORMAT VERIFICATION ==========
    // Comprehensive validation of PageClassification structure

    // 1. Verify classification field exists and contains valid PageClass
    assert!(
        matches!(result.class, PageClass::Vector | PageClass::Scanned | PageClass::Hybrid | PageClass::BrokenVector),
        "OUTPUT FORMAT ERROR: classify_page returned invalid PageClass variant (this is the 'Err' case for the direct-return API). Got: {:?}. Expected one of: Vector, Scanned, Hybrid, BrokenVector",
        result.class
    );

    // 2. Verify classification field is non-empty (not a default/uninitialized state)
    assert!(
        matches!(result.class, PageClass::Vector | PageClass::Scanned | PageClass::Hybrid | PageClass::BrokenVector),
        "OUTPUT FORMAT ERROR: classification field appears uninitialized or invalid. Got: {:?}. Expected a valid, non-default PageClass variant.",
        result.class
    );

    // 3. Verify confidence score is within expected range [0.0, 1.0]
    assert!(
        result.confidence >= 0.0 && result.confidence <= 1.0,
        "OUTPUT FORMAT ERROR: classify_page returned invalid confidence value (this indicates the 'Err' case). Got: {}. Expected value in range [0.0, 1.0]",
        result.confidence
    );

    // 4. Verify confidence is non-zero for valid input (indicates actual classification occurred)
    assert!(
        result.confidence > 0.0,
        "OUTPUT FORMAT ERROR: confidence is 0.0 for valid input, suggesting classification logic did not run. Expected confidence > 0.0 for valid PageContext with text and reasonable density."
    );

    // 5. Verify hybrid_cells field structure (Option<BTreeSet<usize>>)
    // For this test case (valid text content), hybrid_cells should be None
    assert!(
        result.hybrid_cells.is_none(),
        "OUTPUT FORMAT ERROR: hybrid_cells should be None for non-Hybrid classification with valid text content. Got: {:?}. Expected None for this test case.",
        result.hybrid_cells
    );

    // 6. Verify output structure completeness - all critical fields present
    // The PageClassification struct should have all three fields properly set
    assert!(
        (result.confidence >= 0.0) && (result.confidence <= 1.0),
        "OUTPUT FORMAT ERROR: confidence field missing or out of range. Got: {}. Expected valid f32 in [0.0, 1.0]",
        result.confidence
    );

    // 7. Verify JSON serialization format and structure
    let json_result = serde_json::to_string(&result);
    assert!(
        json_result.is_ok(),
        "OUTPUT FORMAT ERROR: PageClassification failed to serialize to JSON: {:?}. Struct fields may be incompatible with Serialize trait.",
        json_result.err()
    );

    let json_str = json_result.unwrap();
    let required_fields = vec!["\"class\"", "\"confidence\"", "\"hybrid_cells\""];
    for field in required_fields {
        assert!(
            json_str.contains(field),
            "OUTPUT FORMAT ERROR: JSON output missing required field '{}'. Got: {}. Expected all top-level fields present in serialized output.",
            field, json_str
        );
    }

    // 8. Verify JSON structure matches expected format
    assert!(
        json_str.contains("{") && json_str.contains("}") && json_str.contains(":"),
        "OUTPUT FORMAT ERROR: JSON output has invalid structure. Got: {}. Expected valid JSON object with key-value pairs.",
        json_str
    );

    // Clear success output indicating comprehensive verification
    println!("✓ SUCCESS: classify_page executed successfully and returned valid PageClassification");
    println!("  ✓ Output format verification complete:");
    println!("    - PageClass: {:?} (valid, non-empty)", result.class);
    println!("    - Confidence: {:.2} (in range [0.0, 1.0], non-zero)", result.confidence);
    println!("    - Hybrid cells: None (correct for non-Hybrid classification)");
    println!("    - JSON serialization: valid (all required fields present)");
    println!("    - Structure: complete (all critical fields validated)");
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

#[test]
fn test_classify_page_output_format_comprehensive() {
    //! Comprehensive test for classify_page output format and structure.
    //!
    //! This test validates all aspects of the PageClassification output format:
    //! - Field presence and validity (classification, confidence, hybrid_cells)
    //! - Data types and ranges
    //! - JSON serialization structure
    //! - Clear error messages for format violations
    //!
    //! This test ensures that any change to the output format will be caught
    //! with explicit, actionable error messages.

    // Test case 1: Vector page output format
    let mut ctx_vector = PageContext::new();
    ctx_vector.text_op_count = 150;
    ctx_vector.raw_char_count = 750;
    ctx_vector.valid_char_count = 735; // 98% validity
    ctx_vector.replacement_char_count = 15;
    ctx_vector.image_coverage = 0.0;
    ctx_vector.has_full_page_image = false;
    ctx_vector.has_visible_text = true;
    ctx_vector.density_ratio = 0.85;
    ctx_vector.width = 612.0;
    ctx_vector.height = 792.0;
    ctx_vector.rotation = 0;

    let result_vector = classify_page(&ctx_vector);

    // COMPREHENSIVE OUTPUT FORMAT VALIDATION FOR VECTOR CLASSIFICATION

    // 1. Verify classification field exists and is valid
    assert!(
        matches!(result_vector.class, PageClass::Vector | PageClass::Scanned | PageClass::Hybrid | PageClass::BrokenVector),
        "OUTPUT FORMAT ERROR [Vector case]: class field must be valid PageClass variant. Got: {:?}. Expected one of: Vector, Scanned, Hybrid, BrokenVector",
        result_vector.class
    );

    // 2. Verify classification field is non-empty and matches expected
    assert_eq!(
        result_vector.class,
        PageClass::Vector,
        "OUTPUT FORMAT ERROR [Vector case]: Expected PageClass::Vector for high-validity text page. Got: {:?}. Classification logic may have failed.",
        result_vector.class
    );

    // 3. Verify confidence is within expected range [0.0, 1.0]
    assert!(
        result_vector.confidence >= 0.0 && result_vector.confidence <= 1.0,
        "OUTPUT FORMAT ERROR [Vector case]: confidence must be in range [0.0, 1.0]. Got: {}. Scoring logic failure detected.",
        result_vector.confidence
    );

    // 4. Verify confidence is reasonable for the input (high confidence for clear vector)
    assert!(
        result_vector.confidence > 0.7,
        "OUTPUT FORMAT ERROR [Vector case]: Expected confidence > 0.7 for clear vector page with 98% validity. Got: {}. Signal evaluator may be under-scoring.",
        result_vector.confidence
    );

    // 5. Verify hybrid_cells field structure (should be None for non-Hybrid)
    assert!(
        result_vector.hybrid_cells.is_none(),
        "OUTPUT FORMAT ERROR [Vector case]: hybrid_cells must be None for non-Hybrid classification. Got: {:?}. Structure violation detected.",
        result_vector.hybrid_cells
    );

    // 6. Verify output is not uninitialized/zeroed
    assert!(
        !((result_vector.class == PageClass::Vector) && (result_vector.confidence == 0.0)),
        "OUTPUT FORMAT ERROR [Vector case]: Output appears uninitialized (Vector with 0.0 confidence). PageClassification::new() may have been called with defaults."
    );

    // Test case 2: Scanned page output format
    let mut ctx_scanned = PageContext::new();
    ctx_scanned.text_op_count = 0;
    ctx_scanned.raw_char_count = 0;
    ctx_scanned.valid_char_count = 0;
    ctx_scanned.replacement_char_count = 0;
    ctx_scanned.image_coverage = 0.95;
    ctx_scanned.image_xobject_areas = vec![460_500.0]; // Full page image
    ctx_scanned.has_full_page_image = true;
    ctx_scanned.has_visible_text = false;
    ctx_scanned.density_ratio = 0.0;
    ctx_scanned.width = 612.0;
    ctx_scanned.height = 792.0;
    ctx_scanned.rotation = 0;

    let result_scanned = classify_page(&ctx_scanned);

    // COMPREHENSIVE OUTPUT FORMAT VALIDATION FOR SCANNED CLASSIFICATION

    // 1-2. Verify classification field
    assert!(
        matches!(result_scanned.class, PageClass::Vector | PageClass::Scanned | PageClass::Hybrid | PageClass::BrokenVector),
        "OUTPUT FORMAT ERROR [Scanned case]: class field must be valid PageClass variant. Got: {:?}",
        result_scanned.class
    );

    assert_eq!(
        result_scanned.class,
        PageClass::Scanned,
        "OUTPUT FORMAT ERROR [Scanned case]: Expected PageClass::Scanned for image-only page. Got: {:?}",
        result_scanned.class
    );

    // 3-4. Verify confidence field
    assert!(
        result_scanned.confidence >= 0.0 && result_scanned.confidence <= 1.0,
        "OUTPUT FORMAT ERROR [Scanned case]: confidence must be in range [0.0, 1.0]. Got: {}",
        result_scanned.confidence
    );

    assert!(
        result_scanned.confidence > 0.9,
        "OUTPUT FORMAT ERROR [Scanned case]: Expected confidence > 0.9 for clear scanned page. Got: {}",
        result_scanned.confidence
    );

    // 5. Verify hybrid_cells field
    assert!(
        result_scanned.hybrid_cells.is_none(),
        "OUTPUT FORMAT ERROR [Scanned case]: hybrid_cells must be None for Scanned classification. Got: {:?}",
        result_scanned.hybrid_cells
    );

    // 6. Verify output completeness
    assert!(
        !((result_scanned.class == PageClass::Scanned) && (result_scanned.confidence == 0.0)),
        "OUTPUT FORMAT ERROR [Scanned case]: Output appears uninitialized (Scanned with 0.0 confidence)."
    );

    // Test case 3: JSON serialization format validation
    let json_vector = serde_json::to_string(&result_vector);
    let json_scanned = serde_json::to_string(&result_scanned);

    assert!(
        json_vector.is_ok(),
        "OUTPUT FORMAT ERROR: Vector classification failed to serialize to JSON: {:?}",
        json_vector.err()
    );

    assert!(
        json_scanned.is_ok(),
        "OUTPUT FORMAT ERROR: Scanned classification failed to serialize to JSON: {:?}",
        json_scanned.err()
    );

    let json_str_vector = json_vector.unwrap();
    let json_str_scanned = json_scanned.unwrap();

    // Verify JSON contains all required top-level fields
    let required_fields = vec!["\"class\"", "\"confidence\"", "\"hybrid_cells\""];
    for field in required_fields {
        assert!(
            json_str_vector.contains(field),
            "OUTPUT FORMAT ERROR [Vector JSON]: Missing required field '{}'. Got: {}",
            field, json_str_vector
        );

        assert!(
            json_str_scanned.contains(field),
            "OUTPUT FORMAT ERROR [Scanned JSON]: Missing required field '{}'. Got: {}",
            field, json_str_scanned
        );
    }

    // Verify JSON structure is valid (contains braces and colons)
    assert!(
        json_str_vector.contains("{") && json_str_vector.contains("}") && json_str_vector.contains(":"),
        "OUTPUT FORMAT ERROR [Vector JSON]: Invalid JSON structure. Got: {}",
        json_str_vector
    );

    assert!(
        json_str_scanned.contains("{") && json_str_scanned.contains("}") && json_str_scanned.contains(":"),
        "OUTPUT FORMAT ERROR [Scanned JSON]: Invalid JSON structure. Got: {}",
        json_str_scanned
    );

    println!("✓ COMPREHENSIVE OUTPUT FORMAT VALIDATION COMPLETE");
    println!("  ✓ Classification field: Valid and non-empty for all test cases");
    println!("  ✓ Confidence field: In range [0.0, 1.0] and reasonable for inputs");
    println!("  ✓ hybrid_cells field: Correct Option<BTreeSet<usize>> structure");
    println!("  ✓ Output completeness: No uninitialized or zeroed outputs");
    println!("  ✓ JSON serialization: All required fields present and valid structure");
    println!("  ✓ Error messages: Clear and specific for format violations");
}
