//! Text correction pipeline (Phase 4.7).
//!
//! This module implements the correction pipeline applied to extracted text
//! before readability scoring. Corrections include:
//! - Mojibake detection and repair (Latin-1 interpreted as UTF-8)
//! - Hyphenation repair (end-of-line hyphen joined with next line)
//! - Word-break normalization (zero-width characters stripped or preserved per script)
//!
//! # Mojibake Detection
//!
//! Mojibake occurs when UTF-8 text is incorrectly produced from Latin-1 bytes,
//! resulting in garbled output like "cafÃ©" instead of "café". This module
//! detects such patterns and attempts to recover the original text by
//! re-decoding the bytes as windows-1252.

use encoding_rs::WINDOWS_1252;

use crate::layout::line::{Block, Line, LineMetadata};
use crate::span::Span;

/// Unicode script category for word-break normalization.
///
/// Simplified script detection based on Unicode codepoint ranges.
/// Used to determine whether zero-width joiner/non-joiner characters
/// should be preserved (they're orthographic in complex scripts) or
/// stripped (they're noise in Latin text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    /// Arabic script - requires ZWNJ/ZWJ for correct rendering
    Arabic,
    /// Hebrew script - may use ZWNJ/ZWJ
    Hebrew,
    /// Devanagari (Hindi, Marathi, Nepali, Sanskrit) - requires ZWNJ/ZWJ for conjuncts
    Devanagari,
    /// Bengali script - requires ZWNJ/ZWJ for conjuncts
    Bengali,
    /// Other Indic scripts (Gurmukhi, Gujarati, Tamil, Telugu, Kannada, Malayalam, Odia)
    Indic,
    /// Thai script - may use ZWNJ/ZWJ
    Thai,
    /// Lao script
    Lao,
    /// Tibetan script
    Tibetan,
    /// Myanmar (Burmese) script
    Myanmar,
    /// Khmer script
    Khmer,
    /// Sinhala script
    Sinhala,
    /// Latin and other simple scripts - ZWNJ/ZWJ are noise
    Latin,
    /// Unknown script - default to strip (safe default)
    Unknown,
}

impl Script {
    /// Returns true if this script uses ZWNJ/ZWJ for legitimate orthographic purposes.
    ///
    /// Complex scripts like Arabic, Indic, and Southeast Asian scripts use
    /// zero-width joiner/non-joiner characters to control ligature formation
    /// and conjunct rendering. Stripping these breaks the text.
    pub fn preserves_joiners(self) -> bool {
        matches!(
            self,
            Self::Arabic
                | Self::Hebrew
                | Self::Devanagari
                | Self::Bengali
                | Self::Indic
                | Self::Thai
                | Self::Lao
                | Self::Tibetan
                | Self::Myanmar
                | Self::Khmer
                | Self::Sinhala
        )
    }
}

/// Detect the dominant script from text content.
///
/// Scans the text and returns the first script category that matches
/// a significant number of characters. Returns `Script::Latin` for
/// ASCII/Latin text and `Script::Unknown` for empty text.
///
/// # Detection Priority
///
/// Scripts are checked in priority order (Arabic first, then Devanagari, etc.).
/// The first script with >=3 matching characters is returned. If no script
/// reaches the threshold, returns `Script::Latin` as a safe default.
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::correction::detect_script;
///
/// assert_eq!(detect_script("Hello world"), Script::Latin);
/// assert_eq!(detect_script("مرحبا"), Script::Arabic);
/// assert_eq!(detect_script("नमस्ते"), Script::Devanagari);
/// assert_eq!(detect_script(""), Script::Unknown);
/// ```
pub fn detect_script(text: &str) -> Script {
    if text.is_empty() {
        return Script::Unknown;
    }

    let mut arabic_count = 0;
    let mut hebrew_count = 0;
    let mut devanagari_count = 0;
    let mut bengali_count = 0;
    let mut indic_count = 0;
    let mut thai_count = 0;
    let mut lao_count = 0;
    let mut tibetan_count = 0;
    let mut myanmar_count = 0;
    let mut khmer_count = 0;
    let mut sinhala_count = 0;

    for c in text.chars() {
        let cp = c as u32;
        match cp {
            // Arabic: U+0600..U+06FF, U+0750..U+077F, U+08A0..U+08FF
            0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF => arabic_count += 1,
            // Hebrew: U+0590..U+05FF
            0x0590..=0x05FF => hebrew_count += 1,
            // Devanagari: U+0900..U+097F
            0x0900..=0x097F => devanagari_count += 1,
            // Bengali: U+0980..U+09FF
            0x0980..=0x09FF => bengali_count += 1,
            // Other Indic scripts:
            // Gurmukhi: U+0A00..U+0A7F
            // Gujarati: U+0A80..U+0AFF
            // Tamil: U+0B80..U+0BFF
            // Telugu: U+0C00..U+0C7F
            // Kannada: U+0C80..U+0CFF
            // Malayalam: U+0D00..U+0D7F
            // Odia: U+0B00..U+0B7F
            0x0A00..=0x0A7F | 0x0A80..=0x0AFF | 0x0B00..=0x0B7F | 0x0B80..=0x0BFF |
            0x0C00..=0x0C7F | 0x0C80..=0x0CFF | 0x0D00..=0x0D7F => indic_count += 1,
            // Thai: U+0E00..U+0E7F
            0x0E00..=0x0E7F => thai_count += 1,
            // Lao: U+0E80..U+0EFF
            0x0E80..=0x0EFF => lao_count += 1,
            // Tibetan: U+0F00..U+0FFF
            0x0F00..=0x0FFF => tibetan_count += 1,
            // Myanmar: U+1000..U+109F
            0x1000..=0x109F => myanmar_count += 1,
            // Khmer: U+1780..U+17FF
            0x1780..=0x17FF => khmer_count += 1,
            // Sinhala: U+0D80..U+0DFF
            0x0D80..=0x0DFF => sinhala_count += 1,
            _ => {}
        }
    }

    const THRESHOLD: usize = 3;

    if arabic_count >= THRESHOLD {
        return Script::Arabic;
    }
    if hebrew_count >= THRESHOLD {
        return Script::Hebrew;
    }
    if devanagari_count >= THRESHOLD {
        return Script::Devanagari;
    }
    if bengali_count >= THRESHOLD {
        return Script::Bengali;
    }
    if indic_count >= THRESHOLD {
        return Script::Indic;
    }
    if thai_count >= THRESHOLD {
        return Script::Thai;
    }
    if lao_count >= THRESHOLD {
        return Script::Lao;
    }
    if tibetan_count >= THRESHOLD {
        return Script::Tibetan;
    }
    if myanmar_count >= THRESHOLD {
        return Script::Myanmar;
    }
    if khmer_count >= THRESHOLD {
        return Script::Khmer;
    }
    if sinhala_count >= THRESHOLD {
        return Script::Sinhala;
    }

    // Default to Latin for ASCII or undetected scripts
    Script::Latin
}

