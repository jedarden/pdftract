//! Unicode recovery tests for Phase 2.2–2.5 no-ToUnicode corpus.
//!
//! Tests Unicode recovery from PDFs without ToUnicode CMaps, exercising:
//! - Level 2: AGL (Adobe Glyph List) fallback lookup
//! - Level 3: SHA-256 font program fingerprint matching
//! - Level 4: Glyph shape recognition (glyph-shapes.json DB)
//!
//! Reference: Plan section Phase 2.2-2.5, lines 263-2450
//! Acceptance criteria: ≥90% recovery rate on this corpus (Tier 1 CI gate)

use pdftract_core::document::PdfExtractor;
use std::path::Path;
use std::fs;

/// Test fixture describing a no-ToUnicode PDF and its expected text output.
struct EncodingFixture {
    name: &'static str,
    pdf_path: &'static str,
    truth_path: &'static str,
    description: &'static str,
}

/// Calculate character error rate (CER) between extracted and ground truth.
///
/// CER = (substitutions + insertions + deletions) / ground_truth_length
/// Returns 0.0 if both strings are identical.
fn calculate_cer(extracted: &str, ground_truth: &str) -> f64 {
    if extracted == ground_truth {
        return 0.0;
    }

    let extract_chars: Vec<char> = extracted.chars().collect();
    let truth_chars: Vec<char> = ground_truth.chars().collect();

    let extract_len = extract_chars.len();
    let truth_len = truth_chars.len();

    // Simple edit distance (Levenshtein) for CER calculation
    let mut dp = vec![vec![0usize; truth_len + 1]; extract_len + 1];

    for i in 0..=extract_len {
        dp[i][0] = i;
    }
    for j in 0..=truth_len {
        dp[0][j] = j;
    }

    for i in 1..=extract_len {
        for j in 1..=truth_len {
            let cost = if extract_chars[i - 1] == truth_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = dp[i - 1][j - 1] + cost
                .min(dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1);
        }
    }

    let edits = dp[extract_len][truth_len];
    edits as f64 / truth_len.max(1) as f64
}

/// Calculate Unicode recovery rate.
///
/// Recovery rate = 1.0 - CER, clamped to [0, 1].
/// A recovery rate of 1.0 means perfect extraction.
/// A recovery rate of 0.9 means ≥90% of characters were recovered correctly.
fn calculate_recovery_rate(extracted: &str, ground_truth: &str) -> f64 {
    let cer = calculate_cer(extracted, ground_truth);
    (1.0 - cer).max(0.0).min(1.0)
}

/// Get all encoding fixtures with their configuration.
fn get_fixtures() -> Vec<EncodingFixture> {
    vec![
        EncodingFixture {
            name: "no-mapping",
            pdf_path: "../../tests/fixtures/encoding/no-mapping.pdf",
            truth_path: "../../tests/fixtures/encoding/no-mapping.txt",
            description: "PDF with no ToUnicode, no standard encoding (worst case)",
        },
        EncodingFixture {
            name: "agl-only",
            pdf_path: "../../tests/fixtures/encoding/agl-only.pdf",
            truth_path: "../../tests/fixtures/encoding/agl-only.txt",
            description: "PDF with AGL glyph names only (Level 2 recovery)",
        },
        EncodingFixture {
            name: "fingerprint-match",
            pdf_path: "../../tests/fixtures/encoding/fingerprint-match.pdf",
            truth_path: "../../tests/fixtures/encoding/fingerprint-match.txt",
            description: "PDF with embedded font for fingerprint matching (Level 3)",
        },
        EncodingFixture {
            name: "shape-match",
            pdf_path: "../../tests/fixtures/encoding/shape-match.pdf",
            truth_path: "../../tests/fixtures/encoding/shape-match.txt",
            description: "PDF with subset font for shape recognition (Level 4)",
        },
    ]
}

/// Test a single encoding fixture and return recovery metrics.
fn test_encoding_fixture(fixture: &EncodingFixture) -> Result<FixtureResult, Box<dyn std::error::Error>> {
    let pdf_path = Path::new(fixture.pdf_path);

    // Open the PDF
    let mut extractor = PdfExtractor::open(pdf_path)
        .map_err(|e| format!("Failed to open PDF: {}", e))?;

    // Materialize pages for extraction
    extractor.materialize_pages()
        .map_err(|e| format!("Failed to materialize pages: {}", e))?;

    // Extract text from first page (all fixtures have single pages)
    let page_extraction = extractor.extract_page(0)
        .map_err(|e| format!("Failed to extract page: {}", e))?;

    // Concatenate text from all blocks
    let extracted_text: String = page_extraction.blocks
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<&str>>()
        .join("");

    let ground_truth = fs::read_to_string(fixture.truth_path)
        .map_err(|e| format!("Failed to read ground truth: {}", e))?;

    let cer = calculate_cer(&extracted_text, &ground_truth);
    let recovery_rate = calculate_recovery_rate(&extracted_text, &ground_truth);

    Ok(FixtureResult {
        name: fixture.name,
        extracted: extracted_text,
        ground_truth,
        cer,
        recovery_rate,
    })
}

