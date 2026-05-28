//! Fingerprint reproducibility tests.
//!
//! This module tests the fingerprint algorithm's reproducibility and
//! content-sensitivity properties.
//!
//! Tests:
//! - INV-3: 100 invocations produce identical output
//! - Fixture pair tests: verify MATCH/DIFFER expectations
//! - Cross-platform: fingerprints match across platforms (CI only)

use std::path::Path;
use pdftract_core::document::PdfExtractor;

/// Helper: compute fingerprint from a PDF file path.
/// Path is relative to the crate root (where fixtures are located).
fn fingerprint_from_path(relative_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    // The fixtures are at tests/fingerprint/fixtures/ from the repo root
    // When running from crates/pdftract-core/, we need to go up two levels
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    let base = Path::new(&cargo_manifest_dir);
    let fixture_path = base
        .parent() // crates
        .and_then(|p| p.parent()) // repo root
        .unwrap_or(base)
        .join(relative_path);

    let extractor = PdfExtractor::open(&fixture_path)
        .map_err(|e| format!("Failed to open {}: {:?}", fixture_path.display(), e))?;
    Ok(extractor.fingerprint().to_string())
}

#[test]
fn test_inv3_reproducibility_100_invocations() {
    //! INV-3: 100 calls on same Document produce identical string.
    //!
    //! Uses the acrobat_resave/v1.pdf fixture as a stable test file.
    let fixture_path = "tests/fingerprint/fixtures/acrobat_resave/v1.pdf";

    // First fingerprint
    let first = fingerprint_from_path(fixture_path)
        .expect("Failed to compute first fingerprint");

    // 99 more invocations, all must match
    for i in 0..99 {
        let next = fingerprint_from_path(fixture_path)
            .expect(&format!("Failed to compute fingerprint (iteration {})", i));
        assert_eq!(
            next, first,
            "Fingerprint must be reproducible (iteration {} differed)",
            i
        );
    }
}

#[test]
fn test_fixture_byte_identical() {
    //! byte_identical: same file copied twice. Expected: MATCH.
    let v1 = fingerprint_from_path("tests/fingerprint/fixtures/byte_identical/v1.pdf")
        .expect("Failed to fingerprint v1");
    let v2 = fingerprint_from_path("tests/fingerprint/fixtures/byte_identical/v2.pdf")
        .expect("Failed to fingerprint v2");

    assert_eq!(v1, v2, "Byte-identical files must have matching fingerprints");
}

#[test]
fn test_fixture_qpdf_resave() {
    //! qpdf_resave: same source through qpdf. Expected: MATCH.
    let v1 = fingerprint_from_path("tests/fingerprint/fixtures/qpdf_resave/v1.pdf")
        .expect("Failed to fingerprint v1");
    let v2 = fingerprint_from_path("tests/fingerprint/fixtures/qpdf_resave/v2.pdf")
        .expect("Failed to fingerprint v2");

    assert_eq!(v1, v2, "qpdf re-save must preserve fingerprint");
}

#[test]
fn test_fixture_acrobat_resave() {
    //! acrobat_resave: simulated Acrobat re-save. Expected: MATCH.
    let v1 = fingerprint_from_path("tests/fingerprint/fixtures/acrobat_resave/v1.pdf")
        .expect("Failed to fingerprint v1");
    let v2 = fingerprint_from_path("tests/fingerprint/fixtures/acrobat_resave/v2.pdf")
        .expect("Failed to fingerprint v2");

    assert_eq!(v1, v2, "Acrobat re-save simulation must preserve fingerprint");
}

#[test]
fn test_fixture_pdftk_resave() {
    //! pdftk_resave: simulated pdftk re-save. Expected: MATCH.
    let v1 = fingerprint_from_path("tests/fingerprint/fixtures/pdftk_resave/v1.pdf")
        .expect("Failed to fingerprint v1");
    let v2 = fingerprint_from_path("tests/fingerprint/fixtures/pdftk_resave/v2.pdf")
        .expect("Failed to fingerprint v2");

    assert_eq!(v1, v2, "pdftk re-save simulation must preserve fingerprint");
}

#[test]
fn test_fixture_linearization_toggle() {
    //! linearization_toggle: unlinearized vs linearized. Expected: MATCH (KU-7).
    let v1 = fingerprint_from_path("tests/fingerprint/fixtures/linearization_toggle/v1.pdf")
        .expect("Failed to fingerprint v1");
    let v2 = fingerprint_from_path("tests/fingerprint/fixtures/linearization_toggle/v2.pdf")
        .expect("Failed to fingerprint v2");

    assert_eq!(v1, v2, "Linearization toggle must preserve fingerprint (KU-7)");
}

