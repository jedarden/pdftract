//! Text correction pipeline (Phase 4.7).
//!
//! This module implements the correction pipeline applied to extracted text
//! before readability scoring. Corrections include:
//! - Mojibake detection and repair (Latin-1 interpreted as UTF-8)
//!
//! # Mojibake Detection
//!
//! Mojibake occurs when UTF-8 text is incorrectly produced from Latin-1 bytes,
//! resulting in garbled output like "cafÃ©" instead of "café". This module
//! detects such patterns and attempts to recover the original text by
//! re-decoding the bytes as windows-1252.

use encoding_rs::WINDOWS_1252;

/// Trait for types with mutable text content that can be corrected.
///
/// This trait abstracts over different span representations to allow
/// the correction pipeline to work with any span type that has text.
pub trait CorrectableText {
    /// Get a mutable reference to the text content.
    fn text_mut(&mut self) -> &mut String;

    /// Get the text content immutably.
    fn text(&self) -> &str;
}

/// Detect and repair mojibake in span text.
///
/// Scans the span's text for sequences characteristic of Latin-1 bytes interpreted
/// as UTF-8 (e.g., `Ã©` for `é`, `â€™` for `'`). If detected, attempts to
/// re-decode via `encoding_rs` (treat the bytes as windows-1252/Latin-1) and
/// accepts the re-decoded text if the scorer reports a higher readability score.
///
/// # Arguments
///
/// * `span` - Mutable reference to a span with text to check/repair
/// * `scorer` - Callback that computes a readability score for text [0.0, 1.0]
///
/// # Returns
///
/// `true` if the span text was replaced with re-decoded text, `false` otherwise.
///
/// # Detection Heuristic
///
/// Checks for at least 2 occurrences of any telltale 2-char sequences:
/// - `Ã©` `Ã¨` `Ã ` `Ã®` `Ã´` `Ã»` `Ã¢` `Ã§` `Ã±` (common French/Spanish chars)
/// - `â€™` `â€"` `â€œ` `â€` (smart quotes / em-dash from Windows-1252)
/// - `Â` followed by a non-ASCII char (NBSP and similar)
///
/// # Correction Process
///
/// 1. Encode the current text as UTF-8 bytes
/// 2. Decode those bytes as windows-1252 (the actual encoding)
/// 3. Score both original and candidate text
/// 4. If `candidate_score > original_score + 0.05`: accept the replacement
///
/// # Epsilon Threshold
///
/// The 0.05 epsilon prevents noise from triggering unnecessary re-decoding.
/// Only readability improvements greater than 5% are accepted.
///
/// # Invariants
///
/// - **INV**: Re-decoding is REVERTED if it doesn't improve readability (false-positive safety).
/// - **INV**: A clean ASCII or pure UTF-8 span (no Ã/â sequences) passes through unchanged.
/// - **INV**: The encoding is windows-1252, not pure Latin-1 (covers smart quotes and Microsoft-isms).
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::correction::{detect_and_repair_mojibake, TestCorrectable};
///
/// // Clean UTF-8 text: no detection
/// let mut span = TestCorrectable::new("café");
/// let repaired = detect_and_repair_mojibake(&mut span, |s| simple_score(s));
/// assert!(!repaired);
/// assert_eq!(span.text(), "café");
///
/// // Mojibake: detected and repaired
/// let mut span = TestCorrectable::new("cafÃ©");
/// let repaired = detect_and_repair_mojibake(&mut span, |s| {
///     // Mock scorer that prefers corrected text
///     if s.contains("Ã©") { 0.3 } else { 0.9 }
/// });
/// assert!(repaired);
/// assert_eq!(span.text(), "café");
/// ```
pub fn detect_and_repair_mojibake<T, F>(span: &mut T, scorer: F) -> bool
where
    T: CorrectableText,
    F: Fn(&str) -> f32,
{
    let text = span.text();

    // Fast-path: empty or ASCII-only text cannot be mojibake
    if text.is_empty() || text.is_ascii() {
        return false;
    }

    // Detection heuristic: check for telltale Latin-1-as-UTF-8 sequences
    if !contains_mojibake_indicators(text) {
        return false;
    }

    // Attempt re-decoding: encode as UTF-8, then decode as windows-1252
    let utf8_bytes = text.as_bytes();
    let (candidate, _) = WINDOWS_1252.decode_without_bom_handling(utf8_bytes);

    // Score both versions
    let original_score = scorer(text);
    let candidate_score = scorer(&candidate);

    // Accept replacement only if score improves by > epsilon
    const EPSILON: f32 = 0.05;
    if candidate_score > original_score + EPSILON {
        *span.text_mut() = candidate.to_string();
        true
    } else {
        false
    }
}