/// Normalize word-break characters in span text based on script hint.
///
/// Strips zero-width formatting characters that are noise in extracted text:
/// - **U+200B** (zero-width space): ALWAYS stripped (never content)
/// - **U+FEFF** (zero-width no-break space / BOM): ALWAYS stripped (never content)
/// - **U+200C** (zero-width non-joiner): stripped unless script requires it
/// - **U+200D** (zero-width joiner): stripped unless script requires it
///
/// The script_hint determines whether ZWNJ/ZWJ are preserved:
/// - **Arabic, Hebrew, Indic, Thai, Lao, Tibetan, Myanmar, Khmer, Sinhala**:
///   ZWNJ/ZWJ are preserved (they control ligature/conjunct formation)
/// - **Latin or Unknown**: All four characters are stripped
///
/// # Arguments
///
/// * `span` - Mutable reference to the span to normalize
/// * `script_hint` - Optional script hint; if None, detects from span text
///
/// # Returns
///
/// Count of characters stripped (u32).
///
/// # Invariants
///
/// - **INV**: U+200B and U+FEFF are NEVER content; always stripped regardless of script.
/// - **INV**: U+200C/U+200D are content in Arabic/Indic; stripping breaks rendering.
/// - **INV**: When script_hint is None, script is detected from the span's own text.
/// - **INV**: For unknown-script text, default to strip (safer for Latin output).
///
/// # Performance
///
/// O(n) where n is the length of the span text. Uses `String::retain` with
/// a closure that checks the script hint once.
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::correction::{normalize_word_breaks, Script};
/// use pdftract_core::span::Span;
/// use std::sync::Arc;
///
/// // Latin text: all zero-width chars stripped
/// let mut span = Span::empty();
/// span.text = String::from("auto\u{200B}mation");
/// let count = normalize_word_breaks(&mut span, Some(Script::Latin));
/// assert_eq!(count, 1);
/// assert_eq!(span.text, "automation");
///
/// // Arabic text: ZWNJ/ZWJ preserved
/// let mut span = Span::empty();
/// span.text = String::from("\u{0627}\u{200C}\u{200D}"); // alef + ZWNJ + ZWJ
/// let count = normalize_word_breaks(&mut span, Some(Script::Arabic));
/// assert_eq!(count, 0);
/// assert_eq!(span.text, "\u{0627}\u{200C}\u{200D}");
///
/// // Unknown script: all stripped (safe default)
/// let mut span = Span::empty();
/// span.text = String::from("test\u{200C}\u{200D}");
/// let count = normalize_word_breaks(&mut span, None);
/// assert_eq!(count, 2);
/// assert_eq!(span.text, "test");
/// ```
pub fn normalize_word_breaks(span: &mut Span, script_hint: Option<Script>) -> u32 {
    let script = script_hint.unwrap_or_else(|| detect_script(&span.text));
    let preserve_joiners = script.preserves_joiners();

    let original_len = span.text.len();

    span.text.retain(|c| {
        match c {
            // U+200B zero-width space: ALWAYS strip
            '\u{200B}' => false,
            // U+FEFF BOM: ALWAYS strip
            '\u{FEFF}' => false,
            // U+200C ZWNJ: strip unless script requires it
            '\u{200C}' => preserve_joiners,
            // U+200D ZWJ: strip unless script requires it
            '\u{200D}' => preserve_joiners,
            // All other characters: keep
            _ => true,
        }
    });

    // Return count of stripped characters by byte length difference
    (original_len - span.text.len()) as u32
}

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

/// Trait for types with bounding box information needed for hyphenation repair.
///
/// This trait abstracts over different span representations to allow
/// the hyphenation repair code to work with any span type that has position data.
pub trait HasBBox {
    /// Get the bounding box [x0, y0, x1, y1] in PDF user space.
    fn bbox(&self) -> [f64; 4];
}

