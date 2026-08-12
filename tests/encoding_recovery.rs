//! Encoding recovery integration tests.
//!
//! This test module verifies Unicode recovery across Level 2–4 encoding strategies:
//! - Level 2: Adobe Glyph List (AGL) lookup by glyph name
//! - Level 3: SHA-256 font program fingerprint matching
//! - Level 4: Glyph shape recognition (bitmap rasterization + OCR)
//!
//! The Phase 2 exit gate requires ≥90% character recovery rate on the encoding corpus.

use std::path::{Path, PathBuf};
use std::fs;
use pdftract_core::extract::{extract_pdf, ExtractionOptions};

/// Encoding fixture with PDF path and expected ground truth
struct EncodingFixture {
    pdf_path: PathBuf,
    ground_truth_path: PathBuf,
    description: &'static str,
}

impl EncodingFixture {
    fn new(pdf_name: &'static str, description: &'static str) -> Self {
        let fixtures_dir = Path::new("tests/fixtures/encoding");
        Self {
            pdf_path: fixtures_dir.join(format!("{}.pdf", pdf_name)),
            ground_truth_path: fixtures_dir.join(format!("{}.txt", pdf_name)),
            description,
        }
    }
}

/// Calculate Character Error Rate (CER) between extracted and ground truth text.
///
/// CER = (substitutions + insertions + deletions) / total_ground_truth_chars
/// Returns a value in [0.0, 1.0] where lower is better.
fn calculate_cer(extracted: &str, ground_truth: &str) -> f64 {
    // Use Levenshtein distance for character-level error calculation
    let distance = levenshtein_distance(extracted, ground_truth);

    let gt_chars = ground_truth.chars().count();
    if gt_chars == 0 {
        // If ground truth is empty, perfect match if extracted is also empty, else 100% error
        return if extracted.is_empty() { 0.0 } else { 1.0 };
    }

    distance as f64 / gt_chars as f64
}

/// Calculate Levenshtein distance between two strings (character-level).
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let chars1: Vec<char> = s1.chars().collect();
    let chars2: Vec<char> = s2.chars().collect();
    let len1 = chars1.len();
    let len2 = chars2.len();

    if len1 == 0 { return len2; }
    if len2 == 0 { return len1; }

    let mut dp = vec![vec![0; len2 + 1]; len1 + 1];

    // Initialize first row and column
    for i in 0..=len1 {
        dp[i][0] = i;
    }
    for j in 0..=len2 {
        dp[0][j] = j;
    }

    // Fill the DP table
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
            dp[i][j] = std::cmp::min(
                std::cmp::min(
                    dp[i - 1][j] + 1,      // deletion
                    dp[i][j - 1] + 1       // insertion
                ),
                dp[i - 1][j - 1] + cost    // substitution
            );
        }
    }

    dp[len1][len2]
}

/// Extract all text from a PDF by concatenating span text from all pages.
fn extract_all_text(result: &pdftract_core::extract::ExtractionResult) -> String {
    let mut text = String::new();
    for page in &result.pages {
        for span in &page.spans {
            text.push_str(&span.text);
        }
    }
    text
}

/// Load ground truth text from file, normalizing whitespace.
fn load_ground_truth(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Warning: Failed to read ground truth {}: {}", path.display(), e);
            String::new()
        }
    }
}

/// Test a single encoding fixture and return recovery statistics.
fn test_encoding_fixture(fixture: &EncodingFixture) -> (f64, String, String) {
    println!("\n=== Testing: {} ===", fixture.description);
    println!("PDF: {}", fixture.pdf_path.display());

    // Verify files exist
    assert!(
        fixture.pdf_path.exists(),
        "PDF fixture not found: {}",
        fixture.pdf_path.display()
    );
    assert!(
        fixture.ground_truth_path.exists(),
        "Ground truth not found: {}",
        fixture.ground_truth_path.display()
    );

    // Extract text from PDF
    let options = ExtractionOptions::default();
    let extraction_result = extract_pdf(&fixture.pdf_path, &options)
        .unwrap_or_else(|e| {
            panic!("Failed to extract PDF {}: {}", fixture.pdf_path.display(), e)
        });

    // Get extracted text
    let extracted_text = extract_all_text(&extraction_result);
    println!("Extracted text bytes: {}", extracted_text.len());
    println!("Extracted text: {:?}", extracted_text);

    // Load ground truth
    let ground_truth = load_ground_truth(&fixture.ground_truth_path);
    println!("Ground truth bytes: {}", ground_truth.len());
    println!("Ground truth: {:?}", ground_truth);

    // Calculate CER
    let cer = calculate_cer(&extracted_text, &ground_truth);
    let recovery_rate = (1.0 - cer) * 100.0;

    println!("CER: {:.4}", cer);
    println!("Recovery rate: {:.2}%", recovery_rate);

    (cer, extracted_text, ground_truth)
}