/// Check if text contains mojibake indicator sequences.
///
/// Returns true if at least 2 occurrences of any telltale 2-char patterns
/// are found. The threshold reduces false positives on legitimate text.
///
/// # Indicator Patterns
///
/// - `Ã©` `Ã¨` `Ãª` `Ã®` `Ã´` `Ã»` `Ã¢` `Ã§` `Ã±` - Latin-1 vowels with diacritics
/// - `â€™` `â€"` `â€œ` `â€` - Smart quotes and dashes from Windows-1252
/// - `Â` followed by non-ASCII - NBSP and related
fn contains_mojibake_indicators(text: &str) -> bool {
    const INDICATORS: &[&str] = &[
        // Latin-1 vowels with diacritics (common French/Spanish/Portuguese)
        "Ã©",
        "Ã¨",
        "Ãª",
        "Ã®",
        "Ã´",
        "Ã»",
        "Ã¢",
        "Ã§",
        "Ã±",
        "Ã£",
        "Ãº",
        "Ã\u{ad}",
        "Ã³",
        "Ã¡",
        // Smart quotes and dashes from Windows-1252
        "â€™",
        "â€\"",
        "â€œ",
        "â€",
        "â€\u{00a0}",
        "â€¡",
    ];

    let mut count = 0;
    let chars: Vec<char> = text.chars().collect();

    // Check for 2-char sequences
    for i in 0..chars.len().saturating_sub(1) {
        let pair: String = chars[i..=i + 1].iter().collect();
        if INDICATORS.contains(&pair.as_str()) {
            count += 1;
            if count >= 2 {
                return true;
            }
        }
    }

    // Check for Â followed by non-ASCII
    for i in 0..chars.len().saturating_sub(1) {
        if chars[i] == 'Â' && !chars[i + 1].is_ascii() {
            count += 1;
            if count >= 2 {
                return true;
            }
        }
    }

    false
}

/// Test implementation of `CorrectableText` for unit tests.
#[cfg(test)]
pub struct TestCorrectable {
    text: String,
}

#[cfg(test)]
impl TestCorrectable {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
impl CorrectableText for TestCorrectable {
    fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple mock scorer that returns 1.0 for clean text, 0.3 for mojibake.
    fn simple_scorer(text: &str) -> f32 {
        // Check for common mojibake patterns
        if text.contains("\u{00c3}\u{00a9}") || // Ã©
           text.contains("\u{00c3}\u{00a8}") || // Ã¨
           text.contains("\u{00e2}\u{20ac}\u{2122}")
        {
            // â€™ (smart quote)
            0.3
        } else {
            0.9
        }
    }

    #[test]
    fn test_clean_utf8_no_change() {
        // Clean UTF-8 text: no mojibake sequences
        let mut span = TestCorrectable::new("caf\u{00e9}");
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(!repaired);
        assert_eq!(span.text(), "caf\u{00e9}");
    }

    #[test]
    fn test_ascii_only_no_change() {
        // ASCII-only text: cannot be mojibake
        let mut span = TestCorrectable::new("hello world");
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(!repaired);
        assert_eq!(span.text(), "hello world");
    }

    #[test]
    fn test_empty_string_no_change() {
        let mut span = TestCorrectable::new("");
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(!repaired);
        assert_eq!(span.text(), "");
    }

    #[test]
    fn test_mojibake_detected_and_repaired() {
        // "cafÃ©" is mojibake for "café" - Latin-1 interpreted as UTF-8
        // In UTF-8, é is 0xC3 0xA9. If those bytes are interpreted as windows-1252,
        // we get "Ã©". Re-encoding those as UTF-8 bytes and decoding as windows-1252
        // should recover the original "é".
        let mut span = TestCorrectable::new("caf\u{00c3}\u{00a9}"); // cafÃ©
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(repaired);
        assert_eq!(span.text(), "caf\u{00e9}"); // café
    }

    #[test]
    fn test_mojibake_multiple_indicators() {
        // Multiple indicators: Ã©Ã¨ (café + è)
        let mut span =
            TestCorrectable::new("caf\u{00c3}\u{00a9} r\u{00c3}\u{00a8}st\u{00c3}\u{00a9}");
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(repaired);
        // Should re-decode to "café résté"
        assert_eq!(span.text(), "caf\u{00e9} r\u{00e9}st\u{00e9}");
    }

    #[test]
    fn test_mojibake_single_indicator_threshold() {
        // Single Ã© without other indicators: below threshold
        let mut span = TestCorrectable::new("caf\u{00c3}\u{00a9}sandbar");
        // With only 1 Ã©, the threshold of 2 is not met
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(!repaired); // Should not detect with only 1 indicator
        assert_eq!(span.text(), "caf\u{00c3}\u{00a9}sandbar");
    }

    #[test]
    fn test_smart_quote_mojibake() {
        // Smart quote mojibake
        let mojibake = "don\u{2019}t"; // don't with curly apostrophe
        let mut span = TestCorrectable::new(mojibake);
        let repaired =
            detect_and_repair_mojibake(
                &mut span,
                |s| {
                    if s.contains("\u{2019}") {
                        0.3
                    } else {
                        0.9
                    }
                },
            );
        assert!(repaired);
        assert_eq!(span.text(), "don't");
    }

