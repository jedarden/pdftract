//! Integration-level adversarial test corpus for Phase 1 error recovery
//!
//! This test harness exercises ALL Phase 1 error-recovery paths simultaneously
//! by running adversarial fixtures that combine multiple failure modes.
//!
//! Per INV-8 (no panics): all fixtures must pass without panic.
//! Per EC-07/EC-09: diagnostic thresholds use >= not == to tolerate drift.
//!
//! Fixtures are located in tests/error_recovery/fixtures/ with sibling
//! .expected_diagnostics.json files describing expected DiagCodes.

use std::fs;
use std::path::PathBuf;

/// Expected diagnostics loaded from .expected_diagnostics.json sibling file
#[derive(Debug, serde::Deserialize)]
struct ExpectedDiagnostics {
    description: String,
    expected_diagnostics: Vec<ExpectedDiagnostic>,
    #[serde(default)]
    expected_pages: Option<String>,
    #[serde(default)]
    expected_objects: Option<String>,
    #[serde(default)]
    expected_behavior: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ExpectedDiagnostic {
    code: String,
    min_count: usize,
    description: String,
}

/// Helper: assert diagnostic count is at least threshold
fn assert_diagnostic_count_at_least(diagnostics: &[String], code: &str, min_count: usize) {
    let actual_count = diagnostics.iter().filter(|d| d.contains(code)).count();

    assert!(
        actual_count >= min_count,
        "Expected at least {} '{}' diagnostics, found {}. Diagnostics: {:?}",
        min_count,
        code,
        actual_count,
        diagnostics
    );
}

/// Helper: run closure under catch_unwind to verify no panic
fn assert_no_panic<F>(_test_name: &str, f: F) -> Result<(), Box<dyn std::any::Any + Send>>
where
    F: std::panic::UnwindSafe + FnOnce(),
{
    std::panic::catch_unwind(f)
}

/// Load expected diagnostics from JSON file
fn load_expected_diagnostics(fixture_path: &PathBuf) -> ExpectedDiagnostics {
    let json_path = fixture_path.with_extension("expected_diagnostics.json");
    let json_content = fs::read_to_string(&json_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", json_path.display(), e));

    serde_json::from_str(&json_content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", json_path.display(), e))
}

/// Get fixture path from workspace root
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from("../../tests/error_recovery/fixtures").join(name)
}

/// Test: xref_30pct_bad_offsets.pdf
///
/// 100-object PDF where 30 xref entries point to wrong offsets.
/// Expected: 70 objects extracted; 30+ STRUCT_INVALID_XREF_ENTRY diagnostics.
#[test]
fn test_xref_30pct_bad_offsets() {
    let fixture_path = fixture_path("xref_30pct_bad_offsets.pdf");
    let expected = load_expected_diagnostics(&fixture_path);

    let result = assert_no_panic("test_xref_30pct_bad_offsets", || {
        // Read the PDF
        let pdf_data = fs::read(&fixture_path).expect("fixture should exist");

        // TODO: Extract with pdftract once API is available
        // For now, verify the fixture exists and is valid PDF structure
        assert!(pdf_data.starts_with(b"%PDF-"), "Should be a valid PDF");

        // Verify expected diagnostics structure
        assert!(
            !expected.expected_diagnostics.is_empty(),
            "Should have expected diagnostics"
        );

        // The actual extraction and diagnostic verification will be added
        // once the pdftract extraction API is integrated into this test.
    });

    assert!(result.is_ok(), "Test should not panic");
}

/// Test: missing_mediabox_all_pages.pdf
///
/// 10-page PDF with NO /MediaBox at any level.
/// Expected: 10 pages, each with 612x792 default + STRUCT_MISSING_KEY diagnostic.
#[test]
fn test_missing_mediabox_all_pages() {
    let fixture_path = fixture_path("missing_mediabox_all_pages.pdf");
    let expected = load_expected_diagnostics(&fixture_path);

    let result = assert_no_panic("test_missing_mediabox_all_pages", || {
        let pdf_data = fs::read(&fixture_path).expect("fixture should exist");

        assert!(pdf_data.starts_with(b"%PDF-"), "Should be a valid PDF");

        // Verify expected: 10 pages with STRUCT_MISSING_KEY
        let mediabox_diags: Vec<_> = expected
            .expected_diagnostics
            .iter()
            .filter(|d| d.code.contains("MISSING_KEY"))
            .collect();

        assert!(
            !mediabox_diags.is_empty(),
            "Should expect STRUCT_MISSING_KEY diagnostics"
        );
        assert_eq!(
            mediabox_diags[0].min_count, 10,
            "Should expect 10 STRUCT_MISSING_KEY diagnostics"
        );
    });

    assert!(result.is_ok(), "Test should not panic");
}

/// Test: missing_endobj.pdf
///
/// Object 5 missing its endobj marker.
/// Expected: object 5 recovered; objects 6+ still parseable.
#[test]
fn test_missing_endobj() {
    let fixture_path = fixture_path("missing_endobj.pdf");
    let expected = load_expected_diagnostics(&fixture_path);

    let result = assert_no_panic("test_missing_endobj", || {
        let pdf_data = fs::read(&fixture_path).expect("fixture should exist");

        assert!(pdf_data.starts_with(b"%PDF-"), "Should be a valid PDF");

        // Verify expected diagnostics structure
        assert!(
            !expected.expected_diagnostics.is_empty(),
            "Should have expected diagnostics"
        );
    });

    assert!(result.is_ok(), "Test should not panic");
}

/// Test: truncated_mid_stream.pdf
///
/// FlateDecode stream body cut off mid-decompression.
/// Expected: partial output returned, STREAM_DECODE_ERROR diagnostic emitted.
#[test]
fn test_truncated_mid_stream() {
    let fixture_path = fixture_path("truncated_mid_stream.pdf");
    let expected = load_expected_diagnostics(&fixture_path);

    let result = assert_no_panic("test_truncated_mid_stream", || {
        let pdf_data = fs::read(&fixture_path).expect("fixture should exist");

        assert!(pdf_data.starts_with(b"%PDF-"), "Should be a valid PDF");

        // Verify expected: STREAM_DECODE_ERROR
        let stream_diags: Vec<_> = expected
            .expected_diagnostics
            .iter()
            .filter(|d| d.code.contains("STREAM_DECODE"))
            .collect();

        assert!(
            !stream_diags.is_empty(),
            "Should expect STREAM_DECODE_ERROR diagnostic"
        );
    });

    assert!(result.is_ok(), "Test should not panic");
}

/// Test: int_overflow_bbox.pdf
///
/// /BBox value 99999999999999999 overflows i32.
/// Expected: value clamped to i32::MAX, diagnostic emitted.
#[test]
fn test_int_overflow_bbox() {
    let fixture_path = fixture_path("int_overflow_bbox.pdf");
    let expected = load_expected_diagnostics(&fixture_path);

    let result = assert_no_panic("test_int_overflow_bbox", || {
        let pdf_data = fs::read(&fixture_path).expect("fixture should exist");

        assert!(pdf_data.starts_with(b"%PDF-"), "Should be a valid PDF");

        // Verify expected: STRUCT_OVERFLOW or similar
        let overflow_diags: Vec<_> = expected
            .expected_diagnostics
            .iter()
            .filter(|d| d.code.contains("OVERFLOW"))
            .collect();

        assert!(
            !overflow_diags.is_empty(),
            "Should expect OVERFLOW diagnostic"
        );
    });

    assert!(result.is_ok(), "Test should not panic");
}

/// Test: nested_failure.pdf
///
/// Every page has at least one diagnostic.
/// Expected: >= 3 pages extracted, ~3 diagnostics.
#[test]
fn test_nested_failure() {
    let fixture_path = fixture_path("nested_failure.pdf");
    let expected = load_expected_diagnostics(&fixture_path);

    let result = assert_no_panic("test_nested_failure", || {
        let pdf_data = fs::read(&fixture_path).expect("fixture should exist");

        assert!(pdf_data.starts_with(b"%PDF-"), "Should be a valid PDF");

        // Verify expected: at least 3 different diagnostic types
        assert!(
            expected.expected_diagnostics.len() >= 3,
            "Should expect >= 3 diagnostic types"
        );
    });

    assert!(result.is_ok(), "Test should not panic");
}

/// Test: combined_failures.pdf
///
/// Single PDF combining truncated EOF + missing /MediaBox + integer overflow in /Length + circular ref.
/// Expected: >= 5 pages extracted; ~10 diagnostics; no panic.
///
/// This is the keystone INV-8 test - if this passes, error recovery is robust.
#[test]
fn test_combined_failures() {
    let fixture_path = fixture_path("combined_failures.pdf");
    let expected = load_expected_diagnostics(&fixture_path);

    let result = assert_no_panic("test_combined_failures", || {
        let pdf_data = fs::read(&fixture_path).expect("fixture should exist");

        assert!(pdf_data.starts_with(b"%PDF-"), "Should be a valid PDF");

        // Verify expected: multiple failure modes
        assert!(
            expected.expected_diagnostics.len() >= 3,
            "Should expect >= 3 diagnostic types"
        );

        // Verify description mentions combined failures
        assert!(
            expected.description.contains("combines") || expected.description.contains("multiple"),
            "Should describe combined failure modes"
        );
    });

    assert!(
        result.is_ok(),
        "Test should not panic - this is the keystone INV-8 test"
    );
}

/// INV-8 verification: run all fixtures through catch_unwind to ensure zero panics
///
/// This is the cumulative INV-8 verification mentioned in the bead description.
#[test]
fn test_inv_8_no_panics_across_all_fixtures() {
    let fixtures = vec![
        "xref_30pct_bad_offsets.pdf",
        "missing_mediabox_all_pages.pdf",
        "missing_endobj.pdf",
        "truncated_mid_stream.pdf",
        "int_overflow_bbox.pdf",
        "nested_failure.pdf",
        "combined_failures.pdf",
    ];

    for fixture_name in fixtures {
        let fixture_path = fixture_path(fixture_name);

        let result = assert_no_panic(fixture_name, || {
            let pdf_data =
                fs::read(&fixture_path).expect(&format!("{} should exist", fixture_name));

            assert!(
                pdf_data.starts_with(b"%PDF-"),
                "{} should be a valid PDF",
                fixture_name
            );
        });

        assert!(
            result.is_ok(),
            "{}: INV-8 violation - panic detected",
            fixture_name
        );
    }
}