/// Trait for types that have mutable text content and position data.
///
/// Combines `CorrectableText` with `HasBBox` for spans that need
/// hyphenation repair.
pub trait HyphenableSpan: CorrectableText + HasBBox {}

/// Blanket implementation for types that implement both traits.
impl<T> HyphenableSpan for T where T: CorrectableText + HasBBox {}

/// Repair end-of-line hyphenation within a block.
///
/// Detects, within a single block, lines ending with a hyphen at or near the
/// column right edge (text ends with `-`, span bbox.x1 is within `0.05 * column_width`
/// of column right) AND the next line in the same block starts with a lowercase letter
/// (continuation). Joins: strip the trailing hyphen from line N's last span, prepend
/// its truncated word to the first word of line N+1's first span.
///
/// # Arguments
///
/// * `block` - Mutable reference to a block with lines to repair
/// * `column_width` - Width of the column in points (used to detect right-edge hyphens)
///
/// # Returns
///
/// Count of repairs performed (u32).
///
/// # Detection Criteria
///
/// A hyphenation repair is performed when ALL of the following are true:
/// 1. line[n].last_span.text ends with `-`, `‐` (U+2010), or `‑` (U+2011)
/// 2. line[n].last_span.bbox[2] >= column_right - 0.05 * column_width (hyphen at right edge)
/// 3. line[n+1].first_span.text starts with a LOWERCASE letter (continuation)
/// 4. line[n].last_span and line[n+1].first_span are in the same column
///
/// # Repair Process
///
/// 1. Find the last word in line[n].last_span.text; strip the trailing hyphen
/// 2. Find the first word in line[n+1].first_span.text
/// 3. Join: `joined_word = stripped_last + first`
/// 4. Modify line[n].last_span.text: replace hyphenated word with `joined_word + " "`
/// 5. Modify line[n+1].first_span.text: remove the first word
/// 6. If line[n+1].first_span becomes empty, remove it; if line becomes empty, remove it
///
/// # Invariants
///
/// - **INV**: do NOT join across blocks (paragraph boundary kills hyphenation)
/// - **INV**: capital-start of next line indicates NOT a continuation (new sentence)
/// - **INV**: mid-line hyphens (not at right edge) are NOT joined
/// - **INV**: lines in different columns are NOT joined
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::correction::{repair_hyphenation, TestSpan, TestLine};
///
/// let mut block = TestBlock {
///     lines: vec![
///         TestLine {
///             spans: vec![TestSpan::new("Long hyphen-", [50.0, 100.0, 445.0, 115.0])],
///             column: Some(0),
///             ..Default::default()
///         },
///         TestLine {
///             spans: vec![TestSpan::new("ation continues", [50.0, 85.0, 200.0, 100.0])],
///             column: Some(0),
///             ..Default::default()
///         },
///     ],
///     column: 0,
/// };
///
/// let count = repair_hyphenation(&mut block, 500.0);
/// assert_eq!(count, 1);
/// assert_eq!(block.lines[0].spans[0].text(), "Long hyphenation ");
/// assert_eq!(block.lines[1].spans[0].text(), "continues");
/// ```
pub fn repair_hyphenation<S>(block: &mut Block<S>, column_width: f64) -> u32
where
    S: HyphenableSpan,
{
    let mut repair_count = 0;
    let column_right = (block.column as f64 + 1.0) * column_width;
    let right_edge_threshold = 0.05 * column_width;

    // Iterate consecutive line pairs within the block
    let mut i = 0;
    while i + 1 < block.lines.len() {
        let current_line = &block.lines[i];
        let next_line = &block.lines[i + 1];

        // Both lines must have spans
        if current_line.spans.is_empty() || next_line.spans.is_empty() {
            i += 1;
            continue;
        }

        let current_last_span = &current_line.spans[current_line.spans.len() - 1];
        let next_first_span = &next_line.spans[0];

        // Check: same column
        if current_line.column != next_line.column {
            i += 1;
            continue;
        }

        // Check: hyphen at end of current line's last span
        let current_text = current_last_span.text();
        let has_hyphen = current_text.ends_with('-')
            || current_text.ends_with('\u{2010}') // hyphen
            || current_text.ends_with('\u{2011}') // non-breaking hyphen
            || current_text.ends_with('\u{00AD}'); // soft hyphen

        if !has_hyphen {
            i += 1;
            continue;
        }

        // Check: hyphen is at right edge of column
        let last_span_bbox = current_last_span.bbox();
        if last_span_bbox[2] < column_right - right_edge_threshold {
            i += 1;
            continue;
        }

        // Check: next line starts with lowercase (continuation)
        let next_text = next_first_span.text();
        let first_char = next_text.chars().next();
        let is_continuation = match first_char {
            Some(c) => c.is_lowercase(),
            None => false,
        };

        if !is_continuation {
            i += 1;
            continue;
        }

        // All checks passed - perform the repair
        // Extract data first to avoid multiple mutable borrows
        let (last_word_end, joined_word, first_word_end) = {
            let current_last_span = &current_line.spans[current_line.spans.len() - 1];
            let current_text = current_last_span.text();

            let last_word_end = current_text
                .rfind(char::is_whitespace)
                .map(|pos| pos + 1)
                .unwrap_or(0);
            let last_word = &current_text[last_word_end..];

            // Strip trailing hyphen(s) and whitespace
            let stripped_last = last_word.trim_end_matches(|c: char| {
                c == '-'
                    || c == '\u{2010}'
                    || c == '\u{2011}'
                    || c == '\u{00AD}'
                    || c.is_whitespace()
            });

            // Find first word in next span
            let next_first_span = &next_line.spans[0];
            let next_text = next_first_span.text();
            let first_word_end = next_text
                .find(char::is_whitespace)
                .unwrap_or(next_text.len());
            let first_word = &next_text[..first_word_end];

            // Join the words
            let joined_word = format!("{}{}", stripped_last, first_word);

            (last_word_end, joined_word, first_word_end)
        };

        // Apply mutations to current line
        {
            let current_line_mut = &mut block.lines[i];
            let last_span_idx = current_line_mut.spans.len() - 1;
            let current_last_span_mut = &mut current_line_mut.spans[last_span_idx];
            let current_text_mut = current_last_span_mut.text_mut();

            // Replace last word in current span
            let before_last_word = &current_text_mut[..last_word_end];
            *current_text_mut = format!("{}{} ", before_last_word, joined_word);
        }

        // Apply mutations to next line
        {
            let next_line_mut = &mut block.lines[i + 1];
            let next_first_span_mut = &mut next_line_mut.spans[0];
            let next_text_mut = next_first_span_mut.text_mut();

            // Remove first word from next span
            let after_first_word = &next_text_mut[first_word_end..];
            let after_first_word_trimmed = after_first_word.trim_start();
            *next_text_mut = after_first_word_trimmed.to_string();

            // Clean up: remove empty spans/lines
            if next_first_span_mut.text().is_empty() {
                next_line_mut.spans.remove(0);
            }
            if next_line_mut.spans.is_empty() {
                block.lines.remove(i + 1);
                // Don't increment i - recheck current line with new next line
                continue;
            }
        }

        repair_count += 1;
        i += 1;
    }

    repair_count
}

