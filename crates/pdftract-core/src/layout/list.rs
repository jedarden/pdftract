//! List block classifier (Phase 4.4).
//!
//! This module implements classification of blocks as lists based on:
//! 1. At least 80% of lines start with a bullet pattern
//! 2. At least 80% of lines start with a numbered pattern
//!
//! Bullet patterns: `•‣◦⁃\-*` (Unicode bullets and common ASCII marks)
//! Numbered patterns: `\d+[.\)]` (e.g., "1.", "2)", "3.")
//!
//! Lettered (a., b.) and Roman (I., II.) lists are NOT covered in v0.1.0.
//! Indented sub-bullets (nesting) is deferred.


/// Bullet pattern: optional whitespace followed by bullet character followed by whitespace.
///
/// Matches: `•`, `‣`, `◦`, `⁃`, `-`, `*` with optional leading/trailing whitespace.
/// Examples: "* Item", "• Item", "- Item", "  * Item"
pub static BULLET_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

/// Numbered list pattern: optional whitespace followed by digits followed by . or ) followed by whitespace.
///
/// Matches: `1.`, `2)`, `3.`, `10)`, etc.
/// Examples: "1. First", "2) Second", "  10. Item"
pub static NUMBER_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

/// Initialize the regex patterns
fn init_regex() {
    BULLET_RE
        .get_or_init(|| regex::Regex::new(r"^\s*[•‣◦⁃\-\*]\s").expect("invalid BULLET_RE regex"));
    NUMBER_RE
        .get_or_init(|| regex::Regex::new(r"^\s*\d+[.\)]\s").expect("invalid NUMBER_RE regex"));
}

/// Check if a line starts with a bullet pattern.
///
/// # Arguments
///
/// * `line_text` - The text content of the line (typically first span's text)
///
/// # Returns
///
/// `true` if the line starts with a bullet pattern, `false` otherwise.
///
/// # Examples
///
/// ```
/// use pdftract_core::list::starts_with_bullet;
///
/// assert!(starts_with_bullet("* Item"));
/// assert!(starts_with_bullet("• Item"));
/// assert!(starts_with_bullet("- Item"));
/// assert!(starts_with_bullet("  * Item")); // with leading whitespace
///
/// assert!(!starts_with_bullet("Item")); // no bullet
/// assert!(!starts_with_bullet("1. Item")); // numbered, not bullet
/// ```
pub fn starts_with_bullet(line_text: &str) -> bool {
    init_regex();
    BULLET_RE.get().unwrap().is_match(line_text)
}

/// Check if a line starts with a numbered pattern.
///
/// # Arguments
///
/// * `line_text` - The text content of the line (typically first span's text)
///
/// # Returns
///
/// `true` if the line starts with a numbered pattern, `false` otherwise.
///
/// # Examples
///
/// ```
/// use pdftract_core::list::starts_with_number;
///
/// assert!(starts_with_number("1. First"));
/// assert!(starts_with_number("2) Second"));
/// assert!(starts_with_number("10. Item"));
/// assert!(starts_with_number("  3. Item")); // with leading whitespace
///
/// assert!(!starts_with_number("Item")); // no number
/// assert!(!starts_with_number("* Item")); // bullet, not numbered
/// ```
pub fn starts_with_number(line_text: &str) -> bool {
    init_regex();
    NUMBER_RE.get().unwrap().is_match(line_text)
}

/// Trait for lines that can provide their text content.
///
/// This trait allows the list classification logic to work with different
/// line representations while abstracting over text access.
pub trait LineText {
    /// Get the text content of this line.
    ///
    /// For list detection, we use the first span's text as the line's text.
    fn line_text(&self) -> String;
}

