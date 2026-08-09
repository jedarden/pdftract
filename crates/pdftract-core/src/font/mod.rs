//! Font type classification and subset prefix handling.
//!
//! This module provides utilities for classifying PDF fonts by type
//! and handling font subset prefixes.

pub mod agl;
pub mod cmap;
pub mod codespace;
pub mod embedded;
pub mod encoding;
pub mod fingerprint;
pub mod predefined_cmap;
pub mod resolver;
pub mod shape;
pub mod std14;
pub mod type0;
pub mod type3;
pub mod type3_rasterizer;
pub mod unmapped;

#[cfg(test)]
mod type3_rasterizer_test;

pub mod test_glyph_helper;
pub mod type3_test_fixtures;

pub use type3_test_fixtures::{
    create_charproc_stream_with_curves, create_empty_content_stream, create_main_content_stream,
    create_main_content_stream_multi, create_simple_charproc_stream,
};

#[cfg(feature = "cjk")]
pub mod cjk_encoding;

pub use agl::{unicode_for_glyph_name, unicode_for_glyph_name_multi};
pub use cmap::{parse_to_unicode, parse_to_unicode_with_diags, ToUnicodeMap};
pub use codespace::{
    parse_codespace_ranges, parse_codespace_ranges_with_diags, CodespaceRange, CodespaceRanges,
};
pub use embedded::{EmbeddedFont, EmptyFontMetrics, FontMetrics, GlyphBbox};
pub use encoding::{DifferencesOverlay, FontEncoding, NamedEncoding};
pub use fingerprint::{lookup_font_fingerprint, CachedFingerprint, FontFingerprint};
pub use predefined_cmap::{
    from_name as predefined_cmap_from_name, CharacterCollection, PredefinedCMap,
};
pub use resolver::{
    resolve_type3, resolve_unicode, Font, FontId, ResolvedGlyph, ResolverCache, UnicodeSource,
};
pub use shape::{hamming_distance, lookup_shape, phash_glyph, ShapeEntry, ShapeMatch};
pub use type0::{CIDToGIDMap, DescendantCIDFont, Type0Font};
pub use type3::Type3Font;
pub use type3_rasterizer::RasterizerContext;
pub use unmapped::{is_unmapped_glyph_name, UNMAPPED_GLYPH_NAMES};

#[cfg(feature = "cjk")]
pub use cjk_encoding::{decode_cjk_bytes, CjkEncoding};

use crate::parser::object::types::{PdfDict, PdfObject};

/// Font type classification.
///
/// Represents all font types defined in PDF 1.7 specification plus
/// OpenType with CFF fonts. Each variant maps to a specific loading
/// strategy and metric source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontKind {
    /// Type 1 font (non-Standard-14)
    Type1,
    /// Type 1 Standard 14 font (Times-Roman, Helvetica, Courier, Symbol, ZapfDingbats)
    Type1Std14,
    /// TrueType font
    TrueType,
    /// Type 0 composite font (has descendant CIDFont)
    Type0,
    /// CIDFontType0 (CFF-based CID font)
    CIDFontType0,
    /// CIDFontType2 (TrueType-based CID font)
    CIDFontType2,
    /// Type 3 font (bitmap/content-stream defined)
    Type3,
    /// OpenType font with CFF data
    OpenTypeCFF,
}

impl FontKind {
    /// Returns true if this is a Standard 14 font.
    pub fn is_standard_14(self) -> bool {
        matches!(self, FontKind::Type1Std14)
    }

    /// Returns true if this is a CID font (Type0, CIDFontType0, or CIDFontType2).
    pub fn is_cid_font(self) -> bool {
        matches!(
            self,
            FontKind::Type0 | FontKind::CIDFontType0 | FontKind::CIDFontType2
        )
    }

    /// Returns true if this is a Type 3 font.
    pub fn is_type3(self) -> bool {
        matches!(self, FontKind::Type3)
    }
}