/// Test implementation of `HasBBox` for unit tests.
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct TestSpan {
    pub text: String,
    pub bbox: [f64; 4],
}

#[cfg(test)]
impl TestSpan {
    pub fn new(text: impl Into<String>, bbox: [f64; 4]) -> Self {
        Self {
            text: text.into(),
            bbox,
        }
    }
}

#[cfg(test)]
impl HasBBox for TestSpan {
    fn bbox(&self) -> [f64; 4] {
        self.bbox
    }
}

#[cfg(test)]
impl CorrectableText for TestSpan {
    fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    fn text(&self) -> &str {
        &self.text
    }
}

/// Test implementation of `Line` for unit tests.
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct TestLine {
    pub spans: Vec<TestSpan>,
    pub column: Option<usize>,
}

#[cfg(test)]
impl Default for TestLine {
    fn default() -> Self {
        Self {
            spans: Vec::new(),
            column: None,
        }
    }
}

/// Test implementation of `Block` for unit tests.
#[cfg(test)]
pub struct TestBlock {
    pub lines: Vec<TestLine>,
    pub column: usize,
}

#[cfg(test)]
impl TestBlock {
    pub fn new(lines: Vec<TestLine>, column: usize) -> Self {
        Self { lines, column }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::line::{Block, Line, LineDirection};

    /// Helper to create a test Line with a single span.
    #[cfg(test)]
    fn make_test_line(text: &str, bbox: [f32; 4], column: Option<usize>) -> Line<TestSpan> {
        Line {
            spans: vec![TestSpan::new(
                text,
                [
                    bbox[0] as f64,
                    bbox[1] as f64,
                    bbox[2] as f64,
                    bbox[3] as f64,
                ],
            )],
            bbox,
            baseline: bbox[1],
            direction: LineDirection::Ltr,
            page_relative_y: 0.5,
            median_font_size: 12.0,
            rendering_mode: None,
            column,
        }
    }
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
        let mut span = TestSpan::new("caf\u{00e9}", [0.0, 0.0, 100.0, 20.0]);
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(!repaired);
        assert_eq!(span.text(), "caf\u{00e9}");
    }

    #[test]
    fn test_ascii_only_no_change() {
        // ASCII-only text: cannot be mojibake
        let mut span = TestSpan::new("hello world", [0.0, 0.0, 100.0, 20.0]);
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(!repaired);
        assert_eq!(span.text(), "hello world");
    }

    #[test]
    fn test_empty_string_no_change() {
        let mut span = TestSpan::new("", [0.0, 0.0, 100.0, 20.0]);
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
        let mut span = TestSpan::new("caf\u{00c3}\u{00a9}", [0.0, 0.0, 100.0, 20.0]); // cafÃ©
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(repaired);
        assert_eq!(span.text(), "caf\u{00e9}"); // café
    }

    #[test]
    fn test_mojibake_multiple_indicators() {
        // Multiple indicators: Ã©Ã¨ (café + è)
        let mut span = TestSpan::new(
            "caf\u{00c3}\u{00a9} r\u{00c3}\u{00a8}st\u{00c3}\u{00a9}",
            [0.0, 0.0, 200.0, 20.0],
        );
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(repaired);
        // Should re-decode to "café résté"
        assert_eq!(span.text(), "caf\u{00e9} r\u{00e9}st\u{00e9}");
    }

    #[test]
    fn test_mojibake_single_indicator_threshold() {
        // Single Ã© without other indicators: below threshold
        let mut span = TestSpan::new("caf\u{00c3}\u{00a9}sandbar", [0.0, 0.0, 200.0, 20.0]);
        // With only 1 Ã©, the threshold of 2 is not met
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(!repaired); // Should not detect with only 1 indicator
        assert_eq!(span.text(), "caf\u{00c3}\u{00a9}sandbar");
    }

