//! Per-page readability aggregation (Phase 4.7).
//!
//! This module implements the char-weighted median aggregation of per-span
//! readability scores into a single page-level score, plus the per-span
//! readability scoring function that computes individual span scores.
//!
//! # Per-Span Readability Scoring
//!
//! Each span receives a composite readability score in [0.0, 1.0] based on
//! five weighted signals:
//!
//! | Signal | Weight | Description |
//! |--------|--------|-------------|
//! | Printable fraction | 0.35 | Ratio of non-U+FFFD, non-control chars to total |
//! | Dictionary coverage | 0.30 | Ratio of words in 20k English wordlist (disabled for non-English) |
//! | Whitespace score | 0.15 | Binary: 1.0 if whitespace ratio in [0.05, 0.40] |
//! | Ligature integrity | 0.10 | Binary: 1.0 if no split ligatures detected |
//! | Confidence floor | 0.10 | Scaled: min(1.0, span.confidence / 0.6) |
//!
//! # Page-Level Aggregation
//!
//! Per-page readability is computed as the **median** of per-span scores,
//! **weighted by character count**. Longer spans contribute more to the
//! median than shorter spans.
//!
//! # Formula
//!
//! 1. Collect `(score, char_count)` pairs for all spans
//! 2. Sort by score ascending
//! 3. Compute cumulative character count
//! 4. Return the score at the half-total-char-count point
//!
//! # Edge Cases
//!
//! - Empty page (no spans): returns 0.0
//! - Single span: returns its score
//! - All spans have same score: returns that score

use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;
use crate::layout::wordlist::is_english_word;

/// Readability signal weights (sum to 1.0).
///
/// Per plan Phase 4.7 (lines 1765-1773), these weights are calibrated
/// against the test corpus to optimize the signal-to-noise ratio of the
/// composite score.
const READABILITY_WEIGHTS: [f32; 5] = [0.35, 0.30, 0.15, 0.10, 0.10];

/// Index positions for each signal in the READABILITY_WEIGHTS array.
const WEIGHT_PRINTABLE: usize = 0;
const WEIGHT_DICT_COVERAGE: usize = 1;
const WEIGHT_WHITESPACE: usize = 2;
const WEIGHT_LIGATURE: usize = 3;
const WEIGHT_CONFIDENCE: usize = 4;

/// Confidence threshold for the confidence_floor signal.
///
/// Spans with confidence >= 0.6 receive a score of 1.0 for this signal;
/// lower confidence spans are scaled proportionally.
const CONFIDENCE_THRESHOLD: f32 = 0.6;

/// Whitespace ratio bounds for the whitespace_score signal.
///
/// Whitespace ratio in [WHITESPACE_MIN, WHITESPACE_MAX] yields 1.0;
/// outside this range yields 0.0.
const WHITESPACE_MIN: f32 = 0.05;
const WHITESPACE_MAX: f32 = 0.40;

/// Ligature patterns to detect for ligature_integrity signal.
///
/// These patterns represent split ligatures where the ligature was not
/// properly reconstructed in Phase 2. Each pattern is (before, after) where
/// the presence of this sequence indicates a split ligature.
const SPLIT_LIGATURE_PATTERNS: &[(&str, &str)] = &[
    ("f", "\u{FFFD}i"), // f + U+FFFD + i (should be "fi")
    ("f", "\u{FFFD}l"), // f + U+FFFD + l (should be "fl")
    ("ff", "\u{FFFD}i"), // ff + U+FFFD + i (should be "ffi")
    ("ff", "\u{FFFD}l"), // ff + U+FFFD + l (should be "ffl")
    ("fi", "\u{FFFD}"),  // fi + U+FFFD (partial ligature)
    ("fl", "\u{FFFD}"),  // fl + U+FFFD (partial ligature)
];

/// Compute the printable fraction signal for a span.
///
/// Returns the ratio of non-U+FFFD, non-control characters to total characters.
/// Values close to 1.0 indicate clean text; values near 0.0 indicate severe
/// encoding issues.
fn printable_fraction(text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }

    let total_chars = text.chars().count();
    let printable_chars = text
        .chars()
        .filter(|&c| c != '\u{FFFD}' && !c.is_control())
        .count();

    printable_chars as f32 / total_chars as f32
}

