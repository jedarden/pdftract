//! Unmapped glyph name configuration.
//!
//! This module defines the set of glyph names that should be skipped during
//! CMAP and ToUnicode entry creation. These glyphs have no valid Unicode mapping
//! and should not appear in text extraction output.

// Include auto-generated unmapped glyph names from build.rs
// This defines UNMAPPED_GLYPH_NAMES as a LazyLock<HashSet<&'static str>>
include!(concat!(env!("OUT_DIR"), "/unmapped_glyph_names.rs"));

/// Check if a glyph name is in the unmapped glyph names set.
///
/// # Arguments
///
/// * `name` - The glyph name to check (with or without leading `/`)
///
/// # Returns
///
/// `true` if the glyph name is in the unmapped set, `false` otherwise.
///
/// # Examples
///
/// ```
/// use pdftract_core::font::unmapped::is_unmapped_glyph_name;
///
/// assert!(is_unmapped_glyph_name(".notdef"));
/// assert!(is_unmapped_glyph_name("/.notdef"));
/// assert!(!is_unmapped_glyph_name("A"));
/// assert!(!is_unmapped_glyph_name("space"));
/// ```
pub fn is_unmapped_glyph_name(name: &str) -> bool {
    // Strip leading slash if present
    let clean_name = if name.starts_with('/') {
        &name[1..]
    } else {
        name
    };
    UNMAPPED_GLYPH_NAMES.contains(clean_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notdef_is_unmapped() {
        assert!(
            is_unmapped_glyph_name(".notdef"),
            ".notdef should be recognized as an unmapped glyph name. \
             Expected: is_unmapped_glyph_name(\".notdef\") == true. \
             Found: false. \
             Why this matters: .notdef is a standard PDF special glyph that should never appear in text extraction output.",
        );
        assert!(
            is_unmapped_glyph_name("/.notdef"),
            "/.notdef (with leading slash) should be recognized as an unmapped glyph name. \
             Expected: is_unmapped_glyph_name(\"/.notdef\") == true. \
             Found: false. \
             Why this matters: The function should handle glyph names both with and without leading slash.",
        );
    }

    #[test]
    fn test_normal_glyphs_not_unmapped() {
        assert!(
            !is_unmapped_glyph_name("A"),
            "Normal glyph 'A' should not be recognized as unmapped. \
             Expected: is_unmapped_glyph_name(\"A\") == false. \
             Found: true. \
             Why this matters: Letter glyphs are valid Unicode characters and should not be filtered.",
        );
        assert!(
            !is_unmapped_glyph_name("/A"),
            "Normal glyph '/A' (with leading slash) should not be recognized as unmapped. \
             Expected: is_unmapped_glyph_name(\"/A\") == false. \
             Found: true. \
             Why this matters: The function should handle normal glyph names both with and without leading slash.",
        );
        assert!(
            !is_unmapped_glyph_name("space"),
            "Normal glyph 'space' should not be recognized as unmapped. \
             Expected: is_unmapped_glyph_name(\"space\") == false. \
             Found: true. \
             Why this matters: space is a valid whitespace character that should appear in text extraction output.",
        );
        assert!(
            !is_unmapped_glyph_name("/space"),
            "Normal glyph '/space' (with leading slash) should not be recognized as unmapped. \
             Expected: is_unmapped_glyph_name(\"/space\") == false. \
             Found: true. \
             Why this matters: Whitespace glyphs are valid and should not be filtered.",
        );
        assert!(
            !is_unmapped_glyph_name("uni0041"),
            "Normal glyph 'uni0041' should not be recognized as unmapped. \
             Expected: is_unmapped_glyph_name(\"uni0041\") == false. \
             Found: true. \
             Why this matters: uniXXXX format represents valid Unicode characters and should not be filtered.",
        );
        assert!(
            !is_unmapped_glyph_name("/uni0041"),
            "Normal glyph '/uni0041' (with leading slash) should not be recognized as unmapped. \
             Expected: is_unmapped_glyph_name(\"/uni0041\") == false. \
             Found: true. \
             Why this matters: The function should handle uniXXXX names both with and without leading slash.",
        );
    }

    #[test]
    fn test_unmapped_set_contains_expected_entries() {
        assert!(
            UNMAPPED_GLYPH_NAMES.contains(".notdef"),
            "UNMAPPED_GLYPH_NAMES set should contain '.notdef'. \
             Expected: UNMAPPED_GLYPH_NAMES.contains(\".notdef\") == true. \
             Found: false. \
             Why this matters: .notdef is a core unmapped glyph defined in build/unmapped-glyph-names.json configuration.",
        );
    }
}