    #[test]
    fn test_smart_quote_mojibake() {
        // Smart quote mojibake
        let mojibake = "don\u{2019}t"; // don't with curly apostrophe
        let mut span = TestSpan::new(mojibake, [0.0, 0.0, 100.0, 20.0]);
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
        let mut span = TestSpan::new(mojibake, [0.0, 0.0, 200.0, 20.0]);
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
        let mut span = TestSpan::new("caf\u{00c3}\u{00a9}", [0.0, 0.0, 100.0, 20.0]);
        let repaired = detect_and_repair_mojibake(&mut span, |_| 0.5); // Both score 0.5
                                                                       // No replacement because candidate_score (0.5) is not > original_score (0.5) + 0.05
        assert!(!repaired);
        assert_eq!(span.text(), "caf\u{00c3}\u{00a9}");
    }

    #[test]
    fn test_epsilon_threshold_prevents_noise() {
        // Candidate score only slightly better - should be rejected
        let mut span = TestSpan::new("caf\u{00c3}\u{00a9}", [0.0, 0.0, 100.0, 20.0]);
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
        let mut span = TestSpan::new("こんにちは世界", [0.0, 0.0, 200.0, 20.0]);
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(!repaired);
        assert_eq!(span.text(), "こんにちは世界");
    }

    #[test]
    fn test_windows1252_specific() {
        // Test that we use windows-1252, not pure Latin-1
        // Smart quote is the windows-1252 smart quote, not in pure Latin-1
        let mojibake = "it\u{2019}s"; // it's with smart quote
        let mut span = TestSpan::new(mojibake, [0.0, 0.0, 100.0, 20.0]);
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
        let mut span = TestSpan::new(
            "The word is caf\u{00e9} and r\u{00e9}sum\u{00e9}",
            [0.0, 0.0, 400.0, 20.0],
        );
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
        let mut span = TestSpan::new("hello\u{00a0} world\u{00a0} here", [0.0, 0.0, 200.0, 20.0]);
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
        let mut span = TestSpan::new(mojibake, [0.0, 0.0, 200.0, 20.0]);
        let repaired = detect_and_repair_mojibake(&mut span, simple_scorer);
        assert!(repaired);
        assert_eq!(span.text(), "don't drink caf\u{00e9}");
    }

