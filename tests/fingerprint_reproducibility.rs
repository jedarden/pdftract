//! Fingerprint reproducibility and content-sensitivity tests.
//!
//! This test module verifies the fingerprint algorithm's core properties:
//! - Reproducibility: same content produces same fingerprint (INV-3)
//! - Content-sensitivity: different content produces different fingerprints
//! - Metadata independence: metadata-only changes don't affect fingerprint (ADR-008)
//! - Linearization independence: linearized and unlinearized versions match (KU-7)
//!
//! Fixture pairs under `tests/fingerprint/fixtures/` contain:
//! - v1.pdf and v2.pdf: Two PDF variants
//! - expected.txt: Either "MATCH" or "DIFFER"

use pdftract_core::document::compute_pdf_fingerprint;
use std::path::PathBuf;

/// Base directory for fingerprint fixtures.
fn fixtures_dir() -> PathBuf {
    PathBuf::from("tests/fingerprint/fixtures")
}

/// Fixture pair configuration.
struct FixturePair {
    name: &'static str,
    expected_match: bool,
}

/// All fixture pairs to test.
fn fixture_pairs() -> Vec<FixturePair> {
    vec![
        FixturePair {
            name: "acrobat_resave",
            expected_match: true,
        },
        FixturePair {
            name: "byte_identical",
            expected_match: true,
        },
        FixturePair {
            name: "content_edit_one_glyph",
            expected_match: false,
        },
        FixturePair {
            name: "content_edit_one_paragraph",
            expected_match: false,
        },
        FixturePair {
            name: "linearization_toggle",
            expected_match: true,
        },
        FixturePair {
            name: "metadata_only",
            expected_match: true,
        },
        FixturePair {
            name: "pdftk_resave",
            expected_match: true,
        },
        FixturePair {
            name: "qpdf_resave",
            expected_match: true,
        },
    ]
}

/// Test all fixture pairs against their expected outcomes.
#[test]
fn test_fingerprint_fixture_pairs() {
    for fixture in fixture_pairs() {
        let dir = fixtures_dir().join(fixture.name);
        let v1 = dir.join("v1.pdf");
        let v2 = dir.join("v2.pdf");

        let fp1 = compute_pdf_fingerprint(&v1)
            .unwrap_or_else(|e| panic!("Failed to compute fingerprint for {}/v1.pdf: {}", fixture.name, e));
        let fp2 = compute_pdf_fingerprint(&v2)
            .unwrap_or_else(|e| panic!("Failed to compute fingerprint for {}/v2.pdf: {}", fixture.name, e));

        if fixture.expected_match {
            assert_eq!(
                fp1, fp2,
                "Fixture pair '{}' expected MATCH but got different fingerprints:\n  v1: {}\n  v2: {}",
                fixture.name, fp1, fp2
            );
        } else {
            assert_ne!(
                fp1, fp2,
                "Fixture pair '{}' expected DIFFER but got identical fingerprints: {}",
                fixture.name, fp1
            );
        }
    }
}

/// INV-3: 100 invocations on same PDF produce identical fingerprints.
///
/// This test invokes compute_fingerprint() 100 times on acrobat_resave/v1.pdf
/// and verifies all outputs are byte-identical. This catches:
/// - Non-deterministic hash initialization
/// - HashMap iteration order affecting output
/// - Unstable sorting or undefined iteration order
#[test]
fn test_inv3_reproducibility_100_invocations() {
    let dir = fixtures_dir().join("acrobat_resave");
    let pdf_path = dir.join("v1.pdf");

    // Compute first fingerprint
    let first = compute_pdf_fingerprint(&pdf_path)
        .unwrap_or_else(|e| panic!("Failed to compute fingerprint for acrobat_resave/v1.pdf: {}", e));

    // Compute 99 more times and verify all match
    for i in 0..99 {
        let next = compute_pdf_fingerprint(&pdf_path)
            .unwrap_or_else(|e| panic!("Invocation {} failed: {}", i, e));
        assert_eq!(
            next, first,
            "Invocation {} produced different fingerprint:\n  Expected: {}\n  Got: {}",
            i + 2, first, next
        );
    }
}

/// INV-13: Verify fingerprint format matches regex `^pdftract-v1:[0-9a-f]{64}$`.
///
/// This test verifies that all fixture fingerprints produce valid output format.
#[test]
fn test_inv13_fingerprint_format() {
    let regex = regex::Regex::new(r"^pdftract-v1:[0-9a-f]{64}$").unwrap();

    for fixture in fixture_pairs() {
        let dir = fixtures_dir().join(fixture.name);
        let v1 = dir.join("v1.pdf");

        let fingerprint = compute_pdf_fingerprint(&v1)
            .unwrap_or_else(|e| panic!("Failed to compute fingerprint for {}/v1.pdf: {}", fixture.name, e));

        assert!(
            regex.is_match(&fingerprint),
            "Fingerprint '{}' from fixture '{}' does not match INV-13 format",
            fingerprint, fixture.name
        );
    }
}

/// Test critical fixture pairs individually for better failure messages.
///
/// This test runs each critical fixture pair separately so that failures
/// are easier to diagnose.
#[test]
fn test_acrobat_resave_fixture() {
    test_fixture_pair("acrobat_resave", true);
}

#[test]
fn test_qpdf_resave_fixture() {
    test_fixture_pair("qpdf_resave", true);
}

#[test]
fn test_pdftk_resave_fixture() {
    test_fixture_pair("pdftk_resave", true);
}