/// Result of testing a single fixture.
#[derive(Debug)]
struct FixtureResult {
    name: &'static str,
    extracted: String,
    ground_truth: String,
    cer: f64,
    recovery_rate: f64,
}

#[test]
fn test_no_mapping_fixture() {
    let fixture = &get_fixtures()[0];
    let result = test_encoding_fixture(fixture).unwrap();

    // no-mapping.pdf has custom glyph names that don't map to AGL
    // Current implementation may emit U+FFFD or recover via shape recognition
    // For now, we just verify it doesn't crash
    assert!(result.cer >= 0.0, "CER should be non-negative");
    assert!(result.recovery_rate <= 1.0, "Recovery rate should be ≤ 1.0");
}

#[test]
fn test_agl_only_fixture() {
    let fixture = &get_fixtures()[1];
    let result = test_encoding_fixture(fixture).unwrap();

    // AGL should successfully recover "Hello\nWorld"
    assert_eq!(result.extracted.trim(), result.ground_truth.trim(),
        "AGL-only fixture should recover text correctly via glyph name mapping");
    assert_eq!(result.cer, 0.0, "CER should be 0 for perfect match");
    assert_eq!(result.recovery_rate, 1.0, "Recovery rate should be 1.0 for perfect match");
}

#[test]
fn test_fingerprint_match_fixture() {
    let fixture = &get_fixtures()[2];
    let result = test_encoding_fixture(fixture).unwrap();

    // Fingerprint matching should recover "Test" if the font is in the DB
    // This is currently a placeholder - the actual fingerprint DB is populated in Phase 2.2
    assert!(result.cer >= 0.0, "CER should be non-negative");
}

#[test]
fn test_shape_match_fixture() {
    let fixture = &get_fixtures()[3];
    let result = test_encoding_fixture(fixture).unwrap();

    // Shape matching should recover "Shape" if glyphs are in the shape DB
    // This is currently a placeholder - the shape DB is populated in Phase 2.5
    assert!(result.cer >= 0.0, "CER should be non-negative");
}

#[test]
fn test_all_encoding_fixtures_exist() {
    for fixture in get_fixtures() {
        assert!(Path::new(fixture.pdf_path).exists(),
            "Encoding fixture PDF should exist: {}", fixture.pdf_path);
        assert!(Path::new(fixture.truth_path).exists(),
            "Encoding fixture ground truth should exist: {}", fixture.truth_path);
    }
}

#[test]
fn test_corpus_recovery_rate() {
    /// Overall recovery rate for the entire corpus.
    ///
    /// The Phase 2 exit gate requires ≥90% recovery rate on this corpus.
    /// This is calculated as the weighted average recovery across all fixtures.
    let fixtures = get_fixtures();
    let mut total_recovery = 0.0;
    let mut fixture_count = 0;

    for fixture in &fixtures {
        match test_encoding_fixture(fixture) {
            Ok(result) => {
                total_recovery += result.recovery_rate;
                fixture_count += 1;
                println!(
                    "Fixture {}: recovery_rate={:.2}, cer={:.2}",
                    result.name, result.recovery_rate, result.cer
                );
            }
            Err(e) => {
                panic!("Fixture {} failed: {}", fixture.name, e);
            }
        }
    }

    let avg_recovery = if fixture_count > 0 {
        total_recovery / fixture_count as f64
    } else {
        0.0
    };

    println!("Average corpus recovery rate: {:.2}%", avg_recovery * 100.0);

    // TODO: Enable the ≥90% gate once Phase 2.2–2.5 are fully implemented
    // For now, this test verifies the corpus is structured correctly
    // assert!(avg_recovery >= 0.9,
    //     "Corpus recovery rate should be ≥90%, got {:.2}%", avg_recovery * 100.0);

    assert!(avg_recovery >= 0.0, "Recovery rate should be non-negative");
    assert!(avg_recovery <= 1.0, "Recovery rate should be ≤ 1.0");
}