/// Classify a block as a list based on bullet/numbered pattern detection.
///
/// A block is classified as a list if at least 80% of its lines start with
/// either a bullet pattern or a numbered pattern.
///
/// # Arguments
///
/// * `block` - The block to classify
///
/// # Returns
///
/// `true` if the block should be classified as a list, `false` otherwise.
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::list::classify_list;
///
/// // Three bullet lines -> list
/// let block = make_block_with_lines(vec!["* One", "* Two", "* Three"]);
/// assert!(classify_list(&block));
///
/// // Two of five bullet lines -> not a list (40% < 80%)
/// let block = make_block_with_lines(vec!["* One", "* Two", "Text", "Text", "Text"]);
/// assert!(!classify_list(&block));
/// ```
pub fn classify_list<S, L>(block: &crate::layout::line::Block<S>) -> bool
where
    L: LineText,
    S: std::ops::Deref<Target = L>,
{
    // Empty blocks are not lists
    if block.lines.is_empty() {
        return false;
    }

    // Count lines that start with bullet or numbered patterns
    let mut list_item_count = 0;

    for line in &block.lines {
        let line_text = line
            .spans
            .first()
            .map(|s| s.line_text())
            .unwrap_or_default();

        if starts_with_bullet(&line_text) || starts_with_number(&line_text) {
            list_item_count += 1;
        }
    }

    // Classify as list if >= 80% of lines match
    let total_lines = block.lines.len();
    let ratio = list_item_count as f64 / total_lines as f64;

    ratio >= 0.8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::line::{Block, Line, LineDirection};
    use std::ops::Deref;

    /// Test helper: create a mock span with text.
    #[derive(Debug, Clone)]
    struct TestSpan {
        text: String,
    }

    impl LineText for TestSpan {
        fn line_text(&self) -> String {
            self.text.clone()
        }
    }

    impl Deref for TestSpan {
        type Target = Self;

        fn deref(&self) -> &Self::Target {
            self
        }
    }

    /// Test helper: create a mock line.
    fn make_test_line(text: &str) -> Line<TestSpan> {
        Line {
            spans: vec![TestSpan {
                text: text.to_string(),
            }],
            bbox: [0.0, 0.0, 100.0, 12.0],
            baseline: 2.4,
            direction: LineDirection::Ltr,
            page_relative_y: 0.5,
            median_font_size: 12.0,
            rendering_mode: None,
            column: Some(0),
        }
    }

    /// Test helper: create a mock block.
    fn make_test_block(lines: Vec<Line<TestSpan>>, kind: &str) -> Block<TestSpan> {
        Block {
            lines,
            kind: kind.to_string(),
            text: String::new(),
            bbox: [0.0, 0.0, 100.0, 12.0],
            median_font_size: 12.0,
            column: 0,
        }
    }

    #[test]
    fn test_starts_with_bullet_asterisk() {
        assert!(starts_with_bullet("* Item"));
        assert!(starts_with_bullet("* ")); // bullet with trailing space
    }

    #[test]
    fn test_starts_with_bullet_dash() {
        assert!(starts_with_bullet("- Item"));
        assert!(starts_with_bullet("- ")); // dash with trailing space
    }

    #[test]
    fn test_starts_with_bullet_unicode() {
        assert!(starts_with_bullet("• Item"));
        assert!(starts_with_bullet("‣ Item"));
        assert!(starts_with_bullet("◦ Item"));
        assert!(starts_with_bullet("⁃ Item"));
    }

    #[test]
    fn test_starts_with_bullet_with_whitespace() {
        assert!(starts_with_bullet("  * Item")); // leading whitespace
        assert!(starts_with_bullet("*  Item")); // multiple spaces after
    }

    #[test]
    fn test_starts_not_with_bullet() {
        assert!(!starts_with_bullet("Item")); // no bullet
        assert!(!starts_with_bullet("1. Item")); // numbered, not bullet
        assert!(!starts_with_bullet("a. Item")); // lettered, not bullet
        assert!(!starts_with_bullet("I. Item")); // Roman, not bullet
    }

    #[test]
    fn test_starts_with_number_dot() {
        assert!(starts_with_number("1. First"));
        assert!(starts_with_number("10. Tenth"));
        assert!(starts_with_number("100. Item"));
    }

    #[test]
    fn test_starts_with_number_paren() {
        assert!(starts_with_number("1) First"));
        assert!(starts_with_number("10) Tenth"));
        assert!(starts_with_number("2) Second"));
    }

    #[test]
    fn test_starts_with_number_with_whitespace() {
        assert!(starts_with_number("  1. Item")); // leading whitespace
        assert!(starts_with_number("1.  Item")); // multiple spaces after
    }

    #[test]
    fn test_starts_not_with_number() {
        assert!(!starts_with_number("Item")); // no number
        assert!(!starts_with_number("* Item")); // bullet, not number
        assert!(!starts_with_number("a. Item")); // lettered, not numbered
        assert!(!starts_with_number("I. Item")); // Roman, not numbered (no digit)
    }

    // ============ classify_list tests ============

    #[test]
    fn test_classify_list_three_bullet_items() {
        // AC: 3 "* Item" lines -> List
        let lines = vec![
            make_test_line("* First"),
            make_test_line("* Second"),
            make_test_line("* Third"),
        ];
        let block = make_test_block(lines, "paragraph");

        assert!(classify_list(&block));
    }

    #[test]
    fn test_classify_list_three_numbered_items() {
        // AC: 3 "1. First/2. Second/3. Third" lines -> List
        let lines = vec![
            make_test_line("1. First"),
            make_test_line("2. Second"),
            make_test_line("3. Third"),
        ];
        let block = make_test_block(lines, "paragraph");

        assert!(classify_list(&block));
    }

    #[test]
    fn test_classify_list_single_bullet_item() {
        // AC: 1 "* Solo" line -> List (100% >= 80%)
        let lines = vec![make_test_line("* Solo")];
        let block = make_test_block(lines, "paragraph");

        assert!(classify_list(&block));
    }

    #[test]
    fn test_classify_list_four_of_five_bullet_items() {
        // AC: 4/5 "- " starts -> List (80% >= 80%)
        let lines = vec![
            make_test_line("- First"),
            make_test_line("- Second"),
            make_test_line("- Third"),
            make_test_line("- Fourth"),
            make_test_line("Not a list item"), // 5th line doesn't match
        ];
        let block = make_test_block(lines, "paragraph");

        assert!(classify_list(&block));
    }

    #[test]
    fn test_classify_list_two_of_five_bullet_items() {
        // AC: 2/5 "- " starts -> NOT List (40% < 80%)
        let lines = vec![
            make_test_line("- First"),
            make_test_line("- Second"),
            make_test_line("Not a list item"),
            make_test_line("Not a list item"),
            make_test_line("Not a list item"),
        ];
        let block = make_test_block(lines, "paragraph");

        assert!(!classify_list(&block));
    }

    #[test]
    fn test_classify_list_empty_block() {
        // Empty blocks are not lists
        let lines = vec![];
        let block = make_test_block(lines, "paragraph");

        assert!(!classify_list(&block));
    }

    #[test]
    fn test_classify_list_zero_matching() {
        // No lines match -> not a list
        let lines = vec![
            make_test_line("First item"),
            make_test_line("Second item"),
            make_test_line("Third item"),
        ];
        let block = make_test_block(lines, "paragraph");

        assert!(!classify_list(&block));
    }

    #[test]
    fn test_classify_list_mixed_bullet_and_numbered() {
        // Mix of bullet and numbered items -> still a list
        let lines = vec![
            make_test_line("* First"),
            make_test_line("1. Second"),
            make_test_line("- Third"),
            make_test_line("2) Fourth"),
        ];
        let block = make_test_block(lines, "paragraph");

        assert!(classify_list(&block));
    }

    #[test]
    fn test_classify_list_unicode_bullets() {
        // Unicode bullets -> list
        let lines = vec![
            make_test_line("• First"),
            make_test_line("‣ Second"),
            make_test_line("◦ Third"),
        ];
        let block = make_test_block(lines, "paragraph");

        assert!(classify_list(&block));
    }

    #[test]
    fn test_classify_list_exactly_80_percent() {
        // 4 out of 5 = 80% exactly -> should be list (>= 80%)
        let lines = vec![
            make_test_line("* First"),
            make_test_line("* Second"),
            make_test_line("* Third"),
            make_test_line("* Fourth"),
            make_test_line("Not a list item"),
        ];
        let block = make_test_block(lines, "paragraph");

        assert!(classify_list(&block));
    }

    #[test]
    fn test_classify_list_just_below_80_percent() {
        // 3 out of 4 = 75% < 80% -> should NOT be list
        let lines = vec![
            make_test_line("* First"),
            make_test_line("* Second"),
            make_test_line("* Third"),
            make_test_line("Not a list item"),
        ];
        let block = make_test_block(lines, "paragraph");

        assert!(!classify_list(&block));
    }
}