/// Strip subset prefix from a font name.
///
/// PDFs often embed font subsets with a six-uppercase-letter prefix followed
/// by a `+` sign (e.g., `ABCDEF+Times-Roman`). This function removes such
/// prefixes.
///
/// The prefix must be **exactly** six ASCII uppercase letters followed by `+`.
/// Five-letter prefixes, lowercase letters, or other patterns are NOT subset
/// prefixes and will be returned unchanged.
///
/// # Examples
///
/// ```
/// use pdftract_core::font::strip_subset_prefix;
///
/// assert_eq!(strip_subset_prefix("ABCDEF+Times-Roman"), "Times-Roman");
/// assert_eq!(strip_subset_prefix("ABCD+Foo"), "ABCD+Foo"); // Too short
/// assert_eq!(strip_subset_prefix("abcdef+Foo"), "abcdef+Foo"); // Lowercase
/// assert_eq!(strip_subset_prefix("Times-Roman"), "Times-Roman"); // No prefix
/// ```
pub fn strip_subset_prefix(name: &str) -> &str {
    // A valid subset prefix is exactly 6 uppercase ASCII letters followed by '+'
    // Minimum length: 6 letters + 1 '+' + 1 char for actual name = 8 chars
    if name.len() < 8 {
        return name;
    }

    let bytes = name.as_bytes();

    // Check that chars 0..6 are all ASCII uppercase A-Z
    let is_all_uppercase = bytes[0..6].iter().all(|&b| b.is_ascii_uppercase());

    // Check that char 6 is '+'
    if is_all_uppercase && bytes[6] == b'+' {
        // Return the string after the prefix (starting at index 7)
        &name[7..]
    } else {
        name
    }
}

/// Standard 14 font names (PDF 1.7 specification).
const STANDARD_14_FONTS: &[&str] = &[
    "Times-Roman",
    "Times-Bold",
    "Times-Italic",
    "Times-BoldItalic",
    "Helvetica",
    "Helvetica-Bold",
    "Helvetica-Oblique",
    "Helvetica-BoldOblique",
    "Courier",
    "Courier-Bold",
    "Courier-Oblique",
    "Courier-BoldOblique",
    "Symbol",
    "ZapfDingbats",
];

/// Check if a font name (with or without subset prefix) is a Standard 14 font.
fn is_standard_14_font(name: &str) -> bool {
    let stripped = strip_subset_prefix(name);
    STANDARD_14_FONTS.contains(&stripped)
}

/// Classify a font from its font dictionary.
///
/// Reads `/Subtype`, `/BaseFont`, and (for Type0) descendant CIDFont subtype
/// to determine the font type.
///
/// # Arguments
///
/// * `font_dict` - The font dictionary from the PDF resource dictionary
///
/// # Returns
///
/// A `FontKind` enum value indicating the font type
///
/// # Classification Logic
///
/// 1. Read `/Subtype` to get the base font type
/// 2. For Type1 fonts, check if BaseFont matches Standard 14 names
/// 3. For Type0 fonts, read descendant CIDFont's `/Subtype`
/// 4. Check `/FontDescriptor` for `/FontFile3` with `/Subtype /OpenType` to distinguish OpenTypeCFF
pub fn classify_font(font_dict: &PdfDict) -> FontKind {
    // Get the /Subtype entry
    let subtype = font_dict
        .get("/Subtype")
        .and_then(|obj| obj.as_name())
        .unwrap_or("");

    // Strip leading slash from subtype for comparison
    let subtype_clean = if subtype.starts_with('/') {
        &subtype[1..]
    } else {
        subtype
    };

    match subtype_clean {
        "Type1" => {
            // Check if this is a Standard 14 font
            let base_font = font_dict
                .get("/BaseFont")
                .and_then(|obj| obj.as_name())
                .unwrap_or("");

            if is_standard_14_font(base_font) {
                FontKind::Type1Std14
            } else {
                FontKind::Type1
            }
        }
        "TrueType" => {
            // Check if this is actually OpenType CFF
            // Look for /FontDescriptor with /FontFile3 having /Subtype /OpenType
            if is_opentype_cff(font_dict) {
                FontKind::OpenTypeCFF
            } else {
                FontKind::TrueType
            }
        }
        "Type0" => {
            // Type0 fonts have a /DescendantFonts array
            // The descendant is a CIDFont; check its subtype
            if let Some(cidfont_kind) = get_descendant_cidfont_subtype(font_dict) {
                cidfont_kind
            } else {
                FontKind::Type0
            }
        }
        "CIDFontType0" => {
            // Check if this is actually OpenType CFF
            if is_opentype_cff(font_dict) {
                FontKind::OpenTypeCFF
            } else {
                FontKind::CIDFontType0
            }
        }
        "CIDFontType2" => FontKind::CIDFontType2,
        "Type3" => FontKind::Type3,
        // Default to Type1 for unknown subtypes (conservative fallback)
        _ => FontKind::Type1,
    }
}

