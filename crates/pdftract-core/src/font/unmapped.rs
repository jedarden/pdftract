//! Unmapped glyph name configuration.
//!
//! This module defines the set of glyph names that should be skipped during
//! CMAP and ToUnicode entry creation. These glyphs have no valid Unicode mapping
//! and should not appear in text extraction output.

use std::collections::HashSet;
use std::sync::LazyLock;

/// Set of glyph names that are known to be unmapped and should be skipped during
/// CMAP and ToUnicode entry creation.
///
/// This includes:
/// - `.notdef`: The special fallback glyph defined by PDF and font specifications
///   that represents "no glyph available". It has no Unicode mapping and should be
///   skipped to prevent it from appearing in text extraction output.
///
/// Additional unmapped glyph names can be added to this set as needed.
pub static UNMAPPED_GLYPH_NAMES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    set.insert(".notdef");
    // Additional unmapped glyph names can be added here
    // e.g., set.insert("g001");
    // e.g., set.insert("g002");
    set
});

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
        assert!(is_unmapped_glyph_name(".notdef"));
        assert!(is_unmapped_glyph_name("/.notdef"));
    }

    #[test]
    fn test_normal_glyphs_not_unmapped() {
        assert!(!is_unmapped_glyph_name("A"));
        assert!(!is_unmapped_glyph_name("/A"));
        assert!(!is_unmapped_glyph_name("space"));
        assert!(!is_unmapped_glyph_name("/space"));
        assert!(!is_unmapped_glyph_name("uni0041"));
        assert!(!is_unmapped_glyph_name("/uni0041"));
    }

    #[test]
    fn test_unmapped_set_contains_expected_entries() {
        assert!(UNMAPPED_GLYPH_NAMES.contains(".notdef"));
    }
}