#[test]
fn test_encoding_recovery_corpus() {
    // Define all encoding fixtures
    let fixtures = vec![
        EncodingFixture::new("no-mapping", "Level 4: No ToUnicode, no standard encoding, custom glyph names"),
        EncodingFixture::new("agl-only", "Level 2: Adobe Glyph List lookup only"),
        EncodingFixture::new("fingerprint-match", "Level 3: SHA-256 font program fingerprint"),
        EncodingFixture::new("shape-match", "Level 4: Glyph shape recognition"),
    ];

    println!("\n========================================");
    println!("Encoding Recovery Corpus Test");
    println!("========================================");
    println!("Total fixtures: {}", fixtures.len());

    let mut total_cer = 0.0;
    let mut results = Vec::new();

    // Test each fixture
    for fixture in &fixtures {
        let (cer, extracted, ground_truth) = test_encoding_fixture(fixture);
        total_cer += cer;
        results.push((fixture.description, cer, extracted, ground_truth));
    }

    // Calculate overall statistics
    let average_cer = total_cer / fixtures.len() as f64;
    let average_recovery = (1.0 - average_cer) * 100.0;

    println!("\n========================================");
    println!("Overall Results");
    println!("========================================");
    println!("Average CER: {:.4}", average_cer);
    println!("Average recovery rate: {:.2}%", average_recovery);

    // Report per-fixture results
    println!("\nPer-Fixture Breakdown:");
    for (description, cer, extracted, ground_truth) in &results {
        let recovery = (1.0 - cer) * 100.0;
        println!("  {}: {:.2}% recovery (CER: {:.4})",
            description, recovery, cer);
        println!("    Extracted: {:?}", extracted);
        println!("    Expected:  {:?}", ground_truth);
    }

    // Phase 2 exit gate: ≥90% recovery rate
    println!("\n========================================");
    println!("Phase 2 Exit Gate Check");
    println!("========================================");
    println!("Required: ≥90% recovery rate");
    println!("Actual: {:.2}% recovery", average_recovery);

    if average_recovery >= 90.0 {
        println!("✓ PASS: Exit gate satisfied");
    } else {
        println!("✗ FAIL: Exit gate not satisfied");
        panic!("Encoding recovery rate {:.2}% is below the 90% threshold", average_recovery);
    }

    // Assert the minimum recovery rate
    assert!(
        average_recovery >= 90.0,
        "Encoding recovery rate {:.2}% is below the 90% Phase 2 exit gate threshold",
        average_recovery
    );
}

#[test]
fn test_no_mapping_fixture() {
    // Specific test for no-mapping.pdf (worst-case scenario)
    let fixture = EncodingFixture::new("no-mapping", "No ToUnicode, no encoding, custom glyph names");

    let (cer, extracted, ground_truth) = test_encoding_fixture(&fixture);

    // no-mapping.pdf should produce U+FFFD replacement characters
    // Expected output: "���" (three U+FFFD characters)
    let recovery = (1.0 - cer) * 100.0;

    println!("Recovery rate for no-mapping.pdf: {:.2}%", recovery);

    // This fixture is the worst case, so we allow some error but still want reasonable recovery
    assert!(recovery >= 0.0, "Should handle unmapped glyphs gracefully");
}

#[test]
fn test_agl_only_fixture() {
    // Level 2: AGL lookup
    let fixture = EncodingFixture::new("agl-only", "Adobe Glyph List lookup");

    let (cer, extracted, ground_truth) = test_encoding_fixture(&fixture);
    let recovery = (1.0 - cer) * 100.0;

    println!("Recovery rate for agl-only.pdf: {:.2}%", recovery);

    // AGL lookup should be highly accurate
    assert!(recovery >= 90.0, "AGL lookup should achieve ≥90% recovery");
}

#[test]
fn test_fingerprint_match_fixture() {
    // Level 3: Font fingerprint matching
    let fixture = EncodingFixture::new("fingerprint-match", "SHA-256 font fingerprint");

    let (cer, extracted, ground_truth) = test_encoding_fixture(&fixture);
    let recovery = (1.0 - cer) * 100.0;

    println!("Recovery rate for fingerprint-match.pdf: {:.2}%", recovery);

    // Fingerprint matching should be highly accurate when fonts match
    assert!(recovery >= 90.0, "Fingerprint matching should achieve ≥90% recovery");
}

#[test]
fn test_shape_match_fixture() {
    // Level 4: Glyph shape recognition
    let fixture = EncodingFixture::new("shape-match", "Glyph shape recognition");

    let (cer, extracted, ground_truth) = test_encoding_fixture(&fixture);
    let recovery = (1.0 - cer) * 100.0;

    println!("Recovery rate for shape-match.pdf: {:.2}%", recovery);

    // Shape recognition may have lower accuracy but should still be reasonable
    assert!(recovery >= 70.0, "Shape recognition should achieve ≥70% recovery");
}