#[test]
fn test_fixture_metadata_only() {
    //! metadata_only: metadata changes only. Expected: MATCH (ADR-008).
    let v1 = fingerprint_from_path("tests/fingerprint/fixtures/metadata_only/v1.pdf")
        .expect("Failed to fingerprint v1");
    let v2 = fingerprint_from_path("tests/fingerprint/fixtures/metadata_only/v2.pdf")
        .expect("Failed to fingerprint v2");

    assert_eq!(v1, v2, "Metadata-only changes must preserve fingerprint (ADR-008)");
}

#[test]
fn test_fixture_content_edit_one_glyph() {
    //! content_edit_one_glyph: one glyph removed. Expected: DIFFER.
    let v1 = fingerprint_from_path("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf")
        .expect("Failed to fingerprint v1");
    let v2 = fingerprint_from_path("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf")
        .expect("Failed to fingerprint v2");

    assert_ne!(v1, v2, "Content edit (one glyph) must change fingerprint");
}

#[test]
fn test_fixture_content_edit_one_paragraph() {
    //! content_edit_one_paragraph: one paragraph re-typed. Expected: DIFFER.
    let v1 = fingerprint_from_path("tests/fingerprint/fixtures/content_edit_one_paragraph/v1.pdf")
        .expect("Failed to fingerprint v1");
    let v2 = fingerprint_from_path("tests/fingerprint/fixtures/content_edit_one_paragraph/v2.pdf")
        .expect("Failed to fingerprint v2");

    assert_ne!(v1, v2, "Content edit (one paragraph) must change fingerprint");
}

#[test]
fn test_inv13_fingerprint_format() {
    //! INV-13: all fingerprints match regex `^pdftract-v1:[0-9a-f]{64}$`.
    //!
    //! Verify all fixture PDFs produce properly formatted fingerprints.
    use regex::Regex;

    let regex = Regex::new(r"^pdftract-v1:[0-9a-f]{64}$").unwrap();

    let fixtures = [
        "tests/fingerprint/fixtures/byte_identical/v1.pdf",
        "tests/fingerprint/fixtures/acrobat_resave/v1.pdf",
        "tests/fingerprint/fixtures/qpdf_resave/v1.pdf",
        "tests/fingerprint/fixtures/linearization_toggle/v1.pdf",
        "tests/fingerprint/fixtures/metadata_only/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf",
        "tests/fingerprint/fixtures/content_edit_one_paragraph/v1.pdf",
    ];

    for path in fixtures {
        let fingerprint = fingerprint_from_path(path)
            .expect(&format!("Failed to fingerprint {}", path));
        assert!(
            regex.is_match(&fingerprint),
            "Fingerprint '{}' for {} must match INV-13 format",
            fingerprint, path
        );
    }
}

#[test]
#[cfg(feature = "cross-platform-test")]
fn test_cross_platform_fingerprints() {
    //! Cross-platform test: verify fingerprints match across platforms.
    //!
    //! This test is enabled only via the `cross-platform-test` feature,
    //! which is used in CI to compare fingerprints across:
    //! - linux-gnu
    //! - linux-musl
    //! - aarch64-linux-musl
    //!
    //! The expected fingerprints are baked into the test binary at compile time.
    //!
    //! Usage in CI:
    //! 1. Build and test on reference platform (linux-gnu), capture fingerprints
    //! 2. Bake fingerprints into EXPECTED_FINGERPRINTS below
    //! 3. Build and test on other platforms, verify they match

    // Expected fingerprints captured from linux-gnu
    // Format: (fixture_path, expected_fingerprint)
    const EXPECTED_FINGERPRINTS: &[(&str, &str)] = &[
        ("tests/fingerprint/fixtures/byte_identical/v1.pdf", "PLACEHOLDER"),
        ("tests/fingerprint/fixtures/acrobat_resave/v1.pdf", "PLACEHOLDER"),
        ("tests/fingerprint/fixtures/qpdf_resave/v1.pdf", "PLACEHOLDER"),
        ("tests/fingerprint/fixtures/linearization_toggle/v1.pdf", "PLACEHOLDER"),
        ("tests/fingerprint/fixtures/metadata_only/v1.pdf", "PLACEHOLDER"),
        ("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf", "PLACEHOLDER"),
        ("tests/fingerprint/fixtures/content_edit_one_paragraph/v1.pdf", "PLACEHOLDER"),
    ];

    for (path, expected) in EXPECTED_FINGERPRINTS {
        if *expected == "PLACEHOLDER" {
            panic!("Cross-platform test not configured: replace PLACEHOLDER with actual fingerprints from linux-gnu");
        }

        let fingerprint = fingerprint_from_path(path)
            .expect(&format!("Failed to fingerprint {}", path));

        assert_eq!(
            fingerprint, *expected,
            "Fingerprint for {} differs across platforms (expected {}, got {})",
            path, expected, fingerprint
        );
    }
}
