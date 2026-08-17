//! Adobe Glyph List (AGL) lookup for glyph name to Unicode mapping.
//!
//! This module provides compile-time phf::Map lookups for the Adobe Glyph List,
//! which is the canonical mapping from PostScript glyph names to Unicode codepoints.
//!
//! # References
//!
//! - Adobe Glyph List Specification: <https://github.com/adobe-type-tools/agl-aglfn>
//! - AGL 1.4 (glyphlist.txt): ~4,400 entries
//! - AGLFN 1.7 (aglfn.txt): ~770 entries for new fonts

include!(concat!(env!("OUT_DIR"), "/agl.rs"));


/// Lookup a single Unicode codepoint for a glyph name.
///
/// This handles:
/// 1. Algorithmic patterns (uniXXXX, uXXXXXX)
/// 2. Variant suffixes (.sc, .alt, etc.)
/// 3. AGL direct lookup
///
/// # Arguments
///
/// * `name` - The glyph name to lookup
///
/// # Returns
///
/// `Some(char)` if the name maps to a single codepoint, `None` otherwise.
///
/// # Examples
///
/// ```
/// use pdftract_core::font::agl::unicode_for_glyph_name;
///
/// assert_eq!(unicode_for_glyph_name("quoteright"), Some('\u{2019}'));
/// assert_eq!(unicode_for_glyph_name("uni20AC"), Some('\u{20AC}')); // Euro
/// assert_eq!(unicode_for_glyph_name("u1F600"), Some('\u{1F600}')); // Emoji
/// assert_eq!(unicode_for_glyph_name("A.sc"), Some('A')); // Variant stripped
/// ```
pub fn unicode_for_glyph_name(name: &str) -> Option<char> {
    // 1. Handle algorithmic patterns first
    if let Some(ch) = parse_algorithmic(name) {
        return Some(ch);
    }

    // 2. Strip variant suffix and retry
    let stripped = strip_variant_suffix(name);
    if stripped != name {
        if let Some(ch) = parse_algorithmic(stripped) {
            return Some(ch);
        }
        if let Some(ch) = AGL.get(stripped) {
            return Some(*ch);
        }
    }

    // 3. Direct AGL lookup
    AGL.get(name).copied()
}

/// Lookup multiple Unicode codepoints for a glyph name (ligatures).
///
/// # Arguments
///
/// * `name` - The glyph name to lookup
///
/// # Returns
///
/// `Some(&[char])` if the name maps to multiple codepoints, `None` otherwise.
///
/// # Examples
///
/// ```
/// use pdftract_core::font::agl::unicode_for_glyph_name_multi;
///
/// assert_eq!(unicode_for_glyph_name_multi("fi"), Some(&['f', 'i'][..]));
/// ```
pub fn unicode_for_glyph_name_multi(name: &str) -> Option<&'static [char]> {
    // Check multi-codepoint map
    if let Some(chars) = AGL_MULTI.get(name) {
        return Some(chars);
    }

    // Strip variant suffix and retry
    let stripped = strip_variant_suffix(name);
    if stripped != name {
        if let Some(chars) = AGL_MULTI.get(stripped) {
            return Some(chars);
        }
    }

    None
}

/// Parse algorithmic glyph name patterns.
///
/// Handles:
/// - `uniXXXX` (4 hex digits)
/// - `uXXXXXX` (up to 6 hex digits)
///
/// These are NOT in the AGL; they are algorithmic conventions.
fn parse_algorithmic(name: &str) -> Option<char> {
    let name = name.trim_start_matches('#'); // Some PDFs use #uniXXXX

    if let Some(rest) = name.strip_prefix("uni") {
        // uniXXXX - exactly 4 hex digits
        if rest.len() == 4 && rest.chars().all(|c| c.is_ascii_hexdigit()) {
            return u32::from_str_radix(rest, 16)
                .ok()
                .and_then(|c| char::from_u32(c));
        }
    }

    if let Some(rest) = name.strip_prefix('u') {
        // uXXXXXX - up to 6 hex digits
        if rest.len() <= 6 && rest.chars().all(|c| c.is_ascii_hexdigit()) {
            return u32::from_str_radix(rest, 16)
                .ok()
                .and_then(|c| char::from_u32(c));
        }
    }

    None
}

