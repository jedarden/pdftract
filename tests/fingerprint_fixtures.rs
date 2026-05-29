//! Fingerprint reproducibility and content-sensitivity tests.
//!
//! This test module verifies the fingerprint algorithm's properties using
//! a corpus of fixture pairs that test reproducibility and content-sensitivity.
//!
//! Fixture pairs are in tests/fingerprint/fixtures/<pair_name>/:
//! - v1.pdf: First variant
//! - v2.pdf: Second variant
//! - expected.txt: Either "MATCH" (fingerprints should be identical) or "DIFFER" (should differ)

use pdftract_core::document::parse_pdf_file;
use std::path::PathBuf;
use std::fs;

/// Fixture pair descriptor.
struct FixturePair {
    name: &'static str,
    expected_match: bool,
}

impl FixturePair {
    /// Path to the fixture directory.
    fn dir(&self) -> PathBuf {
        PathBuf::from("tests/fingerprint/fixtures").join(self.name)
    }

    /// Path to v1.pdf.
    fn v1_path(&self) -> PathBuf {
        self.dir().join("v1.pdf")
    }

    /// Path to v2.pdf.
    fn v2_path(&self) -> PathBuf {
        self.dir().join("v2.pdf")
    }

    /// Read the expected.txt file.
    fn expected_from_file(&self) -> String {
        let expected_path = self.dir().join("expected.txt");
        fs::read_to_string(&expected_path)
            .unwrap_or_else(|_| panic!("Failed to read expected.txt for {}", self.name))
            .trim()
            .to_owned()
    }
}

/// All fixture pairs.
const FIXTURE_PAIRS: &[FixturePair] = &[
    FixturePair { name: "byte_identical", expected_match: true },
    FixturePair { name: "acrobat_resave", expected_match: true },
    FixturePair { name: "pdftk_resave", expected_match: true },
    FixturePair { name: "qpdf_resave", expected_match: true },
    FixturePair { name: "linearization_toggle", expected_match: true },
    FixturePair { name: "metadata_only", expected_match: true },
    FixturePair { name: "content_edit_one_glyph", expected_match: false },
    FixturePair { name: "content_edit_one_paragraph", expected_match: false },
];

#[test]
fn test_fingerprint_fixture_pairs() {
    for fixture in FIXTURE_PAIRS {
        println!("Testing fixture pair: {}", fixture.name);

        let v1_path = fixture.v1_path();
        let v2_path = fixture.v2_path();

        assert!(v1_path.exists(), "v1.pdf does not exist for {}", fixture.name);
        assert!(v2_path.exists(), "v2.pdf does not exist for {}", fixture.name);

        // Parse both PDFs and compute fingerprints
        let (fp1, _, _, _) = parse_pdf_file(&v1_path)
            .unwrap_or_else(|e| panic!("Failed to parse v1.pdf for {}: {}", fixture.name, e));

        let (fp2, _, _, _) = parse_pdf_file(&v2_path)
            .unwrap_or_else(|e| panic!("Failed to parse v2.pdf for {}: {}", fixture.name, e));

        // Verify INV-13 format: ^pdftract-v1:[0-9a-f]{64}$
        let regex = regex::Regex::new(r"^pdftract-v1:[0-9a-f]{64}$").unwrap();
        assert!(
            regex.is_match(&fp1),
            "v1.pdf fingerprint '{}' does not match INV-13 format for {}",
            fp1,
            fixture.name
        );
        assert!(
            regex.is_match(&fp2),
            "v2.pdf fingerprint '{}' does not match INV-13 format for {}",
            fp2,
            fixture.name
        );

        // Check match or differ based on expected
        let match_expected = fixture.expected_match;
        let fingerprints_match = fp1 == fp2;

        if match_expected {
            assert!(
                fingerprints_match,
                "Fingerprints should MATCH for {} but got:\n  v1: {}\n  v2: {}",
                fixture.name, fp1, fp2
            );
        } else {
            assert!(
                !fingerprints_match,
                "Fingerprints should DIFFER for {} but both are: {}",
                fixture.name, fp1
            );
        }

        // Also verify against expected.txt file
        let expected_from_file = fixture.expected_from_file();
        match expected_from_file.as_str() {
            "MATCH" => assert!(fingerprints_match, "expected.txt says MATCH but fingerprints differ for {}", fixture.name),
            "DIFFER" => assert!(!fingerprints_match, "expected.txt says DIFFER but fingerprints match for {}", fixture.name),
            _ => panic!("Invalid expected.txt content '{}' for {}", expected_from_file, fixture.name),
        }

        println!("  ✓ {}: {} (v1: {})", fixture.name, if fingerprints_match { "MATCH" } else { "DIFFER" }, fp1);
    }
}

#[test]
fn test_inv3_reproducibility() {
    // INV-3: 100 calls on same Document produce identical string
    let fixture = &FIXTURE_PAIRS[0]; // byte_identical
    let v1_path = fixture.v1_path();

    let (first_fp, _, _, _) = parse_pdf_file(&v1_path)
        .unwrap_or_else(|e| panic!("Failed to parse v1.pdf for reproducibility test: {}", e));

    // Run 99 more times and verify all match the first
    for i in 1..100 {
        let (fp, _, _, _) = parse_pdf_file(&v1_path)
            .unwrap_or_else(|e| panic!("Failed to parse v1.pdf on iteration {}: {}", i, e));

        assert_eq!(
            fp, first_fp,
            "Fingerprint changed on iteration {}: was '{}', now '{}'",
            i, first_fp, fp
        );
    }

    println!("INV-3 reproducibility test passed: 100 invocations produced identical fingerprints");
}

#[test]
fn test_inv13_fingerprint_format() {
    // INV-13: All fingerprint outputs match ^pdftract-v1:[0-9a-f]{64}$
    let regex = regex::Regex::new(r"^pdftract-v1:[0-9a-f]{64}$").unwrap();

    for fixture in FIXTURE_PAIRS {
        let v1_path = fixture.v1_path();

        let (fp, _, _, _) = parse_pdf_file(&v1_path)
            .unwrap_or_else(|e| panic!("Failed to parse v1.pdf for {}: {}", fixture.name, e));

        assert!(
            regex.is_match(&fp),
            "Fingerprint '{}' for {} does not match INV-13 format",
            fp, fixture.name
        );
    }
}

#[test]
fn test_performance_fixture_corpus() {
    // Performance requirement: total corpus < 5 seconds
    use std::time::Instant;

    let start = Instant::now();

    for fixture in FIXTURE_PAIRS {
        let v1_path = fixture.v1_path();
        let v2_path = fixture.v2_path();

        let _ = parse_pdf_file(&v1_path)
            .unwrap_or_else(|e| panic!("Failed to parse v1.pdf for {}: {}", fixture.name, e));
        let _ = parse_pdf_file(&v2_path)
            .unwrap_or_else(|e| panic!("Failed to parse v2.pdf for {}: {}", fixture.name, e));
    }

    let duration = start.elapsed();

    println!("Total corpus time: {:?}", duration);
    assert!(
        duration.as_secs() < 5,
        "Fixture corpus took {} seconds, should be < 5 seconds",
        duration.as_secs()
    );
}
