//! Pattern matcher for pdftract grep.
//!
//! Supports two matching modes:
//! - Literal (Aho-Corasick): fast single-pattern and multi-pattern literal search
//! - Regex (regex::Regex): full ECMAScript-ish regex syntax
//!
//! Both modes support:
//! - Case-insensitive matching (-i)
//! - Word-boundary matching (-w)
//! - Invert match (-v) at the span granularity

use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;

/// A match range in a text span, expressed as byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchRange {
    /// Start byte offset (inclusive)
    pub start: usize,
    /// End byte offset (exclusive)
    pub end: usize,
}

impl MatchRange {
    /// Create a new MatchRange.
    ///
    /// # Panics
    /// Panics if `start > end`.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end, "MatchRange start must be <= end");
        Self { start, end }
    }

    /// Get the length of the match in bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if the match is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Get the text slice from the given input.
    #[must_use]
    pub fn get<'a>(&self, text: &'a str) -> Option<&'a str> {
        text.get(self.start..self.end)
    }
}

/// Pattern matcher that can be either literal or regex.
#[derive(Debug)]
pub enum Matcher {
    /// Literal string matching using Aho-Corasick automaton.
    Literal(aho_corasick::AhoCorasick),
    /// Regular expression matching.
    Regex(Regex),
}

impl Matcher {
    /// Build a matcher from the given configuration.
    ///
    /// # Arguments
    /// * `pattern` - The pattern to match
    /// * `use_regex` - If true, compile as regex; otherwise as literal
    /// * `ignore_case` - Enable case-insensitive matching
    /// * `word_regexp` - Match on word boundaries only
    ///
    /// # Errors
    /// Returns an error if:
    /// - The pattern is empty
    /// - The pattern contains a null byte
    /// - Regex compilation fails (with line:col context)
    /// - Word-boundary wrapping produces an invalid regex
    pub fn build(
        pattern: &str,
        use_regex: bool,
        ignore_case: bool,
        word_regexp: bool,
    ) -> Result<Self> {
        // Validate pattern
        if pattern.is_empty() {
            bail!("PATTERN may not be empty");
        }
        if pattern.contains('\0') {
            bail!("PATTERN may not contain null byte");
        }

        // Apply word-boundary wrapping if requested
        let effective_pattern = if word_regexp {
            if use_regex {
                // Regex mode: wrap with \b word-boundary anchors
                format!(r"\b{}\b", pattern)
            } else {
                // Literal mode: word-boundary is handled in post-match check
                // Keep pattern as-is for Aho-Corasick
                pattern.to_string()
            }
        } else {
            pattern.to_string()
        };

        if use_regex {
            // Build regex matcher
            let mut builder = RegexBuilder::new(&effective_pattern);
            builder.case_insensitive(ignore_case);

            match builder.build() {
                Ok(regex) => Ok(Matcher::Regex(regex)),
                Err(e) => {
                    // Try to provide line:col context from the regex error
                    let msg = e.to_string();
                    bail!("Pattern compilation failed: {msg}")
                }
            }
        } else {
            // Build literal Aho-Corasick matcher
            let mut builder = aho_corasick::AhoCorasick::builder();
            builder.ascii_case_insensitive(ignore_case);

            // Aho-Corasick can handle multiple patterns, but we only use one for grep
            let patterns = &[effective_pattern.as_str()];
            match builder.build(patterns) {
                Ok(automaton) => Ok(Matcher::Literal(automaton)),
                Err(e) => {
                    bail!("Failed to build literal matcher: {e}")
                }
            }
        }
    }

    /// Find all matches in the given text.
    ///
    /// Returns an iterator over `MatchRange` values representing byte offsets
    /// of each match in the text.
    ///
    /// For literal mode with word-boundary enabled, performs a post-match check
    /// to ensure the match is surrounded by non-word characters (or string boundaries).
    ///
    /// # Arguments
    /// * `text` - The text to search
    ///
    /// # Returns
    /// An iterator that yields `MatchRange` for each match.
    pub fn find_iter<'a>(&'a self, text: &'a str) -> Box<dyn Iterator<Item = MatchRange> + 'a> {
        match self {
            Matcher::Literal(ac) => {
                // Aho-Corasick yields matches in byte order
                let iter = ac.find_iter(text.as_bytes()).filter_map(|m| {
                    let start = m.start();
                    let end = m.end();
                    // Convert to MatchRange
                    Some(MatchRange::new(start, end))
                });
                Box::new(iter)
            }
            Matcher::Regex(regex) => {
                // Regex yields matches in order
                let iter = regex.find_iter(text).map(|m| {
                    let start = m.start();
                    let end = m.end();
                    MatchRange::new(start, end)
                });
                Box::new(iter)
            }
        }
    }