/// Check if a font dictionary describes an OpenType CFF font.
///
/// Looks for `/FontDescriptor` with `/FontFile3` having `/Subtype /OpenType`.
fn is_opentype_cff(font_dict: &PdfDict) -> bool {
    // Get /FontDescriptor
    let font_descriptor = match font_dict.get("/FontDescriptor") {
        Some(PdfObject::Dict(d)) => d,
        Some(PdfObject::Ref(_)) => {
            // Indirect reference - would need resolution, skip for now
            return false;
        }
        _ => return false,
    };

    // Get /FontFile3 from FontDescriptor
    let font_file3 = match font_descriptor.get("/FontFile3") {
        Some(PdfObject::Stream(s)) => &s.dict,
        Some(PdfObject::Ref(_)) => {
            // Indirect reference - would need resolution, skip for now
            return false;
        }
        _ => return false,
    };

    // Check /Subtype of FontFile3 is /OpenType
    match font_file3.get("/Subtype") {
        Some(PdfObject::Name(name)) => {
            let name_str: &str = name.as_ref();
            // Strip leading slash
            let subtype = if name_str.starts_with('/') {
                &name_str[1..]
            } else {
                name_str
            };
            subtype == "OpenType"
        }
        _ => false,
    }
}

/// Get the CIDFont subtype from a Type0 font's descendant.
///
/// Type0 fonts have a `/DescendantFonts` array containing the CIDFont.
/// This function reads the descendant's `/Subtype` to determine if it's
/// CIDFontType0 or CIDFontType2.
fn get_descendant_cidfont_subtype(font_dict: &PdfDict) -> Option<FontKind> {
    // Get /DescendantFonts array
    let descendants = match font_dict.get("/DescendantFonts") {
        Some(PdfObject::Array(arr)) => arr.as_ref(),
        Some(PdfObject::Ref(_)) => {
            // Indirect reference - would need resolution
            return None;
        }
        _ => return None,
    };

    // Get the first descendant (Type0 fonts typically have one)
    let first_descendant = match descendants.first() {
        Some(PdfObject::Dict(d)) => d,
        Some(PdfObject::Ref(_)) => {
            // Indirect reference - would need resolution
            return None;
        }
        _ => return None,
    };

    // Get the descendant's /Subtype
    let subtype = first_descendant
        .get("/Subtype")
        .and_then(|obj| obj.as_name())?;

    // Strip leading slash
    let subtype_clean = if subtype.starts_with('/') {
        &subtype[1..]
    } else {
        subtype
    };

    match subtype_clean {
        "CIDFontType0" => Some(FontKind::CIDFontType0),
        "CIDFontType2" => Some(FontKind::CIDFontType2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::types::intern;

    #[test]
    fn test_strip_subset_prefix_valid() {
        assert_eq!(strip_subset_prefix("ABCDEF+Times-Roman"), "Times-Roman");
        assert_eq!(strip_subset_prefix("UVWXYZ+Helvetica"), "Helvetica");
        assert_eq!(strip_subset_prefix("ABC123+Crazy"), "ABC123+Crazy"); // Not all uppercase
    }

    #[test]
    fn test_strip_subset_prefix_too_short() {
        assert_eq!(strip_subset_prefix("ABCD+Foo"), "ABCD+Foo"); // Only 4 letters before +
        assert_eq!(strip_subset_prefix("ABCDE+Foo"), "ABCDE+Foo"); // Only 5 letters before +
        assert_eq!(strip_subset_prefix("A+B"), "A+B"); // Only 1 letter before +
    }

    #[test]
    fn test_strip_subset_prefix_lowercase() {
        assert_eq!(strip_subset_prefix("abcdef+Foo"), "abcdef+Foo"); // Lowercase letters
        assert_eq!(strip_subset_prefix("ABCDef+Foo"), "ABCDef+Foo"); // Mixed case
    }

    #[test]
    fn test_strip_subset_prefix_no_prefix() {
        assert_eq!(strip_subset_prefix("Times-Roman"), "Times-Roman");
        assert_eq!(strip_subset_prefix("Helvetica-Bold"), "Helvetica-Bold");
        assert_eq!(strip_subset_prefix("Courier"), "Courier");
    }

    #[test]
    fn test_strip_subset_prefix_empty() {
        assert_eq!(strip_subset_prefix(""), "");
        // No name after +: prefix is not stripped (invalid font name anyway)
        assert_eq!(strip_subset_prefix("ABCDEF+"), "ABCDEF+");
    }

    #[test]
    fn test_is_standard_14_font() {
        // Standard 14 fonts without prefix
        assert!(is_standard_14_font("Times-Roman"));
        assert!(is_standard_14_font("Times-Bold"));
        assert!(is_standard_14_font("Times-Italic"));
        assert!(is_standard_14_font("Times-BoldItalic"));
        assert!(is_standard_14_font("Helvetica"));
        assert!(is_standard_14_font("Helvetica-Bold"));
        assert!(is_standard_14_font("Helvetica-Oblique"));
        assert!(is_standard_14_font("Helvetica-BoldOblique"));
        assert!(is_standard_14_font("Courier"));
        assert!(is_standard_14_font("Courier-Bold"));
        assert!(is_standard_14_font("Courier-Oblique"));
        assert!(is_standard_14_font("Courier-BoldOblique"));
        assert!(is_standard_14_font("Symbol"));
        assert!(is_standard_14_font("ZapfDingbats"));

        // Standard 14 fonts with subset prefix
        assert!(is_standard_14_font("ABCDEF+Times-Roman"));
        assert!(is_standard_14_font("UVWXYZ+Helvetica-Bold"));

        // Non-standard fonts
        assert!(!is_standard_14_font("Arial"));
        assert!(!is_standard_14_font("Georgia"));
        assert!(!is_standard_14_font("Verdana"));
    }

    #[test]
    fn test_classify_font_type1_standard() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type1")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("Times-Roman")));

        assert_eq!(classify_font(&dict), FontKind::Type1Std14);
    }

    #[test]
    fn test_classify_font_type1_standard_with_subset() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type1")));
        dict.insert(
            intern("/BaseFont"),
            PdfObject::Name(intern("ABCDEF+Times-Roman")),
        );

        assert_eq!(classify_font(&dict), FontKind::Type1Std14);
    }

    #[test]
    fn test_classify_font_type1_non_standard() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type1")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("CustomFont")));

        assert_eq!(classify_font(&dict), FontKind::Type1);
    }

    #[test]
    fn test_classify_font_truetype() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/TrueType")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("Arial")));

        assert_eq!(classify_font(&dict), FontKind::TrueType);
    }

    #[test]
    fn test_classify_font_truetype_without_slash() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("TrueType")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("Arial")));

        assert_eq!(classify_font(&dict), FontKind::TrueType);
    }

    #[test]
    fn test_classify_font_type3() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type3")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("CustomType3")));

        assert_eq!(classify_font(&dict), FontKind::Type3);
    }

    #[test]
    fn test_classify_font_cidfonttype0() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType0")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("CIDFont0")));

        assert_eq!(classify_font(&dict), FontKind::CIDFontType0);
    }

    #[test]
    fn test_classify_font_cidfonttype2() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType2")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("CIDFont2")));

        assert_eq!(classify_font(&dict), FontKind::CIDFontType2);
    }

    #[test]
    fn test_classify_font_type0_with_cidfonttype0() {
        // Create descendant CIDFont dict
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType0")));

        // Create Type0 font dict with descendant
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("Type0Font")));
        dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        assert_eq!(classify_font(&dict), FontKind::CIDFontType0);
    }

    #[test]
    fn test_classify_font_type0_with_cidfonttype2() {
        // Create descendant CIDFont dict
        let mut cidfont_dict = PdfDict::new();
        cidfont_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/CIDFontType2")));

        // Create Type0 font dict with descendant
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("Type0Font")));
        dict.insert(
            intern("/DescendantFonts"),
            PdfObject::Array(Box::new(vec![PdfObject::Dict(Box::new(cidfont_dict))])),
        );

        assert_eq!(classify_font(&dict), FontKind::CIDFontType2);
    }

    #[test]
    fn test_classify_font_opentype_cff() {
        // Create FontFile3 stream dict with /Subtype /OpenType
        let mut font_file3_dict = PdfDict::new();
        font_file3_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/OpenType")));

        // Create FontDescriptor dict
        let mut font_descriptor = PdfDict::new();
        font_descriptor.insert(
            intern("/FontFile3"),
            PdfObject::Stream(Box::new(crate::parser::object::types::PdfStream {
                dict: font_file3_dict,
                offset: 0,
                len_hint: Some(1000),
            })),
        );

        // Create font dict
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/TrueType")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("OpenTypeFont")));
        dict.insert(
            intern("/FontDescriptor"),
            PdfObject::Dict(Box::new(font_descriptor)),
        );

        assert_eq!(classify_font(&dict), FontKind::OpenTypeCFF);
    }

    #[test]
    fn test_fontkind_is_standard_14() {
        assert!(FontKind::Type1Std14.is_standard_14());
        assert!(!FontKind::Type1.is_standard_14());
        assert!(!FontKind::TrueType.is_standard_14());
        assert!(!FontKind::Type0.is_standard_14());
    }

    #[test]
    fn test_fontkind_is_cid_font() {
        assert!(FontKind::Type0.is_cid_font());
        assert!(FontKind::CIDFontType0.is_cid_font());
        assert!(FontKind::CIDFontType2.is_cid_font());
        assert!(!FontKind::Type1.is_cid_font());
        assert!(!FontKind::TrueType.is_cid_font());
    }

    #[test]
    fn test_fontkind_is_type3() {
        assert!(FontKind::Type3.is_type3());
        assert!(!FontKind::Type1.is_type3());
        assert!(!FontKind::TrueType.is_type3());
    }

    #[test]
    fn test_strip_subset_prefix_unicode() {
        // Test that unicode handling works correctly
        assert_eq!(strip_subset_prefix("ABCDEF+Font-Name"), "Font-Name");
        assert_eq!(strip_subset_prefix("ABCDEF+Font_Name"), "Font_Name");
    }

    #[test]
    fn test_classify_font_unknown_subtype() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/UnknownType")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("SomeFont")));

        // Should default to Type1 for unknown subtypes
        assert_eq!(classify_font(&dict), FontKind::Type1);
    }

    #[test]
    fn test_classify_font_missing_subtype() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("SomeFont")));

        // Should default to Type1 when subtype is missing
        assert_eq!(classify_font(&dict), FontKind::Type1);
    }

    #[test]
    fn test_classify_font_type0_no_descendants() {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type0")));
        dict.insert(intern("/BaseFont"), PdfObject::Name(intern("Type0Font")));

        // Without descendants, should return Type0
        assert_eq!(classify_font(&dict), FontKind::Type0);
    }
}