/// Compute the dictionary coverage signal for a span.
///
/// Returns the ratio of words found in the 20k English wordlist to total words.
/// Uses unicode-segmentation UAX #29 word boundary splitting.
///
/// For non-empty text with no words, returns 0.0.
fn dict_coverage(text: &str, enabled: bool) -> f32 {
    if !enabled {
        return 1.0;
    }

    if text.is_empty() {
        return 1.0;
    }

    let words: Vec<&str> = text.unicode_words().collect();
    if words.is_empty() {
        return 0.0;
    }

    let dict_words = words.iter().filter(|w| is_english_word(w)).count();
    dict_words as f32 / words.len() as f32
}

/// Compute the whitespace score signal for a span.
///
/// Returns 1.0 if the whitespace ratio is in [0.05, 0.40], else 0.0.
/// This is a binary signal that penalizes spans with too little or too much
/// whitespace (indicates garbage data or formatting issues).
fn whitespace_score(text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }

    let total_chars = text.chars().count() as f32;
    let whitespace_chars = text.chars().filter(|c| c.is_whitespace()).count() as f32;
    let ratio = whitespace_chars / total_chars;

    if (WHITESPACE_MIN..=WHITESPACE_MAX).contains(&ratio) {
        1.0
    } else {
        0.0
    }
}

/// Compute the ligature integrity signal for a span.
///
/// Returns 1.0 if no split ligature patterns are detected, else 0.0.
/// Checks for patterns like "f<U+FFFD>i" which indicate a ligature was
/// not properly reconstructed in Phase 2.
fn ligature_integrity(text: &str) -> f32 {
    for &(before, after) in SPLIT_LIGATURE_PATTERNS {
        let combined = format!("{}{}", before, after);
        if text.contains(&combined) {
            return 0.0;
        }
    }
    1.0
}

/// Compute the confidence floor signal for a span.
///
/// Returns min(1.0, confidence / 0.6). Spans with confidence >= 0.6 receive
/// a score of 1.0 for this signal; lower confidence spans are scaled
/// proportionally.
fn confidence_floor(confidence: f32) -> f32 {
    (confidence / CONFIDENCE_THRESHOLD).min(1.0).max(0.0)
}

/// Compute the composite readability score for a span.
///
/// Returns a score in [0.0, 1.0] based on five weighted signals:
/// - 0.35 * printable_fraction
/// - 0.30 * dict_coverage (disabled for non-English documents, enabled for None/English)
/// - 0.15 * whitespace_score
/// - 0.10 * ligature_integrity
/// - 0.10 * confidence_floor
///
/// # Arguments
///
/// * `text` - The span's text content
/// * `confidence` - The span's minimum glyph confidence [0.0, 1.0]
/// * `document_lang` - The document's language tag (e.g., "en", "zh"), or None
///
/// # Returns
///
/// Composite readability score in [0.0, 1.0].
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::readability::score_span_readability;
///
/// // All-printable English text with high confidence
/// let score = score_span_readability("The quick brown fox", 1.0, Some("en"));
/// assert!(score > 0.9);
///
/// // All-U+FFFD replacement characters
/// let score = score_span_readability("\u{FFFD}\u{FFFD}\u{FFFD}", 1.0, Some("en"));
/// assert!(score < 0.25);
///
/// // Non-English: dictionary coverage disabled
/// let score = score_span_readability("中文文本", 1.0, Some("zh"));
/// assert!(score > 0.5);
/// ```
pub fn score_span_readability(text: &str, confidence: f32, document_lang: Option<&str>) -> f32 {
    if text.is_empty() {
        return 0.0;
    }

    // Dict coverage is enabled ONLY for English (en, en-US, en-GB, etc.)
    // None and non-English languages disable dict coverage
    let dict_enabled = document_lang
        .map(|lang| lang.starts_with("en"))
        .unwrap_or(false);

    // Compute each signal
    let print_sig = printable_fraction(text);
    let dict_sig = dict_coverage(text, dict_enabled);
    let white_sig = whitespace_score(text);
    let lig_sig = ligature_integrity(text);
    let conf_sig = confidence_floor(confidence);

    // Weighted sum
    let composite = READABILITY_WEIGHTS[WEIGHT_PRINTABLE] * print_sig
        + READABILITY_WEIGHTS[WEIGHT_DICT_COVERAGE] * dict_sig
        + READABILITY_WEIGHTS[WEIGHT_WHITESPACE] * white_sig
        + READABILITY_WEIGHTS[WEIGHT_LIGATURE] * lig_sig
        + READABILITY_WEIGHTS[WEIGHT_CONFIDENCE] * conf_sig;

    // Clamp to [0.0, 1.0] and return
    composite.clamp(0.0, 1.0)
}