    /// Find all matches in the given text with word-boundary checking.
    ///
    /// This method should be used when `-w` (word-regexp) is enabled in literal mode.
    /// For regex mode, the word-boundary is already handled by the `\b` anchors.
    ///
    /// # Arguments
    /// * `text` - The text to search
    /// * `check_word_boundary` - If true, filter matches to those on word boundaries
    ///
    /// # Returns
    /// An iterator that yields `MatchRange` for each match (optionally filtered).
    pub fn find_iter_with_word_boundary<'a>(
        &'a self,
        text: &'a str,
        check_word_boundary: bool,
    ) -> Box<dyn Iterator<Item = MatchRange> + 'a> {
        if !check_word_boundary {
            return self.find_iter(text);
        }

        // For literal mode, filter matches by word-boundary check
        if matches!(self, Matcher::Literal(_)) {
            let filtered = self
                .find_iter(text)
                .filter(move |m| is_word_boundary_match(text, m.start, m.end));
            return Box::new(filtered);
        }

        // For regex mode, word-boundary is already applied via \b anchors
        self.find_iter(text)
    }

    /// Check if the pattern matches anywhere in the text.
    ///
    /// This is a convenience method for boolean checks.
    #[must_use]
    pub fn is_match(&self, text: &str) -> bool {
        match self {
            Matcher::Literal(ac) => ac.is_match(text.as_bytes()),
            Matcher::Regex(regex) => regex.is_match(text),
        }
    }
}

/// Check if a match at the given byte offsets is on a word boundary.
///
/// A match is on a word boundary if:
/// - The character before `start` is not a word character (or start is 0)
/// - The character after `end` is not a word character (or end is text length)
///
/// Word characters are ASCII alphanumeric and underscore: [A-Za-z0-9_]
fn is_word_boundary_match(text: &str, start: usize, end: usize) -> bool {
    let bytes = text.as_bytes();

    // Check character before the match
    let before_is_word = if start > 0 {
        let ch = bytes[start - 1];
        is_ascii_word_char(ch)
    } else {
        false
    };

    // Check character after the match
    let after_is_word = if end < bytes.len() {
        let ch = bytes[end];
        is_ascii_word_char(ch)
    } else {
        false
    };

    // Word boundary: not surrounded by word characters on both sides
    !before_is_word && !after_is_word
}

/// Check if a byte is an ASCII word character.
///
/// Word characters are: A-Z, a-z, 0-9, underscore.
#[must_use]
const fn is_ascii_word_char(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')
}

/// Wrapper for regex::RegexBuilder to support case_insensitive method.
struct RegexBuilder(regex::RegexBuilder);

impl RegexBuilder {
    fn new(pattern: &str) -> Self {
        Self(regex::RegexBuilder::new(pattern))
    }

    fn case_insensitive(&mut self, yes: bool) -> &mut Self {
        self.0.case_insensitive(yes);
        self
    }

