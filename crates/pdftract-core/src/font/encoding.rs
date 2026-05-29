//! Named encoding tables for PDF Type1 fonts.
//!
//! This module provides the 6 standard named encodings defined in ISO 32000-1 Annex D:
//! - WinAnsiEncoding (Windows-1252 superset of StandardEncoding)
//! - MacRomanEncoding (Mac OS Roman encoding)
//! - MacExpertEncoding (Mac OS Expert character set)
//! - StandardEncoding (Adobe Standard encoding)
//! - SymbolEncoding (Symbol font encoding)
//! - ZapfDingbatsEncoding (Zapf Dingbats font encoding)
//!
//! These tables map character codes (0-255) to glyph names, which are then
//! mapped to Unicode via the Adobe Glyph List (AGL).

use std::sync::Arc;

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::object::types::{PdfDict, PdfObject};

include!(concat!(env!("OUT_DIR"), "/named_encodings.rs"));

/// Named encoding for Type1 fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedEncoding {
    /// WinAnsiEncoding (Windows-1252)
    ///
    /// This is the most common encoding in PDFs. It extends StandardEncoding
    /// with the "Windows" punctuation range at 0x80-0x9F (curly quotes, em dash,
    /// Euro, etc.). Code 0x92 maps to `quoteright` which maps to U+2019.
    WinAnsi,

    /// MacRomanEncoding (Mac OS Roman)
    ///
    /// The classic Mac OS encoding. Has different mappings for some punctuation
    /// characters compared to WinAnsi (e.g., 0xD2 = `quotedblleft`, 0xD3 = `quotedblright`).
    MacRoman,

    /// MacExpertEncoding (Mac OS Expert)
    ///
    /// Additional characters for expert typography (small caps, oldstyle figures,
    /// ligatures, Cyrillic characters).
    MacExpert,

    /// StandardEncoding (Adobe Standard)
    ///
    /// The default encoding for Type1 fonts when no /Encoding entry is present.
    /// This is the base from which other encodings are derived.
    Standard,

    /// SymbolEncoding (Symbol font)
    ///
    /// Maps to Symbol-font glyph names (alpha, beta, etc.) NOT Greek Unicode.
    /// The AGL handles Symbol -> Unicode mapping separately.
    Symbol,

    /// ZapfDingbatsEncoding (Zapf Dingbats font)
    ///
    /// Glyph names start with `a` followed by ZapfDingbats glyph numbers (a1..a202).
    /// The AGL has these mappings.
    ZapfDingbats,
}

impl NamedEncoding {
    /// Get the encoding table as a static array.
    ///
    /// Returns a reference to a 256-element array mapping character codes
    /// to glyph names (or None for unmapped codes).
    pub fn table(self) -> &'static [Option<&'static str>; 256] {
        get_named_encoding_table(self)
    }

    /// Parse a named encoding from a PDF /Encoding name.
    ///
    /// Handles both prefixed and unprefixed names (e.g., "WinAnsiEncoding"
    /// or "/WinAnsiEncoding"). Returns None for unknown encodings.
    ///
    /// # Examples
    ///
    /// ```
    /// use pdftract_core::font::encoding::NamedEncoding;
    ///
    /// assert_eq!(NamedEncoding::from_name("WinAnsiEncoding"), Some(NamedEncoding::WinAnsi));
    /// assert_eq!(NamedEncoding::from_name("/MacRomanEncoding"), Some(NamedEncoding::MacRoman));
    /// assert_eq!(NamedEncoding::from_name("UnknownEncoding"), None);
    /// ```
    pub fn from_name(name: &str) -> Option<Self> {
        // Strip leading slash if present
        let clean_name = if name.starts_with('/') {
            &name[1..]
        } else {
            name
        };

        match clean_name {
            "WinAnsiEncoding" => Some(NamedEncoding::WinAnsi),
            "MacRomanEncoding" => Some(NamedEncoding::MacRoman),
            "MacExpertEncoding" => Some(NamedEncoding::MacExpert),
            "StandardEncoding" => Some(NamedEncoding::Standard),
            "SymbolEncoding" => Some(NamedEncoding::Symbol),
            "ZapfDingbatsEncoding" => Some(NamedEncoding::ZapfDingbats),
            _ => None,
        }
    }

    /// Get the glyph name for a character code.
    ///
    /// Returns None if the code is not mapped in this encoding.
    pub fn glyph_name(self, code: u8) -> Option<&'static str> {
        self.table()[code as usize]
    }
}