#[test]
fn test_linearization_toggle_fixture() {
    test_fixture_pair("linearization_toggle", true);
}

#[test]
fn test_metadata_only_fixture() {
    test_fixture_pair("metadata_only", true);
}

#[test]
fn test_content_edit_one_glyph_fixture() {
    test_fixture_pair("content_edit_one_glyph", false);
}

#[test]
fn test_content_edit_one_paragraph_fixture() {
    test_fixture_pair("content_edit_one_paragraph", false);
}

#[test]
fn test_byte_identical_fixture() {
    test_fixture_pair("byte_identical", true);
}

/// Helper to test a single fixture pair.
fn test_fixture_pair(name: &str, expected_match: bool) {
    let dir = fixtures_dir().join(name);
    let v1 = dir.join("v1.pdf");
    let v2 = dir.join("v2.pdf");

    let fp1 = compute_pdf_fingerprint(&v1)
        .unwrap_or_else(|e| panic!("Failed to compute fingerprint for {}/v1.pdf: {}", name, e));
    let fp2 = compute_pdf_fingerprint(&v2)
        .unwrap_or_else(|e| panic!("Failed to compute fingerprint for {}/v2.pdf: {}", name, e));

    if expected_match {
        assert_eq!(fp1, fp2, "Fixture '{}' expected MATCH", name);
    } else {
        assert_ne!(fp1, fp2, "Fixture '{}' expected DIFFER", name);
    }
}

/// Performance test: verify fingerprint computation is fast enough.
///
/// All fixture pairs should complete in under 5 seconds total.
#[test]
fn test_fingerprint_performance() {
    use std::time::Instant;

    let start = Instant::now();

    for fixture in fixture_pairs() {
        let dir = fixtures_dir().join(fixture.name);
        let v1 = dir.join("v1.pdf");

        compute_pdf_fingerprint(&v1)
            .unwrap_or_else(|e| panic!("Failed to compute fingerprint for {}/v1.pdf: {}", fixture.name, e));
    }

    let duration = start.elapsed();

    // Total time for all fixtures should be under 5 seconds
    assert!(
        duration.as_secs() < 5,
        "Fingerprint computation took {} seconds, should be < 5 seconds",
        duration.as_secs()
    );
}

/// Test that byte-identical files produce identical fingerprints.
///
/// This is a sanity check that the fingerprint function is deterministic
/// and doesn't depend on external state (time, random seed, etc.).
#[test]
fn test_byte_identical_produces_same_fingerprint() {
    let dir = fixtures_dir().join("byte_identical");
    let v1 = dir.join("v1.pdf");
    let v2 = dir.join("v2.pdf");

    let fp1 = compute_pdf_fingerprint(&v1).unwrap();
    let fp2 = compute_pdf_fingerprint(&v2).unwrap();

    assert_eq!(fp1, fp2, "Byte-identical files must produce identical fingerprints");
}

/// Test that metadata-only changes don't affect fingerprint.
///
/// This verifies ADR-008: /Title, /Author, /Producer, /CreationDate
/// changes should not change the fingerprint.
#[test]
fn test_metadata_ignored_in_fingerprint() {
    let dir = fixtures_dir().join("metadata_only");
    let v1 = dir.join("v1.pdf");
    let v2 = dir.join("v2.pdf");

    let fp1 = compute_pdf_fingerprint(&v1).unwrap();
    let fp2 = compute_pdf_fingerprint(&v2).unwrap();

    assert_eq!(fp1, fp2, "Metadata-only changes must not affect fingerprint (ADR-008)");
}

/// Test that linearization toggle doesn't affect fingerprint.
///
/// This verifies KU-7: linearized and unlinearized versions
/// should produce the same fingerprint.
#[test]
fn test_linearization_independent() {
    let dir = fixtures_dir().join("linearization_toggle");
    let v1 = dir.join("v1.pdf");
    let v2 = dir.join("v2.pdf");

    let fp1 = compute_pdf_fingerprint(&v1).unwrap();
    let fp2 = compute_pdf_fingerprint(&v2).unwrap();

    assert_eq!(fp1, fp2, "Linearization toggle must not affect fingerprint (KU-7)");
}

/// Test that single glyph removal changes fingerprint.
///
/// This verifies content-sensitivity: removing a single glyph
/// from content must change the fingerprint.
#[test]
fn test_single_glyph_changes_fingerprint() {
    let dir = fixtures_dir().join("content_edit_one_glyph");
    let v1 = dir.join("v1.pdf");
    let v2 = dir.join("v2.pdf");

    let fp1 = compute_pdf_fingerprint(&v1).unwrap();
    let fp2 = compute_pdf_fingerprint(&v2).unwrap();

    assert_ne!(fp1, fp2, "Single glyph removal must change fingerprint");
}

/// Test that paragraph edit changes fingerprint.
///
/// This verifies content-sensitivity: editing a paragraph
/// must change the fingerprint.
#[test]
fn test_paragraph_edit_changes_fingerprint() {
    let dir = fixtures_dir().join("content_edit_one_paragraph");
    let v1 = dir.join("v1.pdf");
    let v2 = dir.join("v2.pdf");

    let fp1 = compute_pdf_fingerprint(&v1).unwrap();
    let fp2 = compute_pdf_fingerprint(&v2).unwrap();

    assert_ne!(fp1, fp2, "Paragraph edit must change fingerprint");
}
