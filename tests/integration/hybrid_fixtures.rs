//! Integration tests for hybrid PDF fixtures.
//!
//! This test module validates that all 10 hybrid PDF fixtures are correctly
//! classified as PageClass::Hybrid (page_type = "mixed") and meet the >=15%
//! grid-cell coverage threshold (>=10 of 64 cells with image-heavy content).
//!
//! Per the acceptance criteria for bead bf-17rvbg:
//! - Tests verify PageClass::Hybrid classification for all 10 fixtures
//! - Tests verify the >=15% grid-cell coverage rule
//! - Tests document any fixtures that fail classification (expected FAIL list)
//!
//! # Helper Module
//!
//! This test uses helper functions from `crate::fixtures::hybrid` (defined in
//! `tests/fixtures/hybrid/mod.rs`) which provide utilities for loading fixtures,
//! running classification, and validating results.

use std::path::Path;

use crate::fixtures::hybrid::{
    assert_hybrid_classification, load_and_classify_fixture, MIN_HYBRID_CELLS, FIXTURE_DIR,
};

/// List of all 10 hybrid fixtures that should classify as PageClass::Hybrid.
const HYBRID_FIXTURES: &[&str] = &[
    "hybrid-001-vector-header-over-scan.pdf",
    "hybrid-002-vector-form-over-scan.pdf",
    "hybrid-003-mixed-column-layout.pdf",
    "hybrid-004-watermark-over-scan.pdf",
    "hybrid-005-vector-footer-over-scan.pdf",
    "hybrid-006-stamp-annotation.pdf",
    "hybrid-007-textbox-overlay.pdf",
    "hybrid-008-rotated-vector.pdf",
    "hybrid-009-transparent-vector.pdf",
    "hybrid-010-complex-layered.pdf",
];

#[test]
fn test_all_hybrid_fixtures_classify_as_mixed() {
    // Test all 10 hybrid fixtures and verify they classify as PageClass::Hybrid
    let mut passed = 0;
    let mut failed = Vec::new();

    for &fixture_name in HYBRID_FIXTURES {
        let result = match load_and_classify_fixture(fixture_name) {
            Ok(r) => r,
            Err(e) => {
                let error_msg = format!("ERROR: {}", e);
                println!(
                    " Hybrid fixture: {:40} | {}",
                    fixture_name, error_msg
                );
                failed.push((fixture_name, error_msg));
                continue;
            }
        };

        // Check if classification meets Hybrid criteria
        let status = if result.class == pdftract_core::page_class::PageClass::Hybrid {
            "PASS"
        } else {
            "FAIL"
        };

        println!(
            " Hybrid fixture: {:40} | class: {:?} | {}",
            fixture_name, result.class, status
        );

        if result.class == pdftract_core::page_class::PageClass::Hybrid {
            passed += 1;
            assert_hybrid_classification(&result, fixture_name, MIN_HYBRID_CELLS);
        } else {
            failed.push((fixture_name, format!("{:?}", result.class)));
        }
    }

    // Print summary
    println!("\n--- Hybrid Fixture Classification Summary ---");
    println!("Total fixtures: {}", HYBRID_FIXTURES.len());
    println!("Passed: {}", passed);
    println!("Failed: {}", failed.len());

    if !failed.is_empty() {
        println!("\nFailed fixtures:");
        for (fixture_name, class) in &failed {
            println!("  - {}: got class='{}', expected PageClass::Hybrid", fixture_name, class);
        }
    }

    // Assert that all fixtures passed
    assert!(
        failed.is_empty(),
        "Expected all {} hybrid fixtures to classify as PageClass::Hybrid, but {} failed:\n{:?}",
        HYBRID_FIXTURES.len(),
        failed.len(),
        failed
    );

    // Verify we tested exactly 10 fixtures (KU-2 requirement)
    assert_eq!(
        HYBRID_FIXTURES.len(),
        10,
        "KU-2 requires exactly 10 hybrid fixtures, found {}",
        HYBRID_FIXTURES.len()
    );
}