    fn build(&self) -> Result<Regex> {
        self.0
            .build()
            .map_err(|e| anyhow!("regex build failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_matcher(
        pattern: &str,
        use_regex: bool,
        ignore_case: bool,
        word_regexp: bool,
    ) -> Result<Matcher> {
        Matcher::build(pattern, use_regex, ignore_case, word_regexp)
    }

    #[test]
    fn test_literal_basic_match() {
        let matcher = build_matcher("test", false, false, false).unwrap();
        let text = "this is a test string";
        let matches: Vec<_> = matcher.find_iter(text).collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start, 10);
        assert_eq!(matches[0].end, 14);
        assert_eq!(matches[0].get(text), Some("test"));
    }

    #[test]
    fn test_literal_multiple_matches() {
        let matcher = build_matcher("test", false, false, false).unwrap();
        let text = "test one test two test";
        let matches: Vec<_> = matcher.find_iter(text).collect();
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].get(text), Some("test"));
        assert_eq!(matches[1].get(text), Some("test"));
        assert_eq!(matches[2].get(text), Some("test"));
    }

    #[test]
    fn test_literal_case_insensitive() {
        let matcher = build_matcher("TEST", false, true, false).unwrap();
        let text = "Test test TeSt TEST";
        let matches: Vec<_> = matcher.find_iter(text).collect();
        assert_eq!(matches.len(), 4);
    }

    #[test]
    fn test_literal_word_boundary() {
        let matcher = build_matcher("test", false, false, true).unwrap();
        let text = "test testingATESTtest testcase";
        let matches: Vec<_> = matcher.find_iter_with_word_boundary(text, true).collect();
        // Should match "test" at start, but not "testing", "ATESTtest", "testcase"
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].get(text), Some("test"));
    }

    #[test]
    fn test_literal_word_boundary_case_insensitive() {
        let matcher = build_matcher("FISH", false, true, true).unwrap();
        let text = "fish FISH fisheries fishing";
        let matches: Vec<_> = matcher.find_iter_with_word_boundary(text, true).collect();
        // Should match "fish" and "FISH" but not "fisheries" or "fishing"
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_regex_basic_match() {
        let matcher = build_matcher(r"\d+", true, false, false).unwrap();
        let text = "abc 123 def 456";
        let matches: Vec<_> = matcher.find_iter(text).collect();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].get(text), Some("123"));
        assert_eq!(matches[1].get(text), Some("456"));
    }

    #[test]
    fn test_regex_dollar_amount() {
        let matcher = build_matcher(r"\$\d+\.\d{2}", true, false, false).unwrap();
        let text = "Price: $19.99 and $42.50";
        let matches: Vec<_> = matcher.find_iter(text).collect();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].get(text), Some("$19.99"));
        assert_eq!(matches[1].get(text), Some("$42.50"));
    }

    #[test]
    fn test_regex_case_insensitive() {
        let matcher = build_matcher(r"test", true, true, false).unwrap();
        let text = "Test TEST TeSt";
        let matches: Vec<_> = matcher.find_iter(text).collect();
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_regex_word_boundary() {
        let matcher = build_matcher(r"\btest\b", true, false, true).unwrap();
        let text = "test testingATESTtest testcase";
        let matches: Vec<_> = matcher.find_iter_with_word_boundary(text, true).collect();
        // Should match "test" at start, but not "testing", "ATESTtest", "testcase"
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].get(text), Some("test"));
    }

    #[test]
    fn test_empty_pattern_rejected() {
        let result = build_matcher("", false, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_null_byte_rejected() {
        let result = build_matcher("test\0pattern", false, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("null byte"));
    }

    #[test]
    fn test_match_range_len() {
        let range = MatchRange::new(5, 10);
        assert_eq!(range.len(), 5);
        assert!(!range.is_empty());
    }

    #[test]
    fn test_match_range_empty() {
        let range = MatchRange::new(5, 5);
        assert_eq!(range.len(), 0);
        assert!(range.is_empty());
    }

    #[test]
    fn test_match_range_get() {
        let text = "hello world";
        let range = MatchRange::new(0, 5);
        assert_eq!(range.get(text), Some("hello"));
        let range = MatchRange::new(6, 11);
        assert_eq!(range.get(text), Some("world"));
        let range = MatchRange::new(0, 100);
        assert_eq!(range.get(text), None);
    }

    #[test]
    fn test_is_word_boundary_match() {
        let text = "test testing";

        // "test" at position 0-4 is a word boundary (start of string)
        assert!(is_word_boundary_match(text, 0, 4));

        // "test" within "testing" at 5-9 is NOT a word boundary (preceded by 'e')
        assert!(!is_word_boundary_match(text, 5, 9));

        // "testing" at 5-12 is a word boundary (preceded by space, at end)
        assert!(is_word_boundary_match(text, 5, 12));
    }

    #[test]
    fn test_literal_invoice_search() {
        let matcher = build_matcher("INVOICE", false, true, false).unwrap();
        let text = "Invoice #12345: This is an invoice for services rendered.";
        let matches: Vec<_> = matcher.find_iter(text).collect();
        assert_eq!(matches.len(), 2); // "Invoice" and "invoice"
    }

    #[test]
    fn test_regex_invalid_pattern() {
        let result = build_matcher(r"(?P<unclosed", true, false, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("compilation failed") || err_msg.contains("regex"));
    }

    #[test]
    fn test_literal_no_match() {
        let matcher = build_matcher("xyz", false, false, false).unwrap();
        let text = "hello world";
        let matches: Vec<_> = matcher.find_iter(text).collect();
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_regex_dot_star_greedy() {
        let matcher = build_matcher(r"a.*z", true, false, false).unwrap();
        let text = "a1z a2z a3z";
        let matches: Vec<_> = matcher.find_iter(text).collect();
        // Greedy: matches "a1z a2z a3z"
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].get(text), Some("a1z a2z a3z"));
    }

    #[test]
    fn test_regex_dot_star_non_greedy() {
        let matcher = build_matcher(r"a.*?z", true, false, false).unwrap();
        let text = "a1z a2z a3z";
        let matches: Vec<_> = matcher.find_iter(text).collect();
        // Non-greedy: matches each "aXz"
        assert_eq!(matches.len(), 3);
    }
}
