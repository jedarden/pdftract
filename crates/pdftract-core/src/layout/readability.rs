//! Per-page readability aggregation (Phase 4.7).
//!
//! This module implements the char-weighted median aggregation of per-span
//! readability scores into a single page-level score.
//!
//! # Algorithm
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
        fn text(&self) -> Cow<str> {
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
}