#[test]
fn test_hybrid_001_vector_header_over_scan() {
    let result = load_and_classify_fixture("hybrid-001-vector-header-over-scan.pdf")
        .expect("Failed to load hybrid-001");
    assert_hybrid_classification(&result, "hybrid-001", MIN_HYBRID_CELLS);
}

#[test]
fn test_hybrid_002_vector_form_over_scan() {
    let result = load_and_classify_fixture("hybrid-002-vector-form-over-scan.pdf")
        .expect("Failed to load hybrid-002");
    assert_hybrid_classification(&result, "hybrid-002", MIN_HYBRID_CELLS);
}

#[test]
fn test_hybrid_003_mixed_column_layout() {
    let result = load_and_classify_fixture("hybrid-003-mixed-column-layout.pdf")
        .expect("Failed to load hybrid-003");
    assert_hybrid_classification(&result, "hybrid-003", MIN_HYBRID_CELLS);
}

#[test]
fn test_hybrid_004_watermark_over_scan() {
    let result = load_and_classify_fixture("hybrid-004-watermark-over-scan.pdf")
        .expect("Failed to load hybrid-004");
    assert_hybrid_classification(&result, "hybrid-004", MIN_HYBRID_CELLS);
}

#[test]
fn test_hybrid_005_vector_footer_over_scan() {
    let result = load_and_classify_fixture("hybrid-005-vector-footer-over-scan.pdf")
        .expect("Failed to load hybrid-005");
    assert_hybrid_classification(&result, "hybrid-005", MIN_HYBRID_CELLS);
}

#[test]
fn test_hybrid_006_stamp_annotation() {
    let result = load_and_classify_fixture("hybrid-006-stamp-annotation.pdf")
        .expect("Failed to load hybrid-006");
    assert_hybrid_classification(&result, "hybrid-006", MIN_HYBRID_CELLS);
}

#[test]
fn test_hybrid_007_textbox_overlay() {
    let result = load_and_classify_fixture("hybrid-007-textbox-overlay.pdf")
        .expect("Failed to load hybrid-007");
    assert_hybrid_classification(&result, "hybrid-007", MIN_HYBRID_CELLS);
}

#[test]
fn test_hybrid_008_rotated_vector() {
    let result = load_and_classify_fixture("hybrid-008-rotated-vector.pdf")
        .expect("Failed to load hybrid-008");
    assert_hybrid_classification(&result, "hybrid-008", MIN_HYBRID_CELLS);
}

#[test]
fn test_hybrid_009_transparent_vector() {
    let result = load_and_classify_fixture("hybrid-009-transparent-vector.pdf")
        .expect("Failed to load hybrid-009");
    assert_hybrid_classification(&result, "hybrid-009", MIN_HYBRID_CELLS);
}

#[test]
fn test_hybrid_010_complex_layered() {
    let result = load_and_classify_fixture("hybrid-010-complex-layered.pdf")
        .expect("Failed to load hybrid-010");
    assert_hybrid_classification(&result, "hybrid-010", MIN_HYBRID_CELLS);
}

#[test]
fn test_hybrid_fixture_count_matches_ku2_requirement() {
    // KU-2 requires exactly 10 hybrid fixtures
    // This test verifies the test suite is complete
    assert_eq!(
        HYBRID_FIXTURES.len(),
        10,
        "KU-2 requirement: 10 hybrid fixtures needed, but {} are defined",
        HYBRID_FIXTURES.len()
    );

    // Verify all fixture files exist on disk
    for &fixture_name in HYBRID_FIXTURES {
        let fixture_path = Path::new(FIXTURE_DIR).join(fixture_name);
        assert!(
            fixture_path.exists(),
            "KU-2 fixture missing: {}",
            fixture_path.display()
        );
    }

    println!(
        "✓ KU-2 requirement satisfied: {} hybrid fixtures present and verified",
        HYBRID_FIXTURES.len()
    );
}