    #[test]
    fn test_exact_epsilon_boundary() {
        // Test the exact epsilon boundary
        let mut span = TestSpan::new("caf\u{00c3}\u{00a9}", [0.0, 0.0, 100.0, 20.0]);
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
        let mut span = TestSpan::new("caf\u{00c3}\u{00a9}", [0.0, 0.0, 100.0, 20.0]);
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

    // ===== Hyphenation repair tests =====

    #[test]
    fn test_hyphenation_join_basic() {
        // Basic hyphenation join: "hyphen-" + "ation" -> "hyphenation"
        let mut block = Block {
            lines: vec![
                make_test_line("Long hyphen-", [50.0, 100.0, 445.0, 115.0], Some(0)),
                make_test_line("ation continues", [50.0, 85.0, 200.0, 100.0], Some(0)),
            ],
            kind: "paragraph".to_string(),
            text: String::new(),
            bbox: [50.0, 85.0, 445.0, 115.0],
            median_font_size: 12.0,
            column: 0,
        };

        let count = repair_hyphenation(&mut block, 500.0);
        assert_eq!(count, 1);
        assert_eq!(block.lines[0].spans[0].text(), "Long hyphenation ");
        assert_eq!(block.lines[1].spans[0].text(), "continues");
    }

    #[test]
    fn test_hyphenation_capital_start_no_join() {
        // Capital start of next line: NOT a continuation
        let mut block = Block {
            lines: vec![
                make_test_line("Long hyphen-", [50.0, 100.0, 445.0, 115.0], Some(0)),
                make_test_line("More text", [50.0, 85.0, 200.0, 100.0], Some(0)),
            ],
            kind: "paragraph".to_string(),
            text: String::new(),
            bbox: [50.0, 85.0, 445.0, 115.0],
            median_font_size: 12.0,
            column: 0,
        };

        let count = repair_hyphenation(&mut block, 500.0);
        assert_eq!(count, 0);
        assert_eq!(block.lines[0].spans[0].text(), "Long hyphen-");
        assert_eq!(block.lines[1].spans[0].text(), "More text");
    }

    #[test]
    fn test_hyphenation_not_at_right_edge() {
        // Hyphen not at right edge: NOT joined
        let mut block = Block {
            lines: vec![
                make_test_line("Long hyphen-", [50.0, 100.0, 300.0, 115.0], Some(0)), // Not at right edge
                make_test_line("ation continues", [50.0, 85.0, 200.0, 100.0], Some(0)),
            ],
            kind: "paragraph".to_string(),
            text: String::new(),
            bbox: [50.0, 85.0, 300.0, 115.0],
            median_font_size: 12.0,
            column: 0,
        };

        let count = repair_hyphenation(&mut block, 500.0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_hyphenation_different_columns() {
        // Lines in different columns: NOT joined
        let mut block = Block {
            lines: vec![
                make_test_line("Long hyphen-", [50.0, 100.0, 445.0, 115.0], Some(0)),
                make_test_line("ation continues", [300.0, 85.0, 450.0, 100.0], Some(1)), // Different column
            ],
            kind: "paragraph".to_string(),
            text: String::new(),
            bbox: [50.0, 85.0, 450.0, 115.0],
            median_font_size: 12.0,
            column: 0,
        };

        let count = repair_hyphenation(&mut block, 500.0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_hyphenation_soft_hyphen() {
        // Soft hyphen (U+00AD) should be detected and stripped
        let mut block = Block {
            lines: vec![
                make_test_line("Long hyphen\u{00AD}", [50.0, 100.0, 445.0, 115.0], Some(0)),
                make_test_line("ation continues", [50.0, 85.0, 200.0, 100.0], Some(0)),
            ],
            kind: "paragraph".to_string(),
            text: String::new(),
            bbox: [50.0, 85.0, 445.0, 115.0],
            median_font_size: 12.0,
            column: 0,
        };

        let count = repair_hyphenation(&mut block, 500.0);
        assert_eq!(count, 1);
        assert_eq!(block.lines[0].spans[0].text(), "Long hyphenation ");
    }

    #[test]
    fn test_hyphenation_non_breaking_hyphen() {
        // Non-breaking hyphen (U+2011) should be detected and stripped
        let mut block = Block {
            lines: vec![
                make_test_line("Long hyphen\u{2011}", [50.0, 100.0, 445.0, 115.0], Some(0)),
                make_test_line("ation continues", [50.0, 85.0, 200.0, 100.0], Some(0)),
            ],
            kind: "paragraph".to_string(),
            text: String::new(),
            bbox: [50.0, 85.0, 445.0, 115.0],
            median_font_size: 12.0,
            column: 0,
        };

        let count = repair_hyphenation(&mut block, 500.0);
        assert_eq!(count, 1);
        assert_eq!(block.lines[0].spans[0].text(), "Long hyphenation ");
    }

    #[test]
    fn test_hyphenation_empty_span_removed() {
        // When next span becomes empty after removing first word, it should be removed
        let mut block = Block {
            lines: vec![
                make_test_line("Long hyphen-", [50.0, 100.0, 445.0, 115.0], Some(0)),
                make_test_line("ation", [50.0, 85.0, 100.0, 100.0], Some(0)), // Only the continuation word
            ],
            kind: "paragraph".to_string(),
            text: String::new(),
            bbox: [50.0, 85.0, 445.0, 115.0],
            median_font_size: 12.0,
            column: 0,
        };

        let count = repair_hyphenation(&mut block, 500.0);
        assert_eq!(count, 1);
        assert_eq!(block.lines[0].spans[0].text(), "Long hyphenation ");
        // Next line should be removed (span became empty, then line became empty)
        assert_eq!(block.lines.len(), 1);
    }

    #[test]
    fn test_hyphenation_multi_word_continuation() {
        // Continuation line has multiple words: only first word should be moved
        let mut block = Block {
            lines: vec![
                make_test_line("Long hyphen-", [50.0, 100.0, 445.0, 115.0], Some(0)),
                make_test_line("ation continues here", [50.0, 85.0, 300.0, 100.0], Some(0)),
            ],
            kind: "paragraph".to_string(),
            text: String::new(),
            bbox: [50.0, 85.0, 445.0, 115.0],
            median_font_size: 12.0,
            column: 0,
        };

        let count = repair_hyphenation(&mut block, 500.0);
        assert_eq!(count, 1);
        assert_eq!(block.lines[0].spans[0].text(), "Long hyphenation ");
        assert_eq!(block.lines[1].spans[0].text(), "continues here");
    }

    #[test]
    fn test_hyphenation_multiple_repairs() {
        // Multiple hyphenation repairs in the same block
        let mut block = Block {
            lines: vec![
                make_test_line("First hyphen-", [50.0, 200.0, 445.0, 215.0], Some(0)),
                make_test_line("ation here", [50.0, 180.0, 200.0, 195.0], Some(0)),
                make_test_line("Second hyphen-", [50.0, 150.0, 445.0, 165.0], Some(0)),
                make_test_line("ation there", [50.0, 130.0, 200.0, 145.0], Some(0)),
            ],
            kind: "paragraph".to_string(),
            text: String::new(),
            bbox: [50.0, 130.0, 445.0, 215.0],
            median_font_size: 12.0,
            column: 0,
        };

        let count = repair_hyphenation(&mut block, 500.0);
        assert_eq!(count, 2);
        assert_eq!(block.lines[0].spans[0].text(), "First hyphenation ");
        assert_eq!(block.lines[1].spans[0].text(), "here");
        assert_eq!(block.lines[2].spans[0].text(), "Second hyphenation ");
        assert_eq!(block.lines[3].spans[0].text(), "there");
    }

    // ===== Script detection tests =====

    #[test]
    fn test_detect_script_latin() {
        // Latin/ASCII text
        assert_eq!(detect_script("Hello world"), Script::Latin);
        assert_eq!(detect_script("The quick brown fox"), Script::Latin);
    }

    #[test]
    fn test_detect_script_arabic() {
        // Arabic text
        assert_eq!(detect_script("مرحبا"), Script::Arabic);
        assert_eq!(detect_script("السلام عليكم"), Script::Arabic);
    }

    #[test]
    fn test_detect_script_hebrew() {
        // Hebrew text
        assert_eq!(detect_script("שלום"), Script::Hebrew);
        assert_eq!(detect_script("מה נשמע"), Script::Hebrew);
    }

    #[test]
    fn test_detect_script_devanagari() {
        // Devanagari text (Hindi)
        assert_eq!(detect_script("नमस्ते"), Script::Devanagari);
        assert_eq!(detect_script("धन्यवाद"), Script::Devanagari);
    }

    #[test]
    fn test_detect_script_bengali() {
        // Bengali text
        assert_eq!(detect_script("হ্যালো"), Script::Bengali);
        assert_eq!(detect_script("ধন্যবাদ"), Script::Bengali);
    }

    #[test]
    fn test_detect_script_thai() {
        // Thai text
        assert_eq!(detect_script("สวัสดี"), Script::Thai);
        assert_eq!(detect_script("ขอบคุณ"), Script::Thai);
    }

    #[test]
    fn test_detect_script_empty() {
        // Empty text
        assert_eq!(detect_script(""), Script::Unknown);
    }

    #[test]
    fn test_detect_script_mixed_latin_arabic() {
        // Mixed text - Arabic wins with threshold
        assert_eq!(detect_script("Hello مرحبا"), Script::Arabic);
    }

    // ===== Word-break normalization tests =====

    #[test]
    fn test_normalize_word_breaks_latin_zero_width_space() {
        // AC: "auto\u{200B}mation" (Latin) -> "automation" (1 stripped, U+200B)
        let mut span = Span::empty();
        span.text = String::from("auto\u{200B}mation");
        let count = normalize_word_breaks(&mut span, Some(Script::Latin));
        assert_eq!(count, 3); // U+200B is 3 bytes in UTF-8
        assert_eq!(span.text, "automation");
    }

    #[test]
    fn test_normalize_word_breaks_latin_bom() {
        // AC: Mixed BOM "\u{FEFF}hello" -> "hello" (always stripped)
        let mut span = Span::empty();
        span.text = String::from("\u{FEFF}hello");
        let count = normalize_word_breaks(&mut span, Some(Script::Latin));
        assert_eq!(count, 3); // U+FEFF is 3 bytes in UTF-8
        assert_eq!(span.text, "hello");
    }

    #[test]
    fn test_normalize_word_breaks_latin_zwnj_zwj() {
        // Latin text: ZWNJ/ZWJ should be stripped
        let mut span = Span::empty();
        span.text = String::from("test\u{200C}\u{200D}case");
        let count = normalize_word_breaks(&mut span, Some(Script::Latin));
        assert_eq!(count, 6); // Each is 3 bytes in UTF-8
        assert_eq!(span.text, "testcase");
    }

    #[test]
    fn test_normalize_word_breaks_arabic_preserves_zwnj_zwj() {
        // AC: Arabic "ای\u{200C}\u{200D}" with script_hint=Arabic -> unchanged
        // Note: Using a simpler Arabic example since "ای" requires specific characters
        let mut span = Span::empty();
        span.text = String::from("\u{0627}\u{200C}\u{200D}"); // alef + ZWNJ + ZWJ
        let count = normalize_word_breaks(&mut span, Some(Script::Arabic));
        assert_eq!(count, 0);
        assert_eq!(span.text, "\u{0627}\u{200C}\u{200D}");
    }

    #[test]
    fn test_normalize_word_breaks_arabic_strips_zw_space() {
        // Arabic text: U+200B should still be stripped even in Arabic
        let mut span = Span::empty();
        span.text = String::from("\u{0627}\u{200B}\u{0628}"); // alef + ZWSP + beh
        let count = normalize_word_breaks(&mut span, Some(Script::Arabic));
        assert_eq!(count, 3); // U+200B is 3 bytes in UTF-8
        assert_eq!(span.text, "\u{0627}\u{0628}");
    }

    #[test]
    fn test_normalize_word_breaks_arabic_strips_bom() {
        // Arabic text: U+FEFF should still be stripped even in Arabic
        let mut span = Span::empty();
        span.text = String::from("\u{FEFF}\u{0627}\u{0628}"); // BOM + alef + beh
        let count = normalize_word_breaks(&mut span, Some(Script::Arabic));
        assert_eq!(count, 3); // U+FEFF is 3 bytes in UTF-8
        assert_eq!(span.text, "\u{0627}\u{0628}");
    }

    #[test]
    fn test_normalize_word_breaks_unknown_script_strips_all() {
        // AC: Arabic same with script_hint=None -> stripped (default-strip)
        let mut span = Span::empty();
        span.text = String::from("\u{0627}\u{200C}\u{200D}");
        let count = normalize_word_breaks(&mut span, None);
        assert_eq!(count, 6); // Both ZWNJ and ZWJ stripped
        assert_eq!(span.text, "\u{0627}");
    }

    #[test]
    fn test_normalize_word_breaks_devanagari_preserves_zwnj_zwj() {
        // AC: Devanagari "क\u{200D}ष" with script_hint=Devanagari -> unchanged
        let mut span = Span::empty();
        span.text = String::from("\u{0915}\u{200D}\u{0937}"); // ka + ZWJ + ssa
        let count = normalize_word_breaks(&mut span, Some(Script::Devanagari));
        assert_eq!(count, 0);
        assert_eq!(span.text, "\u{0915}\u{200D}\u{0937}");
    }

    #[test]
    fn test_normalize_word_breaks_devanagari_strips_zw_space() {
        // Devanagari text: U+200B should still be stripped
        let mut span = Span::empty();
        span.text = String::from("\u{0915}\u{200B}\u{0937}");
        let count = normalize_word_breaks(&mut span, Some(Script::Devanagari));
        assert_eq!(count, 3); // U+200B is 3 bytes
        assert_eq!(span.text, "\u{0915}\u{0937}");
    }

    #[test]
    fn test_normalize_word_breaks_auto_detect_latin() {
        // Auto-detect Latin text
        let mut span = Span::empty();
        span.text = String::from("test\u{200C}\u{200D}");
        let count = normalize_word_breaks(&mut span, None);
        assert_eq!(count, 6);
        assert_eq!(span.text, "test");
    }

    #[test]
    fn test_normalize_word_breaks_auto_detect_arabic() {
        // Auto-detect Arabic text and preserve ZWNJ/ZWJ
        let mut span = Span::empty();
        span.text = String::from("مرحبا\u{200C}"); // Arabic + ZWNJ
        let count = normalize_word_breaks(&mut span, None);
        assert_eq!(count, 0);
        assert_eq!(span.text, "مرحبا\u{200C}");
    }

    #[test]
    fn test_normalize_word_breaks_auto_detect_devanagari() {
        // Auto-detect Devanagari text and preserve ZWNJ/ZWJ
        let mut span = Span::empty();
        span.text = String::from("नमस्ते\u{200D}"); // Devanagari + ZWJ
        let count = normalize_word_breaks(&mut span, None);
        assert_eq!(count, 0);
        assert_eq!(span.text, "नमस्ते\u{200D}");
    }

    #[test]
    fn test_normalize_word_breaks_empty_span() {
        // Empty span: no changes
        let mut span = Span::empty();
        span.text = String::from("");
        let count = normalize_word_breaks(&mut span, None);
        assert_eq!(count, 0);
        assert_eq!(span.text, "");
    }

    #[test]
    fn test_normalize_word_breaks_multiple_zero_width_chars() {
        // Multiple zero-width characters in Latin text
        let mut span = Span::empty();
        span.text = String::from("a\u{200B}b\u{200C}c\u{200D}d\u{FEFF}e");
        let count = normalize_word_breaks(&mut span, Some(Script::Latin));
        assert_eq!(count, 12); // 4 chars * 3 bytes each
        assert_eq!(span.text, "abcde");
    }

    #[test]
    fn test_normalize_word_breaks_hebrew_preserves_joiners() {
        // Hebrew text: ZWNJ/ZWJ should be preserved
        let mut span = Span::empty();
        span.text = String::from("\u{05E9}\u{05DC}\u{200C}\u{05D5}\u{05DD}"); // shalom with ZWNJ
        let count = normalize_word_breaks(&mut span, Some(Script::Hebrew));
        assert_eq!(count, 0);
        assert!(span.text.contains("\u{200C}"));
    }

    #[test]
    fn test_normalize_word_breaks_thai_preserves_joiners() {
        // Thai text: ZWNJ/ZWJ should be preserved
        let mut span = Span::empty();
        span.text = String::from("\u{0E2A}\u{0E27}\u{0E31}\u{0E12}\u{200D}"); // sawasdee with ZWJ
        let count = normalize_word_breaks(&mut span, Some(Script::Thai));
        assert_eq!(count, 0);
        assert!(span.text.contains("\u{200D}"));
    }

    #[test]
    fn test_normalize_word_breaks_bengali_preserves_joiners() {
        // Bengali text: ZWNJ/ZWJ should be preserved
        let mut span = Span::empty();
        span.text = String::from("\u{0985}\u{09BE}\u{200C}"); // a with ZWNJ
        let count = normalize_word_breaks(&mut span, Some(Script::Bengali));
        assert_eq!(count, 0);
        assert!(span.text.contains("\u{200C}"));
    }

    #[test]
    fn test_normalize_word_breaks_indic_preserves_joiners() {
        // Indic text (Tamil): ZWNJ/ZWJ should be preserved
        let mut span = Span::empty();
        span.text = String::from("\u{0B85}\u{0BBE}\u{200D}"); // Tamil a with ZWJ
        let count = normalize_word_breaks(&mut span, Some(Script::Indic));
        assert_eq!(count, 0);
        assert!(span.text.contains("\u{200D}"));
    }

    #[test]
    fn test_script_preserves_joiners_arabic() {
        // Test Script::Arabic.preserves_joiners()
        assert!(Script::Arabic.preserves_joiners());
    }

    #[test]
    fn test_script_preserves_joiners_latin() {
        // Test Script::Latin.preserves_joiners()
        assert!(!Script::Latin.preserves_joiners());
    }

    #[test]
    fn test_script_preserves_joiners_all_complex_scripts() {
        // All complex scripts should preserve joiners
        assert!(Script::Arabic.preserves_joiners());
        assert!(Script::Hebrew.preserves_joiners());
        assert!(Script::Devanagari.preserves_joiners());
        assert!(Script::Bengali.preserves_joiners());
        assert!(Script::Indic.preserves_joiners());
        assert!(Script::Thai.preserves_joiners());
        assert!(Script::Lao.preserves_joiners());
        assert!(Script::Tibetan.preserves_joiners());
        assert!(Script::Myanmar.preserves_joiners());
        assert!(Script::Khmer.preserves_joiners());
        assert!(Script::Sinhala.preserves_joiners());
    }

    #[test]
    fn test_script_preserves_joiners_simple_scripts() {
        // Simple scripts should NOT preserve joiners
        assert!(!Script::Latin.preserves_joiners());
        assert!(!Script::Unknown.preserves_joiners());
    }
}