    #[test]
    fn test_em_dash_mojibake() {
        // em dash mojibake test
        let mojibake = "hello\u{2014}world"; // â€" pattern
        let mut span = TestCorrectable::new(mojibake);
        let repaired =
            detect_and_repair_mojibake(
                &mut span,
                |s| {
                    if s.contains("\u{2014}") {
                        0.3
                    } else {
                        0.9
                    }
                },
            );
        assert!(repaired);
        // Should decode to proper em dash
        assert!(span.text().contains("\u{2014}"));
    }

    #[test]
    fn test_replacement_rejected_if_score_doesnt_improve() {
        // Even with mojibake indicators, don't replace if score doesn't improve
        let mut span = TestCorrectable::new("caf\u{00c3}\u{00a9}");
        let repaired = detect_and_repair_mojibake(&mut span, |_| 0.5); // Both score 0.5
                                                                       // No replacement because candidate_score (0.5) is not > original_score (0.5) + 0.05
        assert!(!repaired);
        assert_eq!(span.text(), "caf\u{00c3}\u{00a9}");
    }

    #[test]
    fn test_epsilon_threshold_prevents_noise() {
        // Candidate score only slightly better - should be rejected
        let mut span = TestCorrectable::new("caf\u{00c3}\u{00a9}");
        let repaired = detect_and_repair_mojibake(&mut span, |s| {
            if s.contains("\u{00c3}\u{00a9}") {
                0.7
            } else {
                0.74
            } // Only 0.04 improvement
        });
        // 0.74 is not > 0.7 + 0.05 (0.75), so no replacement
        assert!(!repaired);
        assert_eq!(span.text(), "caf\u{00c3}\u{00a9}");
    }

    #[test]
    fn test_asian_text_unaffected() {
        // Asian text (no Latin-1 indicators): pass-through
        let mut span = TestCorrectable::new("こんにちは世界");
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(!repaired);
        assert_eq!(span.text(), "こんにちは世界");
    }

    #[test]
    fn test_windows1252_specific() {
        // Test that we use windows-1252, not pure Latin-1
        // Smart quote is the windows-1252 smart quote, not in pure Latin-1
        let mojibake = "it\u{2019}s"; // it's with smart quote
        let mut span = TestCorrectable::new(mojibake);
        let repaired =
            detect_and_repair_mojibake(
                &mut span,
                |s| {
                    if s.contains("\u{2019}") {
                        0.3
                    } else {
                        0.9
                    }
                },
            );
        assert!(repaired);
        assert_eq!(span.text(), "it's");
    }

    #[test]
    fn test_mixed_ascii_and_mojibake() {
        // Mixed content: some ASCII, some mojibake
        let mut span = TestCorrectable::new("The word is caf\u{00e9} and r\u{00e9}sum\u{00e9}");
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(repaired);
        assert_eq!(
            span.text(),
            "The word is caf\u{00e9} and r\u{00e9}sum\u{00e9}"
        );
    }

    #[test]
    fn test_nbsp_indicator() {
        // NBSP pattern: \u{00a0} followed by non-ASCII
        let mut span = TestCorrectable::new("hello\u{00a0} world\u{00a0} here");
        let repaired =
            detect_and_repair_mojibake(
                &mut span,
                |s| {
                    if s.contains("\u{00a0} ") {
                        0.3
                    } else {
                        0.9
                    }
                },
            );
        assert!(repaired);
        // NBSP + space should be handled
        assert!(!span.text().contains("\u{00a0} "));
    }

    #[test]
    fn test_multiple_mojibake_patterns() {
        // Multiple different indicators: curly quote + accent
        let mojibake = "don\u{2019}t drink caf\u{00e9}";
        let mut span = TestCorrectable::new(mojibake);
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(repaired);
        assert_eq!(span.text(), "don't drink caf\u{00e9}");
    }

    #[test]
    fn test_exact_epsilon_boundary() {
        // Test the exact epsilon boundary
        let mut span = TestCorrectable::new("caf\u{00c3}\u{00a9}");
        let repaired = detect_and_repair_mojibake(&mut span, |s| {
            if s.contains("\u{00c3}\u{00a9}") {
                0.70
            } else {
                0.75
            } // Exactly 0.05 improvement
        });
        // 0.75 is NOT > 0.70 + 0.05 (0.75), so no replacement (strict inequality)
        assert!(!repaired);
    }

    #[test]
    fn test_just_above_epsilon() {
        // Just above epsilon threshold
        let mut span = TestCorrectable::new("caf\u{00c3}\u{00a9}");
        let repaired = detect_and_repair_mojibake(&mut span, |s| {
            if s.contains("\u{00c3}\u{00a9}") {
                0.70
            } else {
                0.751
            } // 0.051 improvement
        });
        // 0.751 > 0.70 + 0.05 (0.75), so replacement happens
        assert!(repaired);
        assert_eq!(span.text(), "caf\u{00e9}");
    }
}
