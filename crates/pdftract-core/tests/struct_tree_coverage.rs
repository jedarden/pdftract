//! Integration tests for Phase 7.1.4: StructTree coverage check and XY-cut fallback.
//!
//! These tests verify the full extraction pipeline with /MarkInfo /Suspects flag
//! and the coverage-based fallback to XY-cut reading order.
//!
//! Acceptance criteria from pdftract-2w3r:
//! - PDF with Suspects true falls back to XY-cut, reading_order_algorithm = "xy_cut"
//! - Unit tests: Suspects false + 50% coverage -> no fallback
//! - Unit tests: Suspects true + 95% coverage -> no fallback
//! - Unit tests: Suspects true + 60% coverage -> fallback
//! - Per-page diagnostic appears in receipts when fallback triggers
//! - Integration: full pipeline test on tagged-suspects-true.pdf fixture produces expected reading order

use pdftract_core::extract::extract_pdf;
use pdftract_core::options::ExtractionOptions;
use std::path::PathBuf;

/// Get the path to a fixture file, handling both workspace and crate test locations
fn get_fixture_path(fixture_name: &str) -> PathBuf {
    // Try workspace root first (when running from workspace)
    let workspace_path = PathBuf::from(format!("tests/fixtures/{}", fixture_name));
    if workspace_path.exists() {
        return workspace_path;
    }

    // Try from crate directory (when running from crate tests)
    let crate_path = PathBuf::from(format!("../../tests/fixtures/{}", fixture_name));
    if crate_path.exists() {
        return crate_path;
    }

    // Try using CARGO_MANIFEST_DIR
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let from_manifest = PathBuf::from(manifest_dir)
            .join("../../tests/fixtures")
            .join(fixture_name);
        if from_manifest.exists() {
            return from_manifest;
        }
    }

    // Fallback: panic with helpful message
    panic!(
        "Fixture {} not found. Tried:\n  1. {}\n  2. {}\n  3. $CARGO_MANIFEST_DIR/../../tests/fixtures/{}",
        fixture_name,
        workspace_path.display(),
        crate_path.display(),
        fixture_name
    );
}

#[test]
fn test_suspects_true_fallback_to_xy_cut() {
    // Integration test: full pipeline with Suspects true triggers fallback
    // This test verifies the acceptance criteria:
    // "PDF with Suspects true falls back to XY-cut, reading_order_algorithm = 'xy_cut'"

    // For this test, we'll use a mock PDF or fixture if available
    // The fixture should have:
    // - /MarkInfo /Suspects true
    // - StructTree with coverage < 80% (e.g., 60%)

    // Note: This test requires a tagged-suspects-true.pdf fixture
    // If the fixture doesn't exist, the test will be skipped

    let fixture_path = get_fixture_path("tagged-suspects-true.pdf");

    if !fixture_path.exists() {
        println!("WARNING: Fixture tagged-suspects-true.pdf not found, skipping integration test");
        println!("To create this fixture, run: cargo run --manifest-path=tests/fixtures/Cargo.toml --bin generate_suspects_fixture");
        return;
    }

    let options = ExtractionOptions {
        receipts: pdftract_core::options::ReceiptsMode::Off,
        max_parallel_pages: 1,
        memory_budget_mb: 512,
        full_render: false,
        ocr_dpi_override: None,
        ocr_language: vec!["eng".to_string()],
        markdown_anchors: false,
        max_decompress_bytes: 512 * 1024 * 1024,
        output: Default::default(),
        pages: None,
        password: None,
        http_headers: None,
    };

    let result = extract_pdf(&fixture_path, &options);

    match result {
        Ok(extraction_result) => {
            // Verify reading_order_algorithm is "xy_cut" due to Suspects + low coverage
            let algo = extraction_result
                .metadata
                .reading_order_algorithm
                .expect("reading_order_algorithm should be set");

            assert_eq!(
                algo,
                "xy_cut",
                "Expected reading_order_algorithm='xy_cut' for Suspects true with low coverage, got '{}'",
                algo
            );

            println!(
                "Integration test passed: reading_order_algorithm = '{}'",
                algo
            );
        }
        Err(e) => {
            panic!("Extraction failed: {}", e);
        }
    }
}

#[test]
fn test_suspects_false_trusts_tree() {
    // Integration test: Suspects false means we trust the StructTree
    // even if coverage is low

    // This test would require a fixture with:
    // - /MarkInfo /Suspects false
    // - StructTree with coverage < 80%
    // Expected: reading_order_algorithm = "struct_tree"

    let fixture_path = get_fixture_path("tagged-suspects-false.pdf");

    if !fixture_path.exists() {
        println!("WARNING: Fixture tagged-suspects-false.pdf not found, skipping integration test");
        return;
    }

    let options = ExtractionOptions {
        receipts: pdftract_core::options::ReceiptsMode::Off,
        max_parallel_pages: 1,
        memory_budget_mb: 512,
        full_render: false,
        ocr_dpi_override: None,
        ocr_language: vec!["eng".to_string()],
        markdown_anchors: false,
        max_decompress_bytes: 512 * 1024 * 1024,
        output: Default::default(),
        pages: None,
        password: None,
        http_headers: None,
    };

    let result = extract_pdf(&fixture_path, &options);

    match result {
        Ok(extraction_result) => {
            // Verify reading_order_algorithm is "struct_tree" even with low coverage
            let algo = extraction_result
                .metadata
                .reading_order_algorithm
                .expect("reading_order_algorithm should be set");

            assert_eq!(
                algo, "struct_tree",
                "Expected reading_order_algorithm='struct_tree' for Suspects false, got '{}'",
                algo
            );

            println!(
                "Integration test passed: reading_order_algorithm = '{}'",
                algo
            );
        }
        Err(e) => {
            panic!("Extraction failed: {}", e);
        }
    }
}

#[test]
fn test_suspects_true_high_coverage_no_fallback() {
    // Integration test: Suspects true + high coverage (>= 80%) = no fallback

    // This test would require a fixture with:
    // - /MarkInfo /Suspects true
    // - StructTree with coverage >= 80%
    // Expected: reading_order_algorithm = "struct_tree"

    let fixture_path = get_fixture_path("tagged-suspects-true-high-coverage.pdf");

    if !fixture_path.exists() {
        println!("WARNING: Fixture tagged-suspects-true-high-coverage.pdf not found, skipping integration test");
        return;
    }

    let options = ExtractionOptions {
        receipts: pdftract_core::options::ReceiptsMode::Off,
        max_parallel_pages: 1,
        memory_budget_mb: 512,
        full_render: false,
        ocr_dpi_override: None,
        ocr_language: vec!["eng".to_string()],
        markdown_anchors: false,
        max_decompress_bytes: 512 * 1024 * 1024,
        output: Default::default(),
        pages: None,
        password: None,
        http_headers: None,
    };

    let result = extract_pdf(&fixture_path, &options);

    match result {
        Ok(extraction_result) => {
            // Verify reading_order_algorithm is "struct_tree" with high coverage
            let algo = extraction_result
                .metadata
                .reading_order_algorithm
                .expect("reading_order_algorithm should be set");

            assert_eq!(
                algo, "struct_tree",
                "Expected reading_order_algorithm='struct_tree' for high coverage, got '{}'",
                algo
            );

            println!(
                "Integration test passed: reading_order_algorithm = '{}'",
                algo
            );
        }
        Err(e) => {
            panic!("Extraction failed: {}", e);
        }
    }
}