/// Sparse overlay of glyph name assignments from a /Differences array.
///
/// The /Differences array sparsely overrides specific character codes with custom
/// glyph names on top of a base encoding. Format: `[n /Name1 /Name2 ... m /OtherName ...]`
/// where each integer resets the position and subsequent names are assigned to consecutive codes.
///
/// # Example
///
/// A Differences array `[ 39 /quotesingle 96 /grave ]` creates:
/// - code 39 → "quotesingle"
/// - code 96 → "grave"
///
/// # Lookup behavior
///
/// The overlay is sparse; most codes are not present. Use `get()` to check for an override,
/// which returns `None` either when the code is not in the overlay or when the code is out of range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DifferencesOverlay {
    /// Sparse list of (code, glyph_name) overrides.
    /// Sorted by code for binary search, though linear search is fine for <32 entries.
    entries: Vec<(u8, Arc<str>)>,
}

impl DifferencesOverlay {
    /// Create an empty overlay.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Parse a /Differences array into an overlay.
    ///
    /// The array alternates between integers (starting codes) and names (glyph names).
    /// Each integer resets the cursor, and subsequent names are assigned to consecutive codes.
    ///
    /// # Arguments
    ///
    /// * `diff_array` - The /Differences array from the font's Encoding dictionary
    /// * `diagnostics` - Diagnostic list for parsing errors
    ///
    /// # Returns
    ///
    /// A `DifferencesOverlay` with parsed entries. Invalid entries are skipped with diagnostics.
    ///
    /// # Example
    ///
    /// ```text
    /// // [ 39 /quotesingle 96 /grave ]
    /// // → entries: [(39, "quotesingle"), (96, "grave")]
    /// ```
    pub fn parse(diff_array: &PdfObject, diagnostics: &mut Vec<Diagnostic>) -> Self {
        let mut overlay = Self::new();

        let PdfObject::Array(arr) = diff_array else {
            return overlay;
        };

        let mut cursor: u32 = 0;

        for (i, obj) in arr.iter().enumerate() {
            match obj {
                PdfObject::Integer(code) => {
                    // Clamp to u8 range and emit diagnostic if out of range
                    if *code < 0 {
                        diagnostics.push(Diagnostic::with_dynamic_no_offset(
                            DiagCode::FontEncodingDifferenceOutOfRange,
                            format!("/Differences array at index {i} has negative integer {code}, clamping to 0"),
                        ));
                        cursor = 0;
                    } else if *code > 255 {
                        diagnostics.push(Diagnostic::with_dynamic_no_offset(
                            DiagCode::FontEncodingDifferenceOutOfRange,
                            format!("/Differences array at index {i} has code {code} > 255, clamping to 255"),
                        ));
                        cursor = 255;
                    } else {
                        cursor = *code as u32;
                    }
                }
                PdfObject::Name(name) => {
                    // Assign this name to the current cursor position
                    if cursor <= 255 {
                        overlay.entries.push((cursor as u8, Arc::clone(name)));
                    }
                    cursor = cursor.saturating_add(1);
                }
                _ => {
                    // Skip non-integer, non-name objects
                    // (this is technically a PDF spec violation, but we recover)
                }
            }
        }

        overlay
    }

    /// Get the glyph name override for a character code.
    ///
    /// Returns `Some(name)` if this code has an override, `None` otherwise.
    /// The returned name may not be in the AGL; the resolver must handle that.
    pub fn get(&self, code: u8) -> Option<Arc<str>> {
        // Linear search is fine for <32 entries; binary search for larger
        self.entries
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, name)| Arc::clone(name))
    }

    /// Check if the overlay has any entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the number of entries in the overlay.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl Default for DifferencesOverlay {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined font encoding with base encoding and /Differences overlay.
///
/// PDF font encodings are composed of:
/// 1. A base named encoding (WinAnsi, Standard, etc.) - optional
/// 2. A /Differences overlay that overrides specific codes - optional
///
/// When both are present, the overlay takes precedence. The lookup order is:
/// 1. Check /Differences overlay for an override
/// 2. Fall back to base encoding table
/// 3. Return None if neither has the code
///
/// # Default base encoding
///
/// When neither `/Encoding/BaseEncoding` nor `/Encoding` is present:
/// - Type1 fonts: StandardEncoding
/// - TrueType fonts: The font's built-in encoding (often MacRoman or WinAnsi)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontEncoding {
    /// Base named encoding (WinAnsi, Standard, etc.) - None for identity/implicit
    base: Option<NamedEncoding>,
    /// Sparse overrides from /Differences array
    differences: DifferencesOverlay,
}

