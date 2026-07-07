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
    /// Glyph names that should be skipped during CMAP entry creation.
    /// These glyphs have no valid Unicode mapping and should not appear in text extraction.
    /// Defaults to the global UNMAPPED_GLYPH_NAMES set if not explicitly configured.
    unmapped_glyph_names: std::collections::HashSet<String>,
}

impl DifferencesOverlay {
    /// Create an empty overlay with default unmapped glyph names.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            unmapped_glyph_names: Self::default_unmapped_glyph_names(),
        }
    }

    /// Create an empty overlay with custom unmapped glyph names.
    ///
    /// # Arguments
    ///
    /// * `unmapped_glyph_names` - Set of glyph names to skip during CMAP entry creation
    pub fn with_unmapped_glyph_names(unmapped_glyph_names: std::collections::HashSet<String>) -> Self {
        Self {
            entries: Vec::new(),
            unmapped_glyph_names,
        }
    }

    /// Get the default set of unmapped glyph names.
    ///
    /// Returns the global UNMAPPED_GLYPH_NAMES set as a HashSet<String>.
    fn default_unmapped_glyph_names() -> std::collections::HashSet<String> {
        crate::font::unmapped::UNMAPPED_GLYPH_NAMES
            .iter()
            .map(|s| s.to_string())
            .collect()
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
                    // MARKER: CMAP entry creation point - Type1 font encoding differences.
                    // See notes/bf-e4uvb-child-1.md for documentation.
                    if cursor <= 255 {
                        // Skip unmapped glyph names (e.g., .notdef) to prevent them from
                        // appearing in text extraction output. These glyphs have no valid
                        // Unicode mapping and should emit GLYPH_UNMAPPED diagnostics instead.
                        if !overlay.is_unmapped_glyph_name(&name) {
                            overlay.entries.push((cursor as u8, Arc::clone(name)));
                        }
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

    /// Check if a glyph name is in the unmapped glyph names set.
    ///
    /// # Arguments
    ///
    /// * `name` - The glyph name to check (with or without leading `/`)
    ///
    /// # Returns
    ///
    /// `true` if the glyph name is in the unmapped set, `false` otherwise.
    fn is_unmapped_glyph_name(&self, name: &str) -> bool {
        // Strip leading slash if present
        let clean_name = if name.starts_with('/') {
            &name[1..]
        } else {
            name
        };
        self.unmapped_glyph_names.contains(clean_name)
    }

    /// Get a reference to the unmapped glyph names set.
    pub fn unmapped_glyph_names(&self) -> &std::collections::HashSet<String> {
        &self.unmapped_glyph_names
    }

    /// Set the unmapped glyph names set.
    ///
    /// # Arguments
    ///
    /// * `unmapped_glyph_names` - New set of glyph names to skip during CMAP entry creation
    pub fn set_unmapped_glyph_names(&mut self, unmapped_glyph_names: std::collections::HashSet<String>) {
        self.unmapped_glyph_names = unmapped_glyph_names;
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
        assert_eq!(
            enc.glyph_name(0x80),
            None,
            "StandardEncoding code 0x80 should be unmapped. \
             Expected: None. \
             Found: {:?}. \
             Why this matters: Most codes in StandardEncoding above 0x7F are unmapped and should fail Level 2 (encoding + AGL) resolution per build/unmapped-glyph-names.json filtering.",
            enc.glyph_name(0x80)
        );
        assert_eq!(
            enc.glyph_name(0x92),
            None,
            "StandardEncoding code 0x92 should be unmapped. \
             Expected: None. \
             Found: {:?}. \
             Why this matters: Code 0x92 is mapped in WinAnsi (quoteright) but unmapped in StandardEncoding - this test verifies the encoding tables are correctly differentiated.",
            enc.glyph_name(0x92)
        );
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

        assert_eq!(
            overlay.get(39),
            Some(Arc::from("quotesingle")),
            "Code 39 should map to quotesingle glyph. Expected: Some(\"quotesingle\"). Found: {:?}. Why: /Differences array [ 39 /quotesingle ] should create this mapping.",
            overlay.get(39)
        );
        assert_eq!(
            overlay.get(96),
            Some(Arc::from("grave")),
            "Code 96 should map to grave glyph. Expected: Some(\"grave\"). Found: {:?}. Why: /Differences array [ 96 /grave ] should create this mapping.",
            overlay.get(96)
        );
        assert_eq!(
            overlay.get(40),
            None,
            "Code 40 should not have a mapping. Expected: None (not defined in /Differences). Found: {:?}. Why: Only codes 39 and 96 are defined in this test.",
            overlay.get(40)
        );
        assert_eq!(
            overlay.len(),
            2,
            "Overlay should contain exactly 2 entries. Expected: 2 entries (quotesingle at 39, grave at 96). Found: {} entries. Why: /Differences array defines only 2 code-glyph pairs.",
            overlay.len()
        );
        assert!(
            diagnostics.is_empty(),
            "Parsing should not generate diagnostics. Expected: empty diagnostics vector. Found: {} diagnostics. Why: /Differences array [ 39 /quotesingle 96 /grave ] is well-formed.",
            diagnostics.len()
        );
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

        assert_eq!(
            overlay.get(39),
            Some(Arc::from("a")),
            "Code 39 should map to 'a'. Expected: Some(\"a\"). Found: {:?}. Why: /Differences [ 39 /a /b /c ] assigns 'a' to starting code 39.",
            overlay.get(39)
        );
        assert_eq!(
            overlay.get(40),
            Some(Arc::from("b")),
            "Code 40 should map to 'b'. Expected: Some(\"b\"). Found: {:?}. Why: /Differences consecutive sequence auto-increments code: 39→a, 40→b, 41→c.",
            overlay.get(40)
        );
        assert_eq!(
            overlay.get(41),
            Some(Arc::from("c")),
            "Code 41 should map to 'c'. Expected: Some(\"c\"). Found: {:?}. Why: /Differences consecutive sequence auto-increments code: 39→a, 40→b, 41→c.",
            overlay.get(41)
        );
        assert_eq!(
            overlay.get(42),
            None,
            "Code 42 should not have a mapping. Expected: None (no glyph defined at this code). Found: {:?}. Why: /Differences array has only 3 names after code 39, covering codes 39-41.",
            overlay.get(42)
        );
        assert_eq!(
            overlay.len(),
            3,
            "Overlay should contain exactly 3 entries. Expected: 3 consecutive entries (a at 39, b at 40, c at 41). Found: {} entries. Why: /Differences [ 39 /a /b /c ] creates 3 mappings.",
            overlay.len()
        );
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

        assert_eq!(
            overlay.get(39),
            Some(Arc::from("a")),
            "Code 39 should map to 'a'. Expected: Some(\"a\"). Found: {:?}. Why: First block starts at 39 with /a /b.",
            overlay.get(39)
        );
        assert_eq!(
            overlay.get(40),
            Some(Arc::from("b")),
            "Code 40 should map to 'b'. Expected: Some(\"b\"). Found: {:?}. Why: Consecutive auto-increment from 39.",
            overlay.get(40)
        );
        assert_eq!(
            overlay.get(100),
            Some(Arc::from("x")),
            "Code 100 should map to 'x'. Expected: Some(\"x\"). Found: {:?}. Why: Second block starts at 100 with /x /y.",
            overlay.get(100)
        );
        assert_eq!(
            overlay.get(101),
            Some(Arc::from("y")),
            "Code 101 should map to 'y'. Expected: Some(\"y\"). Found: {:?}. Why: Consecutive auto-increment from 100.",
            overlay.get(101)
        );
        assert_eq!(
            overlay.len(),
            4,
            "Overlay should contain exactly 4 entries. Expected: 4 entries from 2 blocks (39→a, 40→b, 100→x, 101→y). Found: {} entries. Why: /Differences [ 39 /a /b 100 /x /y ] creates 2 separate blocks.",
            overlay.len()
        );
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

        assert_eq!(
            overlay.get(255),
            Some(Arc::from("a")),
            "Code 255 should map to 'a' (clamped from out-of-range). Expected: Some(\"a\") at clamped code 255. Found: {:?}. Why: Code 300 exceeds u8 max (255), so parser clamps to 255.",
            overlay.get(255)
        );
        assert_eq!(
            diagnostics.len(),
            1,
            "Should generate 1 diagnostic for out-of-range code. Expected: 1 diagnostic. Found: {} diagnostics. Why: Code 300 > 255 triggers FontEncodingDifferenceOutOfRange warning.",
            diagnostics.len()
        );
        assert_eq!(
            diagnostics[0].code,
            DiagCode::FontEncodingDifferenceOutOfRange,
            "Diagnostic should have FontEncodingDifferenceOutOfRange code. Expected: DiagCode::FontEncodingDifferenceOutOfRange. Found: {:?}. Why: Out-of-range codes generate this specific diagnostic.",
            diagnostics[0].code
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

        assert_eq!(
            overlay.get(0),
            Some(Arc::from("a")),
            "Code 0 should map to 'a' (clamped from negative). Expected: Some(\"a\") at clamped code 0. Found: {:?}. Why: Negative code -5 is below u8 min (0), so parser clamps to 0.",
            overlay.get(0)
        );
        assert_eq!(
            diagnostics.len(),
            1,
            "Should generate 1 diagnostic for negative code. Expected: 1 diagnostic. Found: {} diagnostics. Why: Code -5 < 0 triggers FontEncodingDifferenceOutOfRange warning.",
            diagnostics.len()
        );
        assert_eq!(
            diagnostics[0].code,
            DiagCode::FontEncodingDifferenceOutOfRange,
            "Diagnostic should have FontEncodingDifferenceOutOfRange code. Expected: DiagCode::FontEncodingDifferenceOutOfRange. Found: {:?}. Why: Out-of-range codes (even negative) generate this specific diagnostic.",
            diagnostics[0].code
        );
    }

    #[test]
    fn test_differences_overlay_empty() {
        let mut diagnostics = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![]));

        let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

        assert!(
            overlay.is_empty(),
            "Empty /Differences array should produce empty overlay. Expected: is_empty() == true. Found: overlay with {} entries. Why: No code-glyph pairs to parse.",
            overlay.len()
        );
        assert_eq!(
            overlay.len(),
            0,
            "Empty overlay should have zero entries. Expected: 0 entries. Found: {} entries. Why: Empty input array produces no mappings.",
            overlay.len()
        );
        assert!(
            diagnostics.is_empty(),
            "Empty array should not generate diagnostics. Expected: empty diagnostics. Found: {} diagnostics. Why: Empty input is valid, not an error.",
            diagnostics.len()
        );
    }

    #[test]
    fn test_differences_overlay_default() {
        let overlay = DifferencesOverlay::default();
        assert!(
            overlay.is_empty(),
            "Default overlay should be empty. Expected: is_empty() == true. Found: overlay with {} entries. Why: Default constructor creates empty overlay.",
            overlay.len()
        );
        assert_eq!(
            overlay.get(0),
            None,
            "Default overlay should have no mappings. Expected: None for code 0. Found: {:?}. Why: No entries in default overlay.",
            overlay.get(0)
        );
    }

    // === FontEncoding tests ===

    #[test]
    fn test_font_encoding_new() {
        let enc = FontEncoding::new(Some(NamedEncoding::WinAnsi));
        assert_eq!(
            enc.base_encoding(),
            Some(NamedEncoding::WinAnsi),
            "FontEncoding should store the provided base encoding. Expected: Some(NamedEncoding::WinAnsi). Found: {:?}. Why: Constructor should preserve the base encoding parameter.",
            enc.base_encoding()
        );
        assert!(
            !enc.has_differences(),
            "New FontEncoding should have no differences overlay. Expected: has_differences() == false. Found: has_differences() == true. Why: Constructor with only base encoding creates empty differences.",
        );
    }

    #[test]
    fn test_font_encoding_glyph_name_base_only() {
        let enc = FontEncoding::new(Some(NamedEncoding::WinAnsi));
        assert_eq!(
            enc.glyph_name_for(0x92),
            Some(Arc::from("quoteright")),
            "Code 0x92 should map to 'quoteright' in WinAnsi. Expected: Some(\"quoteright\"). Found: {:?}. Why: WinAnsi encoding defines 0x92 = quoteright.",
            enc.glyph_name_for(0x92)
        );
        assert_eq!(
            enc.glyph_name_for(0x80),
            Some(Arc::from("Euro")),
            "Code 0x80 should map to 'Euro' in WinAnsi. Expected: Some(\"Euro\"). Found: {:?}. Why: WinAnsi encoding defines 0x80 = Euro.",
            enc.glyph_name_for(0x80)
        );
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

        assert_eq!(
            enc.glyph_name_for(0x92),
            Some(Arc::from("customquote")),
            "Code 0x92 should use differences overlay, not base. Expected: Some(\"customquote\"). Found: {:?}. Why: Differences overlay takes precedence over base encoding.",
            enc.glyph_name_for(0x92)
        );
        // Non-overlaid codes still use base
        assert_eq!(
            enc.glyph_name_for(0x80),
            Some(Arc::from("Euro")),
            "Code 0x80 should use base encoding (no overlay). Expected: Some(\"Euro\"). Found: {:?}. Why: Only 0x92 has a difference entry; 0x80 falls through to WinAnsi base.",
            enc.glyph_name_for(0x80)
        );
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

        assert_eq!(
            enc.glyph_name_for(0x20),
            Some(Arc::from("space")),
            "Code 0x20 should map to 'space' from differences. Expected: Some(\"space\"). Found: {:?}. Why: Differences overlay defines this mapping explicitly.",
            enc.glyph_name_for(0x20)
        );
        assert_eq!(
            enc.glyph_name_for(0x21),
            None,
            "Code 0x21 should not have a mapping. Expected: None (not in differences, no base). Found: {:?}. Why: No base encoding and no difference entry for 0x21.",
            enc.glyph_name_for(0x21)
        );
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
            Some(Arc::from("ArbitraryCustomGlyph")),
            "Custom glyph names should be returned as-is. Expected: Some(\"ArbitraryCustomGlyph\"). Found: {:?}. Why: Differences overlay can contain arbitrary glyph names not in Adobe Glyph List.",
            enc.glyph_name_for(0x20)
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

        assert_eq!(
            enc.glyph_name_for(0x92),
            Some(Arc::from("override")),
            "Code 0x92 should use differences override. Expected: Some(\"override\"). Found: {:?}. Why: Differences take precedence over base encoding (WinAnsi has 0x92 = quoteright, but difference overrides it).",
            enc.glyph_name_for(0x92)
        );
        // Base encoding still works for non-overlaid codes
        assert_eq!(
            enc.glyph_name_for(0x80),
            Some(Arc::from("Euro")),
            "Code 0x80 should use base encoding (no difference). Expected: Some(\"Euro\"). Found: {:?}. Why: No difference entry for 0x80, so lookup falls through to WinAnsi base.",
            enc.glyph_name_for(0x80)
        );
    }

    #[test]
    fn test_differences_overlay_skips_notdef() {
        // .notdef should be skipped during parsing
        let mut diagnostics = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(39),
            PdfObject::Name(Arc::from(".notdef")),
            PdfObject::Integer(96),
            PdfObject::Name(Arc::from("grave")),
        ]));

        let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

        // .notdef should be skipped, only grave should be present
        assert_eq!(
            overlay.get(39),
            None,
            "Code 39 should not have a mapping (.notdef skipped). \
             Expected: None. \
             Found: {:?}. \
             Why this matters: .notdef is in the default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be silently filtered during /Differences array parsing.",
            overlay.get(39)
        );
        assert_eq!(
            overlay.get(96),
            Some(Arc::from("grave")),
            "Code 96 should map to 'grave'. \
             Expected: Some(\"grave\"). \
             Found: {:?}. \
             Why this matters: 'grave' is not in the unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included in the /Differences overlay.",
            overlay.get(96)
        );
        assert_eq!(
            overlay.len(),
            1,
            "Overlay should contain exactly 1 entry. \
             Expected: 1 entry (grave at 96). \
             Found: {} entries. \
             Why this matters: .notdef at 39 was skipped per build/unmapped-glyph-names.json filtering rules, leaving only grave in the overlay.",
            overlay.len()
        );
        assert!(
            diagnostics.is_empty(),
            "Skipping .notdef should not generate diagnostics. \
             Expected: empty diagnostics. \
             Found: {} diagnostics. \
             Why this matters: Skipping unmapped glyphs is silent behavior by design - glyphs in build/unmapped-glyph-names.json are filtered during parsing without producing warnings.",
            diagnostics.len()
        );
    }

    #[test]
    fn test_differences_overlay_skips_notdef_with_slash() {
        // .notdef with leading slash should also be skipped
        let mut diagnostics = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(10),
            PdfObject::Name(Arc::from("/.notdef")),
            PdfObject::Integer(11),
            PdfObject::Name(Arc::from("A")),
        ]));

        let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

        // /.notdef should be skipped, only A should be present
        assert_eq!(
            overlay.get(10),
            None,
            "Code 10 should not have a mapping (/.notdef skipped). \
             Expected: None. \
             Found: {:?}. \
             Why this matters: /.notdef (with leading slash) is matched as an unmapped glyph in build/unmapped-glyph-names.json and should be filtered during /Differences array parsing.",
            overlay.get(10)
        );
        assert_eq!(
            overlay.get(11),
            Some(Arc::from("A")),
            "Code 11 should map to 'A'. \
             Expected: Some(\"A\"). \
             Found: {:?}. \
             Why this matters: 'A' is not in the unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included in the /Differences overlay.",
            overlay.get(11)
        );
        assert_eq!(
            overlay.len(),
            1,
            "Overlay should contain exactly 1 entry. \
             Expected: 1 entry (A at 11). \
             Found: {} entries. \
             Why this matters: /.notdef at 10 was skipped per build/unmapped-glyph-names.json filtering rules (slash variant matched), leaving only A.",
            overlay.len()
        );
        assert!(
            diagnostics.is_empty(),
            "Skipping /.notdef should not generate diagnostics. \
             Expected: empty diagnostics. \
             Found: {} diagnostics. \
             Why this matters: Skipping unmapped glyphs (including slash variants from build/unmapped-glyph-names.json) is silent behavior - no warnings should be emitted.",
            diagnostics.len()
        );
    }

    #[test]
    fn test_differences_overlay_custom_unmapped_glyph_names() {
        // Test that custom unmapped_glyph_names configuration works correctly
        // Create a custom set that skips "custom1" and "custom2" but allows other glyphs
        let mut custom_unmapped = std::collections::HashSet::new();
        custom_unmapped.insert("custom1".to_string());
        custom_unmapped.insert("custom2".to_string());

        // Parse a /Differences array with mixed glyphs:
        // - custom1 (should be skipped)
        // - A (normal glyph, should appear)
        // - custom2 (should be skipped)
        // - B (normal glyph, should appear)
        // - .notdef (default unmapped, but NOT in our custom set, so should appear)
        let mut diagnostics = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(10),
            PdfObject::Name(Arc::from("custom1")),
            PdfObject::Integer(11),
            PdfObject::Name(Arc::from("A")),
            PdfObject::Integer(12),
            PdfObject::Name(Arc::from("custom2")),
            PdfObject::Integer(13),
            PdfObject::Name(Arc::from("B")),
            PdfObject::Integer(14),
            PdfObject::Name(Arc::from(".notdef")),
        ]));

        // Use parse() which creates overlay with default unmapped set
        let overlay_default = DifferencesOverlay::parse(&arr.clone(), &mut diagnostics);

        // With default config, .notdef should be skipped but custom1/custom2 should appear
        assert_eq!(
            overlay_default.get(10),
            Some(Arc::from("custom1")),
            "Default config: custom1 should appear. \
             Expected: Some(\"custom1\"). \
             Found: {:?}. \
             Why this matters: custom1 is NOT in the default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included in /Differences overlay.",
            overlay_default.get(10)
        );
        assert_eq!(
            overlay_default.get(11),
            Some(Arc::from("A")),
            "Default config: 'A' should appear. \
             Expected: Some(\"A\"). \
             Found: {:?}. \
             Why this matters: 'A' is a normal glyph not in the unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included.",
            overlay_default.get(11)
        );
        assert_eq!(
            overlay_default.get(12),
            Some(Arc::from("custom2")),
            "Default config: custom2 should appear. \
             Expected: Some(\"custom2\"). \
             Found: {:?}. \
             Why this matters: custom2 is NOT in the default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included.",
            overlay_default.get(12)
        );
        assert_eq!(
            overlay_default.get(13),
            Some(Arc::from("B")),
            "Default config: 'B' should appear. \
             Expected: Some(\"B\"). \
             Found: {:?}. \
             Why this matters: 'B' is a normal glyph not in the unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it should be included.",
            overlay_default.get(13)
        );
        assert_eq!(
            overlay_default.get(14),
            None,
            "Default config: .notdef should be skipped. \
             Expected: None. \
             Found: {:?}. \
             Why this matters: .notdef IS in the default unmapped_glyph_names set (from build/unmapped-glyph-names.json), so it must be filtered from the /Differences overlay.",
            overlay_default.get(14)
        );
        assert_eq!(
            overlay_default.len(),
            4,
            "Default config: overlay should have 4 entries. \
             Expected: 4 entries (custom1, A, custom2, B). \
             Found: {} entries. \
             Why this matters: Only .notdef is filtered by default config per build/unmapped-glyph-names.json; all other glyphs should appear.",
            overlay_default.len()
        );
        assert!(
            diagnostics.is_empty(),
            "Default config: should not generate diagnostics. \
             Expected: empty diagnostics. \
             Found: {} diagnostics. \
             Why this matters: All glyphs were processed normally per build/unmapped-glyph-names.json filtering rules; no warnings expected.",
            diagnostics.len()
        );

        // Now test with custom unmapped set
        let mut overlay_custom = DifferencesOverlay::with_unmapped_glyph_names(custom_unmapped);

        // Manually add entries (simulating what parse() does with custom config)
        for (i, obj) in arr.as_array().unwrap().iter().enumerate() {
            if let PdfObject::Integer(code) = obj {
                let next_obj = arr.as_array().unwrap().get(i + 1);
                if let Some(PdfObject::Name(name)) = next_obj {
                    if *code <= 255 && !overlay_custom.is_unmapped_glyph_name(name) {
                        overlay_custom.entries.push((*code as u8, Arc::clone(name)));
                    }
                }
            }
        }

        // With custom config, custom1 and custom2 should be skipped, but .notdef should appear
        assert_eq!(
            overlay_custom.get(10),
            None,
            "Custom config: custom1 should be skipped. \
             Expected: None. \
             Found: {:?}. \
             Why this matters: custom1 IS in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}} (overriding default).",
            overlay_custom.get(10)
        );
        assert_eq!(
            overlay_custom.get(11),
            Some(Arc::from("A")),
            "Custom config: 'A' should appear. \
             Expected: Some(\"A\"). \
             Found: {:?}. \
             Why this matters: 'A' is NOT in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}}, so it should be included.",
            overlay_custom.get(11)
        );
        assert_eq!(
            overlay_custom.get(12),
            None,
            "Custom config: custom2 should be skipped. \
             Expected: None. \
             Found: {:?}. \
             Why this matters: custom2 IS in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}} (overriding default).",
            overlay_custom.get(12)
        );
        assert_eq!(
            overlay_custom.get(13),
            Some(Arc::from("B")),
            "Custom config: 'B' should appear. \
             Expected: Some(\"B\"). \
             Found: {:?}. \
             Why this matters: 'B' is NOT in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}}, so it should be included.",
            overlay_custom.get(13)
        );
        assert_eq!(
            overlay_custom.get(14),
            Some(Arc::from(".notdef")),
            "Custom config: .notdef should appear. \
             Expected: Some(\".notdef\"). \
             Found: {:?}. \
             Why this matters: .notdef is NOT in the custom unmapped_glyph_names set {{\"custom1\", \"custom2\"}} (unlike default config from build/unmapped-glyph-names.json).",
            overlay_custom.get(14)
        );
        assert_eq!(
            overlay_custom.len(),
            3,
            "Custom config: overlay should have 3 entries. \
             Expected: 3 entries (A, B, .notdef). \
             Found: {} entries. \
             Why this matters: custom1 and custom2 are filtered by custom set; .notdef is kept because custom set {{\"custom1\", \"custom2\"}} differs from default (build/unmapped-glyph-names.json).",
            overlay_custom.len()
        );
    }

    #[test]
    fn test_differences_overlay_empty_unmapped_glyph_names() {
        // Test that providing an empty unmapped_glyph_names set allows all glyphs
        let empty_unmapped = std::collections::HashSet::new();

        let mut diagnostics: Vec<Diagnostic> = Vec::new();
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(10),
            PdfObject::Name(Arc::from(".notdef")),
            PdfObject::Integer(11),
            PdfObject::Name(Arc::from("A")),
        ]));

        let mut overlay = DifferencesOverlay::with_unmapped_glyph_names(empty_unmapped);

        // Manually add entries (simulating what parse() does with empty config)
        for (i, obj) in arr.as_array().unwrap().iter().enumerate() {
            if let PdfObject::Integer(code) = obj {
                let next_obj = arr.as_array().unwrap().get(i + 1);
                if let Some(PdfObject::Name(name)) = next_obj {
                    if *code <= 255 && !overlay.is_unmapped_glyph_name(name) {
                        overlay.entries.push((*code as u8, Arc::clone(name)));
                    }
                }
            }
        }

        // With empty config, ALL glyphs should appear including .notdef
        assert_eq!(
            overlay.get(10),
            Some(Arc::from(".notdef")),
            "Empty config: .notdef should appear. \
             Expected: Some(\".notdef\"). \
             Found: {:?}. \
             Why this matters: Empty unmapped_glyph_names set means no glyphs are filtered, even .notdef from build/unmapped-glyph-names.json is not applied.",
            overlay.get(10)
        );
        assert_eq!(
            overlay.get(11),
            Some(Arc::from("A")),
            "Empty config: 'A' should appear. \
             Expected: Some(\"A\"). \
             Found: {:?}. \
             Why this matters: Empty unmapped_glyph_names set allows all glyphs without filtering from build/unmapped-glyph-names.json.",
            overlay.get(11)
        );
        assert_eq!(
            overlay.len(),
            2,
            "Empty config: overlay should have 2 entries. \
             Expected: 2 entries (.notdef, A). \
             Found: {} entries. \
             Why this matters: No glyphs are filtered when unmapped_glyph_names is empty (override of build/unmapped-glyph-names.json).",
            overlay.len()
        );
        assert!(
            diagnostics.is_empty(),
            "Empty config: should not generate diagnostics. \
             Expected: empty diagnostics. \
             Found: {} diagnostics. \
             Why this matters: Empty config is valid and processes all glyphs normally without applying filtering from build/unmapped-glyph-names.json.",
            diagnostics.len()
        );
    }

    #[test]
    fn test_unmapped_glyph_skip_behavior() {
        // Demonstrates that unmapped glyphs are skipped while mapped glyphs appear.
        // This test verifies the core CMAP generation behavior for glyph filtering.
        let mut diagnostics = Vec::new();

        // Create a /Differences array with mixed glyphs:
        // - .notdef (unmapped, should be skipped)
        // - A (normal glyph, should appear)
        // - space (normal glyph, should appear)
        // - .notdef again (unmapped, should be skipped)
        // - B (normal glyph, should appear)
        let arr = PdfObject::Array(Box::new(vec![
            PdfObject::Integer(32),
            PdfObject::Name(Arc::from(".notdef")),   // code 32: unmapped, should skip
            PdfObject::Integer(65),
            PdfObject::Name(Arc::from("A")),          // code 65: normal, should appear
            PdfObject::Integer(66),
            PdfObject::Name(Arc::from("space")),      // code 66: normal, should appear
            PdfObject::Integer(67),
            PdfObject::Name(Arc::from(".notdef")),    // code 67: unmapped, should skip
            PdfObject::Integer(68),
            PdfObject::Name(Arc::from("B")),          // code 68: normal, should appear
        ]));

        let overlay = DifferencesOverlay::parse(&arr, &mut diagnostics);

        // Verify unmapped glyphs do NOT appear in CMAP
        assert_eq!(
            overlay.get(32),
            None,
            ".notdef should be skipped. \
             Expected: code 32 not present in final mapping. \
             Found: present. \
             Why this matters: .notdef is in build/unmapped-glyph-names.json and should be filtered during CMAP generation."
        );
        assert_eq!(
            overlay.get(67),
            None,
            ".notdef should be skipped. \
             Expected: code 67 not present in final mapping. \
             Found: present. \
             Why this matters: .notdef is in build/unmapped-glyph-names.json and should be filtered regardless of position in /Differences array."
        );

        // Verify mapped glyphs DO appear in CMAP
        assert_eq!(
            overlay.get(65),
            Some(Arc::from("A")),
            "A should appear. \
             Expected: code 65 present in final mapping. \
             Found: absent. \
             Why this matters: 'A' is not in build/unmapped-glyph-names.json and should be included in the CMAP."
        );
        assert_eq!(
            overlay.get(66),
            Some(Arc::from("space")),
            "space should appear. \
             Expected: code 66 present in final mapping. \
             Found: absent. \
             Why this matters: 'space' is not in build/unmapped-glyph-names.json and should be included for proper text spacing."
        );
        assert_eq!(
            overlay.get(68),
            Some(Arc::from("B")),
            "B should appear. \
             Expected: code 68 present in final mapping. \
             Found: absent. \
             Why this matters: 'B' is not in build/unmapped-glyph-names.json and should be included in the CMAP."
        );

        // Verify final state
        assert_eq!(
            overlay.len(),
            3,
            "Should have exactly 3 mapped glyphs. \
             Expected: 3 entries (A, space, B). \
             Found: {} entries. \
             Why this matters: .notdef instances (codes 32, 67) were filtered per build/unmapped-glyph-names.json, leaving 3 normal glyphs.",
            overlay.len()
        );
        assert!(
            diagnostics.is_empty(),
            "Should not emit diagnostics for skipping. \
             Expected: empty diagnostics. \
             Found: {} diagnostics. \
             Why this matters: Skipping glyphs from build/unmapped-glyph-names.json is silent - no warnings should be emitted.",
            diagnostics.len()
        );
    }
}