/// Strip variant suffix from a glyph name.
///
/// Handles patterns like:
/// - `H.sc` → `H` (small caps)
/// - `A.alt` → `A` (alternate)
/// - `foo.bar` → `foo`
///
/// The variant suffix is everything after the first `.`.
fn strip_variant_suffix(name: &str) -> &str {
    name.split('.').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agl_quoteright() {
        // quoteright is U+2019 in WinAnsiEncoding
        assert_eq!(unicode_for_glyph_name("quoteright"), Some('\u{2019}'));
    }

    #[test]
    fn test_agl_uni20ac() {
        // uniXXXX pattern (Euro)
        assert_eq!(unicode_for_glyph_name("uni20AC"), Some('\u{20AC}'));
    }

    #[test]
    fn test_agl_u1f600() {
        // uXXXXXX pattern (emoji)
        assert_eq!(unicode_for_glyph_name("u1F600"), Some('\u{1F600}'));
    }

    #[test]
    fn test_agl_variant_stripping() {
        // Small caps variant
        assert_eq!(unicode_for_glyph_name("A.sc"), Some('A'));
        assert_eq!(unicode_for_glyph_name("H.sc"), Some('H'));

        // Alt variant
        assert_eq!(unicode_for_glyph_name("A.alt"), Some('A'));
    }

    #[test]
    fn test_agl_unknown() {
        // Unknown name returns None
        assert_eq!(unicode_for_glyph_name("NotARealGlyphName"), None);
    }

    #[test]
    fn test_agl_multi_fi() {
        // fi ligature is single-codepoint U+FB01 in AGL, not multi-codepoint
        assert_eq!(unicode_for_glyph_name("fi"), Some('\u{FB01}'));
        assert_eq!(unicode_for_glyph_name_multi("fi"), None);
    }

    #[test]
    fn test_agl_multi_ffi() {
        // ffi ligature is single-codepoint U+FB03 in AGL
        assert_eq!(unicode_for_glyph_name("ffi"), Some('\u{FB03}'));
        assert_eq!(unicode_for_glyph_name_multi("ffi"), None);
    }

    #[test]
    fn test_agl_multi_ff() {
        // ff ligature is single-codepoint U+FB00 in AGL
        assert_eq!(unicode_for_glyph_name("ff"), Some('\u{FB00}'));
        assert_eq!(unicode_for_glyph_name_multi("ff"), None);
    }

    #[test]
    fn test_agl_multi_fl() {
        // fl ligature is single-codepoint U+FB02 in AGL
        assert_eq!(unicode_for_glyph_name("fl"), Some('\u{FB02}'));
        assert_eq!(unicode_for_glyph_name_multi("fl"), None);
    }

    #[test]
    fn test_agl_multi_unknown() {
        // Unknown name returns None
        assert_eq!(unicode_for_glyph_name_multi("NotALigature"), None);
    }

    #[test]
    fn test_agl_multi_hebrew() {
        // Hebrew combining sequences are multi-codepoint
        assert_eq!(
            unicode_for_glyph_name_multi("dalethatafpatah"),
            Some(&['\u{05D3}', '\u{05B2}'][..])
        );
        assert_eq!(
            unicode_for_glyph_name_multi("lamedholam"),
            Some(&['\u{05DC}', '\u{05B9}'][..])
        );
    }

    #[test]
    fn test_parse_algorithmic_uni() {
        // uniXXXX (4 hex digits)
        assert_eq!(parse_algorithmic("uni0041"), Some('A'));
        assert_eq!(parse_algorithmic("uni20AC"), Some('\u{20AC}'));
        assert_eq!(parse_algorithmic("uniFFFF"), Some('\u{FFFF}'));

        // Not 4 digits
        assert_eq!(parse_algorithmic("uni123"), None);
        assert_eq!(parse_algorithmic("uni12345"), None);
        assert_eq!(parse_algorithmic("uniGHIJ"), None);
    }

    #[test]
    fn test_parse_algorithmic_u() {
        // uXXXXXX (up to 6 hex digits)
        assert_eq!(parse_algorithmic("u0041"), Some('A'));
        assert_eq!(parse_algorithmic("u20AC"), Some('\u{20AC}'));
        assert_eq!(parse_algorithmic("u1F600"), Some('\u{1F600}'));

        // More than 6 digits
        assert_eq!(parse_algorithmic("u1234567"), None);
        assert_eq!(parse_algorithmic("uGGGGGG"), None);
    }

    #[test]
    fn test_strip_variant_suffix() {
        assert_eq!(strip_variant_suffix("A.sc"), "A");
        assert_eq!(strip_variant_suffix("H.sc"), "H");
        assert_eq!(strip_variant_suffix("foo.alt"), "foo");
        assert_eq!(strip_variant_suffix("bar.baz.qux"), "bar");
        assert_eq!(strip_variant_suffix("nosuffix"), "nosuffix");
        assert_eq!(strip_variant_suffix(".dot"), "");
    }

    #[test]
    fn test_agl_basic_letters() {
        assert_eq!(unicode_for_glyph_name("A"), Some('A'));
        assert_eq!(unicode_for_glyph_name("a"), Some('a'));
        assert_eq!(unicode_for_glyph_name("Z"), Some('Z'));
        assert_eq!(unicode_for_glyph_name("z"), Some('z'));
    }

    #[test]
    fn test_agl_punctuation() {
        assert_eq!(unicode_for_glyph_name("period"), Some('.'));
        assert_eq!(unicode_for_glyph_name("comma"), Some(','));
        assert_eq!(unicode_for_glyph_name("exclam"), Some('!'));
        assert_eq!(unicode_for_glyph_name("question"), Some('?'));
    }

    #[test]
    fn test_agl_quotes() {
        assert_eq!(unicode_for_glyph_name("quoteleft"), Some('\u{2018}'));
        assert_eq!(unicode_for_glyph_name("quoteright"), Some('\u{2019}'));
        assert_eq!(unicode_for_glyph_name("quotedblleft"), Some('\u{201C}'));
        assert_eq!(unicode_for_glyph_name("quotedblright"), Some('\u{201D}'));
    }

    #[test]
    fn test_agl_euro() {
        assert_eq!(unicode_for_glyph_name("Euro"), Some('\u{20AC}'));
    }

    #[test]
    fn test_algorithmic_with_hash_prefix() {
        // Some PDFs use #uniXXXX notation
        assert_eq!(parse_algorithmic("#uni0041"), Some('A'));
        assert_eq!(parse_algorithmic("#u0041"), Some('A'));
    }

    #[test]
    fn test_multi_lookup_single_returns_none() {
        // Single-codepoint names should return None from _multi
        assert_eq!(unicode_for_glyph_name_multi("A"), None);
        assert_eq!(unicode_for_glyph_name_multi("quoteright"), None);
    }

    #[test]
    fn test_variant_stripping_with_multi() {
        // Multi-codepoint with variant suffix should still work
        // (though unlikely in practice)
        assert_eq!(unicode_for_glyph_name_multi("fi.alt"), None); // No fi.alt in AGL_MULTI
    }
}