impl FontEncoding {
    /// Create a new font encoding with the given base and empty differences.
    pub fn new(base: Option<NamedEncoding>) -> Self {
        Self {
            base,
            differences: DifferencesOverlay::new(),
        }
    }

    /// Create a font encoding by parsing the /Encoding dictionary from a font.
    ///
    /// This handles all the encoding indirection patterns:
    /// - `/Encoding` is a name → use that named encoding directly
    /// - `/Encoding` is a dict with `/BaseEncoding` → use base + /Differences
    /// - `/Encoding` is a dict without `/BaseEncoding` → use implicit base + /Differences
    /// - No `/Encoding` key → use default base (Standard for Type1, built-in for TrueType)
    ///
    /// # Arguments
    ///
    /// * `font_dict` - The font dictionary from the PDF resource dictionary
    /// * `default_base` - Default base encoding when /Encoding is absent (Standard for Type1)
    /// * `diagnostics` - Diagnostic list for parsing errors
    ///
    /// # Returns
    ///
    /// A `FontEncoding` with parsed base encoding and differences overlay.
    pub fn parse_from_font(
        font_dict: &PdfDict,
        default_base: Option<NamedEncoding>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Self {
        // Get the /Encoding entry
        let encoding_obj = match font_dict.get("/Encoding") {
            Some(obj) => obj,
            None => return Self::new(default_base),
        };

        match encoding_obj {
            // Case 1: /Encoding is a name → use that named encoding directly
            PdfObject::Name(name) => {
                let base = NamedEncoding::from_name(name.as_ref());
                Self::new(base.or(default_base))
            }

            // Case 2: /Encoding is a dict → read /BaseEncoding and /Differences
            PdfObject::Dict(encoding_dict) => {
                // Parse /BaseEncoding (if present)
                let base = encoding_dict
                    .get("/BaseEncoding")
                    .and_then(|obj| obj.as_name())
                    .and_then(|name| NamedEncoding::from_name(name.as_ref()))
                    .or(default_base);

                // Parse /Differences (if present)
                let differences = encoding_dict
                    .get("/Differences")
                    .map(|diff| DifferencesOverlay::parse(diff, diagnostics))
                    .unwrap_or_default();

                Self { base, differences }
            }

            // Case 3: /Encoding is an indirect reference → would need resolution
            // For now, treat as missing and use default
            PdfObject::Ref(_) => Self::new(default_base),

            // Invalid /Encoding type → use default
            _ => Self::new(default_base),
        }
    }

    /// Get the glyph name for a character code.
    ///
    /// Lookup order:
    /// 1. Check /Differences overlay for an override
    /// 2. Fall back to base encoding table
    /// 3. Return None if neither has the code
    ///
    /// Returns `Some(name)` if found, `None` if not mapped.
    /// The returned name may not be in the AGL; the resolver must handle that.
    pub fn glyph_name_for(&self, code: u8) -> Option<Arc<str>> {
        // Check differences overlay first
        if let Some(name) = self.differences.get(code) {
            return Some(name);
        }

        // Fall back to base encoding
        self.base
            .and_then(|enc| enc.glyph_name(code).map(|s| Arc::from(s)))
    }

    /// Check if this encoding has a differences overlay.
    pub fn has_differences(&self) -> bool {
        !self.differences.is_empty()
    }

    /// Get the base encoding.
    pub fn base_encoding(&self) -> Option<NamedEncoding> {
        self.base
    }

    /// Get a reference to the differences overlay.
    pub fn differences(&self) -> &DifferencesOverlay {
        &self.differences
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winansi_0x92_quoteright() {
        let enc = NamedEncoding::WinAnsi;
        assert_eq!(enc.glyph_name(0x92), Some("quoteright"));
    }

    #[test]
    fn test_macroman_0xd2_quotedblleft() {
        let enc = NamedEncoding::MacRoman;
        assert_eq!(enc.glyph_name(0xD2), Some("quotedblleft"));
        assert_eq!(enc.glyph_name(0xD3), Some("quotedblright"));
    }

    #[test]
    fn test_standard_0x20_space() {
        let enc = NamedEncoding::Standard;
        assert_eq!(enc.glyph_name(0x20), Some("space"));
    }

    #[test]
    fn test_from_name() {
        assert_eq!(
            NamedEncoding::from_name("WinAnsiEncoding"),
            Some(NamedEncoding::WinAnsi)
        );
        assert_eq!(
            NamedEncoding::from_name("MacRomanEncoding"),
            Some(NamedEncoding::MacRoman)
        );
        assert_eq!(
            NamedEncoding::from_name("MacExpertEncoding"),
            Some(NamedEncoding::MacExpert)
        );
        assert_eq!(
            NamedEncoding::from_name("StandardEncoding"),
            Some(NamedEncoding::Standard)
        );
        assert_eq!(
            NamedEncoding::from_name("SymbolEncoding"),
            Some(NamedEncoding::Symbol)
        );
        assert_eq!(
            NamedEncoding::from_name("ZapfDingbatsEncoding"),
            Some(NamedEncoding::ZapfDingbats)
        );

        // Test with leading slash
        assert_eq!(
            NamedEncoding::from_name("/WinAnsiEncoding"),
            Some(NamedEncoding::WinAnsi)
        );

        // Test unknown encoding
        assert_eq!(NamedEncoding::from_name("UnknownEncoding"), None);
    }

    #[test]
    fn test_table_length() {
        let enc = NamedEncoding::WinAnsi;
        assert_eq!(enc.table().len(), 256);
    }

    #[test]
    fn test_winansi_euro_at_0x80() {
        let enc = NamedEncoding::WinAnsi;
        assert_eq!(enc.glyph_name(0x80), Some("Euro"));
    }

    #[test]
    fn test_symbol_encoding_alpha() {
        let enc = NamedEncoding::Symbol;
        assert_eq!(enc.glyph_name(0x41), Some("Alpha"));
        assert_eq!(enc.glyph_name(0x61), Some("alpha"));
    }

    #[test]
    fn test_zapfdingbats_a1() {
        let enc = NamedEncoding::ZapfDingbats;
        assert_eq!(enc.glyph_name(0x21), Some("a1"));
        assert_eq!(enc.glyph_name(0xFF), Some("a222"));
    }

    #[test]
    fn test_unmapped_codes() {
        let enc = NamedEncoding::Standard;
        // Most codes 0x80-0x9F are unmapped in StandardEncoding
        assert_eq!(enc.glyph_name(0x80), None);
        assert_eq!(enc.glyph_name(0x92), None); // WinAnsi has this, Standard doesn't
    }

    // === DifferencesOverlay tests ===

    #[test]
    fn test_differences_overlay_parse_simple() {
        // [ 39 /quotesingle 96 /grave ]
        let mut diagnostics = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(39),
            PdfObject::Name(Arc::from("quotesingle")),
            PdfObject::Integer(96),
            PdfObject::Name(Arc::from("grave")),
        ]));

