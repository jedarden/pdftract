//! English wordlist for dictionary coverage scoring (Phase 4.7).
//!
//! This module provides a compile-time `phf::Set` of ~20,000 common English
//! words, used to compute the dictionary coverage signal in readability scoring.
//!
//! # Algorithm
//!
//! The wordlist is compiled into a perfect hash function (`phf::Set`) for
//! O(1) lookup performance. The set contains the 20,000 most common English
//! words from the Google Books Ngram corpus, sorted by frequency.
//!
//! # API
//!
//! - [`is_english_word`]: Check if a lowercase word is in the wordlist
//!
//! # Binary Size
//!
//! The wordlist adds ~200 KB to the compiled binary (verified by CI gate).
//! If this exceeds 250 KB, the implementation should be replaced with a
//! Bloom filter (~25 KB for 20k words at 0.1% FPR).
//!
//! # Non-English Documents
//!
//! For documents with `/Lang` attribute indicating non-English (not matching
//! `en*`), the dictionary coverage signal is disabled (set to 1.0) and this
//! module is not used.

include!(concat!(env!("OUT_DIR"), "/wordlist.rs"));

/// Check if a word is in the English wordlist.
///
/// Lookup is case-insensitive: the input is lowercased before checking.
/// Non-ASCII characters return false (this wordlist is English-only).
///
/// # Arguments
///
/// * `s` - The word to check
///
/// # Returns
///
/// `true` if the lowercase word is in the 20k wordlist, `false` otherwise.
///
/// # Examples
///
/// ```
/// use pdftract_core::layout::wordlist::is_english_word;
///
/// assert!(is_english_word("the"));
/// assert!(is_english_word("THE"));  // case-insensitive
/// assert!(is_english_word("computer"));
/// assert!(!is_english_word("xyzqwerty"));
/// assert!(!is_english_word("café"));  // non-ASCII
/// ```
///
/// # Performance
///
/// O(1) lookup via phf's perfect hash function. Benchmark: < 100 ns per
/// call (see acceptance criteria).
pub fn is_english_word(s: &str) -> bool {
    // Lowercase for case-insensitive lookup
    let s_lower = s.to_lowercase();
    EN_WORDLIST_20K.contains(s_lower.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_words() {
        // Top frequency words from the wordlist
        assert!(is_english_word("the"));
        assert!(is_english_word("of"));
        assert!(is_english_word("and"));
        assert!(is_english_word("to"));
        assert!(is_english_word("a"));
        assert!(is_english_word("in"));
        assert!(is_english_word("is"));
        assert!(is_english_word("you"));
        assert!(is_english_word("that"));
        assert!(is_english_word("it"));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(is_english_word("The"));
        assert!(is_english_word("THE"));
        assert!(is_english_word("CoMpUtEr"));
    }

    #[test]
    fn test_not_in_wordlist() {
        assert!(!is_english_word("xyzqwerty"));
        assert!(!is_english_word("abcdefg"));
        assert!(!is_english_word("nonexistentword123"));
    }

    #[test]
    fn test_non_ascii_returns_false() {
        // Non-ASCII characters return false (English-only wordlist)
        assert!(!is_english_word("café"));
        assert!(!is_english_word("naïve"));
        assert!(!is_english_word("日本語"));
        assert!(!is_english_word("中文"));
    }

    #[test]
    fn test_inflected_forms() {
        // Common inflections should be present
        assert!(is_english_word("walked"));
        assert!(is_english_word("walking"));
        assert!(is_english_word("cats"));
        assert!(is_english_word("dogs"));
    }

    #[test]
    fn test_empty_string() {
        assert!(!is_english_word(""));
    }

    #[test]
    fn test_single_letter_words() {
        // Common single-letter words
        assert!(is_english_word("a"));
        assert!(is_english_word("i"));
    }

    #[test]
    fn test_medium_frequency_words() {
        // Words that should be in a 20k list
        assert!(is_english_word("computer"));
        assert!(is_english_word("program"));
        assert!(is_english_word("language"));
        assert!(is_english_word("document"));
        assert!(is_english_word("extract"));
    }

    #[test]
    fn test_lookup_timing() {
        // This is a smoke test, not a precise benchmark
        // The real benchmark is in benches/wordlist.rs
        use std::time::Instant;

        let words = vec!["the", "computer", "xyzqwerty", "document"];
        let iterations = 1000;

        let start = Instant::now();
        for _ in 0..iterations {
            for word in &words {
                is_english_word(word);
            }
        }
        let duration = start.elapsed();

        // 1000 iterations * 4 words = 4000 lookups
        // Should be well under 1 second even on slow machines
        assert!(
            duration.as_millis() < 1000,
            "lookup too slow: {:?}",
            duration
        );
    }
}