/// A span with a readability score.
///
/// This trait abstracts over different span representations (internal Span
/// from hybrid.rs, SpanJson from schema, etc.) to allow the aggregation
/// function to work with any span type that has text and a score.
pub trait ScoredSpan {
    /// Get the text content of this span.
    fn text(&self) -> Cow<str>;

    /// Get the readability score for this span [0.0, 1.0].
    ///
    /// Returns None if the span has no score (should be excluded from aggregation).
    fn score(&self) -> Option<f32>;
}

/// Aggregate per-span readability scores into a page-level score.
///
/// Computes the **char-weighted median** of span scores:
/// - Sort spans by score ascending
/// - Accumulate character counts
/// - Return the score at the half-total-char point
///
/// # Arguments
///
/// * `spans` - Slice of spans with text and readability scores
///
/// # Returns
///
/// Page-level readability score in [0.0, 1.0], or 0.0 for empty pages.
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::readability::{aggregate_page_readability, TestSpan};
///
/// // Single span: page score = span score
/// let spans = vec![TestSpan::new("Test", 0.9)];
/// assert_eq!(aggregate_page_readability(&spans), 0.9);
///
/// // Char-weighted median: longer spans count more
/// let spans = vec![
///     TestSpan::new("a".repeat(100), 0.9),  // 100 chars
///     TestSpan::new("b".repeat(10), 0.5),   // 10 chars
///     TestSpan::new("c".repeat(100), 0.8),  // 100 chars
/// ];
/// // Sorted: 0.5(10), 0.8(100), 0.9(100)
/// // Cumsum: 10, 110, 210
/// // Half = 105 -> score at cumsum >= 105 is 0.8
/// assert_eq!(aggregate_page_readability(&spans), 0.8);
/// ```
pub fn aggregate_page_readability<T: ScoredSpan>(spans: &[T]) -> f32 {
    // Collect (score, char_count) pairs, excluding spans with no score
    let mut pairs: Vec<(f32, usize)> = spans
        .iter()
        .filter_map(|span| {
            let score = span.score()?;
            let char_count = span.text().chars().count();
            Some((score, char_count))
        })
        .collect();

    // Edge case: empty page or no scored spans
    if pairs.is_empty() {
        return 0.0;
    }

    // Edge case: single span
    if pairs.len() == 1 {
        return pairs[0].0;
    }

    // Sort by score ascending
    pairs.sort_by_key(|&(score, _)| {
        // Sort f32 with total ordering: handle NaN by treating as +infinity
        score.to_bits()
    });

    // Compute total character count
    let total_chars: usize = pairs.iter().map(|&(_, count)| count).sum();

    // Edge case: all empty strings (total_chars = 0)
    if total_chars == 0 {
        return 0.0;
    }

    // Find the score at the half-total-char point
    let half_chars = total_chars / 2;
    let mut cumulative = 0;

    for (score, count) in &pairs {
        cumulative += count;
        if cumulative > half_chars {
            return *score;
        }
    }

    // Fallback: return the highest score (should not reach here with valid data)
    pairs.last().map(|&(score, _)| score).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    /// Test span implementation.
    #[derive(Debug, Clone)]
    struct TestSpan {
        text: String,
        score: Option<f32>,
    }

    impl TestSpan {
        fn new(text: impl Into<String>, score: f32) -> Self {
            Self {
                text: text.into(),
                score: Some(score),
            }
        }

        fn without_score(text: impl Into<String>) -> Self {
            Self {
                text: text.into(),
                score: None,
            }
        }
    }

    impl ScoredSpan for TestSpan {
        fn text(&self) -> Cow<'_, str> {
            Cow::Borrowed(&self.text)
        }

        fn score(&self) -> Option<f32> {
            self.score
        }
    }

    #[test]
    fn test_single_span() {
        let spans = vec![TestSpan::new("Test", 0.9)];
        assert_eq!(aggregate_page_readability(&spans), 0.9);
    }

    #[test]
    fn test_empty_page() {
        let spans: Vec<TestSpan> = vec![];
        assert_eq!(aggregate_page_readability(&spans), 0.0);
    }

    #[test]
    fn test_all_unscored_spans() {
        let spans = vec![
            TestSpan::without_score("text1"),
            TestSpan::without_score("text2"),
        ];
        assert_eq!(aggregate_page_readability(&spans), 0.0);
    }

    #[test]
    fn test_mixed_scored_unscored() {
        let spans = vec![
            TestSpan::new("scored", 0.8),
            TestSpan::without_score("ignored"),
        ];
        assert_eq!(aggregate_page_readability(&spans), 0.8);
    }

    #[test]
    fn test_char_weighted_median_example() {
        // From acceptance criteria:
        // (100 chars, 0.9), (10 chars, 0.5), (100 chars, 0.8)
        // Sorted by score: 0.5(10), 0.8(100), 0.9(100)
        // Cumsum: 10, 110, 210
        // Half = 210 / 2 = 105
        // Score at cumsum >= 105 is 0.8
        let spans = vec![
            TestSpan::new("a".repeat(100), 0.9),
            TestSpan::new("b".repeat(10), 0.5),
            TestSpan::new("c".repeat(100), 0.8),
        ];
        assert_eq!(aggregate_page_readability(&spans), 0.8);
    }

    #[test]
    fn test_char_weighted_median_even_split() {
        // Two equal spans: median is the higher score (half point at boundary)
        let spans = vec![
            TestSpan::new("a".repeat(100), 0.5),
            TestSpan::new("b".repeat(100), 0.9),
        ];
        // Total = 200, half = 100
        // Cumsum after first span = 100, not > 100
        // Cumsum after second span = 200 > 100
        // Returns 0.9
        assert_eq!(aggregate_page_readability(&spans), 0.9);
    }

    #[test]
    fn test_all_same_score() {
        let spans = vec![
            TestSpan::new("a", 0.8),
            TestSpan::new("b", 0.8),
            TestSpan::new("c", 0.8),
        ];
        assert_eq!(aggregate_page_readability(&spans), 0.8);
    }

    #[test]
    fn test_empty_strings() {
        let spans = vec![TestSpan::new("", 0.5), TestSpan::new("", 0.8)];
        // All empty -> total_chars = 0 -> return 0.0
        assert_eq!(aggregate_page_readability(&spans), 0.0);
    }

    #[test]
    fn test_unicode_char_count() {
        // Test that char_count counts Unicode code points, not bytes
        let spans = vec![
            TestSpan::new("é", 0.9),  // 2 bytes, 1 char
            TestSpan::new("中", 0.8), // 3 bytes, 1 char
        ];
        // Each span is 1 char, total = 2, half = 1
        // Sorted by score: (0.8, 1), (0.9, 1)
        // Cumsum after first = 1, not > 1
        // Cumsum after second = 2 > 1
        // Returns second score (0.9) after sorting
        assert_eq!(aggregate_page_readability(&spans), 0.9);
    }

    #[test]
    fn test_longer_span_dominates() {
        // One very long span dominates the median
        let spans = vec![
            TestSpan::new("x".repeat(1000), 0.9),
            TestSpan::new("y".repeat(10), 0.1),
            TestSpan::new("z".repeat(10), 0.2),
        ];
        // Total = 1020, half = 510
        // Cumsum: 10 (0.1), 20 (0.2), 1020 (0.9)
        // 1020 > 510, returns 0.9
        assert_eq!(aggregate_page_readability(&spans), 0.9);
    }

    #[test]
    fn test_all_perfect_scores() {
        let spans = vec![
            TestSpan::new("a".repeat(100), 1.0),
            TestSpan::new("b".repeat(100), 1.0),
        ];
        assert_eq!(aggregate_page_readability(&spans), 1.0);
    }

    #[test]
    fn test_all_zero_scores() {
        let spans = vec![TestSpan::new("a", 0.0), TestSpan::new("b", 0.0)];
        assert_eq!(aggregate_page_readability(&spans), 0.0);
    }

    #[test]
    fn test_order_preservation() {
        // Verify that sort order doesn't affect result
        let spans1 = vec![
            TestSpan::new("a".repeat(100), 0.9),
            TestSpan::new("b".repeat(10), 0.5),
            TestSpan::new("c".repeat(100), 0.8),
        ];

        let spans2 = vec![
            TestSpan::new("c".repeat(100), 0.8),
            TestSpan::new("a".repeat(100), 0.9),
            TestSpan::new("b".repeat(10), 0.5),
        ];

        assert_eq!(
            aggregate_page_readability(&spans1),
            aggregate_page_readability(&spans2)
        );
    }

    #[test]
    fn test_nan_score_handling() {
        // NaN scores should be sorted to the end (due to to_bits() ordering)
        let spans = vec![
            TestSpan::new("a".repeat(10), 0.5),
            TestSpan::new("b".repeat(10), f32::NAN),
            TestSpan::new("c".repeat(10), 0.8),
        ];
        // Total = 30, half = 15
        // Sorted: 0.5(10), 0.8(10), NaN(10)
        // Cumsum: 10, 20, 30
        // 20 > 15, returns 0.8
        let result = aggregate_page_readability(&spans);
        assert!(result.is_finite());
        assert_eq!(result, 0.8);
    }

    #[test]
    fn test_zero_width_joiner() {
        // Test zero-width joiner and combining marks
        let spans = vec![
            TestSpan::new("café", 0.9), // 4 chars: c a f é
            TestSpan::new("नमस्ते", 0.8), // 6 chars (Hindi namaste)
        ];
        // Total = 10 chars, half = 5
        // Cumsum after first = 4, not > 5
        // Cumsum after second = 10 > 5
        // Returns second score
        assert_eq!(aggregate_page_readability(&spans), 0.8);
    }

    // Tests for score_span_readability acceptance criteria (pdftract-1q4ku)

    #[test]
    fn test_all_printable_english_high_coverage() {
        // AC1: All-printable English high coverage: > 0.9
        let text = "The quick brown fox jumps over the lazy dog";
        let score = score_span_readability(text, 1.0, Some("en"));
        assert!(score > 0.9, "Expected high score for clean English, got {}", score);
    }

    #[test]
    fn test_all_replacement_chars() {
        // AC2: All-U+FFFD: significantly reduced (printable_fraction=0, whitespace_score=0)
        // Score = 0.35*0 + 0.30*0 + 0.15*0 + 0.10*1 + 0.10*1 = 0.2
        // (dict_coverage=0 because U+FFFD sequences are not English words)
        let text = "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}";
        let score = score_span_readability(text, 1.0, Some("en"));
        assert!(score < 0.7, "Expected reduced score for all U+FFFD, got {}", score);
        assert!(score > 0.1, "Score should still be >0 due to lig/conf signals");
    }

    #[test]
    fn test_all_whitespace() {
        // AC3: All-whitespace: whitespace_score=0 (binary fail)
        // The key is that whitespace_score signal is 0.0, not that total score is low
        let text = "     \n\t\r\n     ";
        let white_sig = whitespace_score(text);
        assert_eq!(white_sig, 0.0, "whitespace_score should be 0.0 for all-whitespace");
        // Total score may still be decent due to other signals
        let score = score_span_readability(text, 1.0, Some("en"));
        assert!(score < 1.0, "Score should be reduced due to whitespace penalty");
    }

    #[test]
    fn test_low_confidence_scaling() {
        // AC4: Single short low-confidence word: scaled by confidence_floor
        let text = "test";
        let score_high = score_span_readability(text, 1.0, Some("en"));
        let score_low = score_span_readability(text, 0.3, Some("en"));
        assert!(score_low < score_high, "Low confidence should lower score");
        // 0.3 confidence -> 0.3/0.6 = 0.5 confidence_floor
        // Confidence weight is 0.10, so max reduction is 0.10 * 0.5 = 0.05
        let diff = score_high - score_low;
        assert!(diff > 0.0 && diff < 0.10, "Confidence penalty should be small, got {}", diff);
    }

    #[test]
    fn test_non_english_dict_disabled() {
        // AC5: Non-English doc: dict forced 1.0; score from other signals
        let text = "This is clean text with good characters";
        let score_en = score_span_readability(text, 1.0, Some("en"));
        let score_zh = score_span_readability(text, 1.0, Some("zh"));
        // Non-English should have different score since dict_coverage is disabled
        // Dict weight is 0.30, so max difference is ~0.30 (all other signals equal)
        let diff = (score_en - score_zh).abs();
        assert!(diff <= 0.31, "Dict weight is 0.30, max diff should be ~0.30, got {}", diff);
        // Both should still be decent (printable, whitespace, ligature, confidence all good)
        assert!(score_zh > 0.6, "Non-English with clean text should still score well");
    }

    #[test]
    fn test_ligature_split_penalty() {
        // AC6: Ligature-split span: integrity 0 lowers score
        // Use exact split ligature pattern: "f" + U+FFFD + "i"
        let clean_text = "The first line";
        let split_ligature = "The f\u{FFFD}i line"; // Exact pattern: f + U+FFFD + i

        let score_clean = score_span_readability(clean_text, 1.0, Some("en"));
        let score_split = score_span_readability(split_ligature, 1.0, Some("en"));

        assert!(score_split < score_clean, "Split ligature should lower score");
        // Ligature integrity weight is 0.10, plus some printable_fraction effect
        let diff = score_clean - score_split;
        assert!(diff >= 0.09, "Ligature penalty should be at least 0.09, got {}", diff);
    }

    #[test]
    fn test_empty_span_returns_zero() {
        // Edge case: Empty span should return 0.0
        let score = score_span_readability("", 1.0, Some("en"));
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_confidence_threshold() {
        // Confidence threshold test: 0.6 confidence -> 1.0 confidence_floor
        let text = "The quick brown fox";
        let score_060 = score_span_readability(text, 0.6, Some("en"));
        let score_100 = score_span_readability(text, 1.0, Some("en"));
        // Both should be same (confidence_floor is 1.0 at 0.6+)
        assert_eq!(score_060, score_100);
    }

    #[test]
    fn test_whitespace_bounds() {
        // Test whitespace ratio boundaries
        // 5% whitespace -> score 1.0
        let text_05 = "aaaaa b"; // 6 chars, 1 space = 0.167 ratio (in bounds)
        assert_eq!(whitespace_score(text_05), 1.0);

        // 0% whitespace -> score 0.0
        let text_00 = "aaaaab";
        assert_eq!(whitespace_score(text_00), 0.0);
    }

    #[test]
    fn test_printable_fraction_perfect() {
        // All printable -> 1.0
        let text = "Hello World 123";
        assert_eq!(printable_fraction(text), 1.0);
    }

    #[test]
    fn test_dict_coverage_disabled_non_english() {
        // Dict coverage disabled for non-English returns 1.0
        let text = "xyzzy plugh"; // Non-words
        assert_eq!(dict_coverage(text, false), 1.0);
        assert!(dict_coverage(text, true) < 1.0); // Enabled should be < 1.0
    }

    #[test]
    fn test_non_english_enables_dict_only_for_en() {
        // Verify dict coverage is enabled ONLY for "en" prefix
        // Use text with non-dictionary words to show the difference
        let text = "xyzzy plugh";  // Non-words not in the 20k wordlist
        let score_en = score_span_readability(text, 1.0, Some("en"));
        let score_en_us = score_span_readability(text, 1.0, Some("en-US"));
        let score_zh = score_span_readability(text, 1.0, Some("zh"));
        let score_none = score_span_readability(text, 1.0, None);

        // English variants should have same score (dict enabled, both words fail -> lower score)
        assert_eq!(score_en, score_en_us, "en and en-US should have same score");
        // Non-English and None should have same score (dict disabled -> higher score)
        assert_eq!(score_zh, score_none, "Non-English and None should have same score");
        // English should be DIFFERENT from non-English (dict enabled for en, disabled for zh)
        // For "xyzzy plugh", dict_coverage=0 for en (words not in dict), but 1.0 for zh (disabled)
        // Dict weight is 0.30, so max difference is 0.30
        assert_ne!(score_en, score_zh, "English and non-English should differ due to dict");
        // Verify non-English score is higher (dict disabled gives 1.0 vs 0.0 for en)
        assert!(score_zh > score_en, "Non-English should have higher score when words not in dict");
    }
}