        let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

        assert_eq!(overlay.get(39), Some(Arc::from("quotesingle")));
        assert_eq!(overlay.get(96), Some(Arc::from("grave")));
        assert_eq!(overlay.get(40), None);
        assert_eq!(overlay.len(), 2);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_differences_overlay_parse_consecutive() {
        // [ 39 /a /b /c ]
        let mut diagnostics = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(39),
            PdfObject::Name(Arc::from("a")),
            PdfObject::Name(Arc::from("b")),
            PdfObject::Name(Arc::from("c")),
        ]));

        let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

        assert_eq!(overlay.get(39), Some(Arc::from("a")));
        assert_eq!(overlay.get(40), Some(Arc::from("b")));
        assert_eq!(overlay.get(41), Some(Arc::from("c")));
        assert_eq!(overlay.get(42), None);
        assert_eq!(overlay.len(), 3);
    }

    #[test]
    fn test_differences_overlay_parse_multiple_blocks() {
        // [ 39 /a /b 100 /x /y ]
        let mut diagnostics = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(39),
            PdfObject::Name(Arc::from("a")),
            PdfObject::Name(Arc::from("b")),
            PdfObject::Integer(100),
            PdfObject::Name(Arc::from("x")),
            PdfObject::Name(Arc::from("y")),
        ]));

        let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

        assert_eq!(overlay.get(39), Some(Arc::from("a")));
        assert_eq!(overlay.get(40), Some(Arc::from("b")));
        assert_eq!(overlay.get(100), Some(Arc::from("x")));
        assert_eq!(overlay.get(101), Some(Arc::from("y")));
        assert_eq!(overlay.len(), 4);
    }

    #[test]
    fn test_differences_overlay_out_of_range_positive() {
        // Code > 255 should emit diagnostic and clamp
        let mut diagnostics = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(300),
            PdfObject::Name(Arc::from("a")),
        ]));

        let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

        assert_eq!(overlay.get(255), Some(Arc::from("a")));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            DiagCode::FontEncodingDifferenceOutOfRange
        );
    }

    #[test]
    fn test_differences_overlay_out_of_range_negative() {
        // Negative code should emit diagnostic and clamp to 0
        let mut diagnostics = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(-5),
            PdfObject::Name(Arc::from("a")),
        ]));

        let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

        assert_eq!(overlay.get(0), Some(Arc::from("a")));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            DiagCode::FontEncodingDifferenceOutOfRange
        );
    }

    #[test]
    fn test_differences_overlay_empty() {
        let mut diagnostics = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![]));

        let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

        assert!(overlay.is_empty());
        assert_eq!(overlay.len(), 0);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_differences_overlay_default() {
        let overlay = DifferencesOverlay::default();
        assert!(overlay.is_empty());
        assert_eq!(overlay.get(0), None);
    }

    // === FontEncoding tests ===

    #[test]
    fn test_font_encoding_new() {
        let enc = FontEncoding::new(Some(NamedEncoding::WinAnsi));
        assert_eq!(enc.base_encoding(), Some(NamedEncoding::WinAnsi));
        assert!(!enc.has_differences());
    }

    #[test]
    fn test_font_encoding_glyph_name_base_only() {
        let enc = FontEncoding::new(Some(NamedEncoding::WinAnsi));
        assert_eq!(enc.glyph_name_for(0x92), Some(Arc::from("quoteright")));
        assert_eq!(enc.glyph_name_for(0x80), Some(Arc::from("Euro")));
    }

    #[test]
    fn test_font_encoding_glyph_name_with_differences() {
        // Base encoding has 0x92 = quoteright, but difference overrides it
        let mut differences = DifferencesOverlay::new();
        differences.entries.push((0x92, Arc::from("customquote")));

        let enc = FontEncoding {
            base: Some(NamedEncoding::WinAnsi),
            differences,
        };

        assert_eq!(enc.glyph_name_for(0x92), Some(Arc::from("customquote")));
        // Non-overlaid codes still use base
        assert_eq!(enc.glyph_name_for(0x80), Some(Arc::from("Euro")));
    }

    #[test]
    fn test_font_encoding_glyph_name_no_base() {
        // No base encoding, only differences
        let mut differences = DifferencesOverlay::new();
        differences.entries.push((0x20, Arc::from("space")));

        let enc = FontEncoding {
            base: None,
            differences,
        };

        assert_eq!(enc.glyph_name_for(0x20), Some(Arc::from("space")));
        assert_eq!(enc.glyph_name_for(0x21), None); // Not in differences, no base
    }

    #[test]
    fn test_font_encoding_unknown_glyph_name() {
        // Differences can contain arbitrary glyph names not in AGL
        let mut differences = DifferencesOverlay::new();
        differences
            .entries
            .push((0x20, Arc::from("ArbitraryCustomGlyph")));

        let enc = FontEncoding {
            base: None,
            differences,
        };

        // Should return the custom name, not None
        assert_eq!(
            enc.glyph_name_for(0x20),
            Some(Arc::from("ArbitraryCustomGlyph"))
        );
    }

    #[test]
    fn test_font_encoding_lookup_order() {
        // Differences should take precedence over base encoding
        let mut differences = DifferencesOverlay::new();
        // WinAnsi has 0x92 = quoteright, override it
        differences.entries.push((0x92, Arc::from("override")));

        let enc = FontEncoding {
            base: Some(NamedEncoding::WinAnsi),
            differences,
        };

        assert_eq!(enc.glyph_name_for(0x92), Some(Arc::from("override")));
        // Base encoding still works for non-overlaid codes
        assert_eq!(enc.glyph_name_for(0x80), Some(Arc::from("Euro")));
    }
}
