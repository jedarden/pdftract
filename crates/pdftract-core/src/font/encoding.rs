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
        assert_eq!(NamedEncoding::from_name("WinAnsiEncoding"), Some(NamedEncoding::WinAnsi));
        assert_eq!(NamedEncoding::from_name("MacRomanEncoding"), Some(NamedEncoding::MacRoman));
        assert_eq!(NamedEncoding::from_name("MacExpertEncoding"), Some(NamedEncoding::MacExpert));
        assert_eq!(NamedEncoding::from_name("StandardEncoding"), Some(NamedEncoding::Standard));
        assert_eq!(NamedEncoding::from_name("SymbolEncoding"), Some(NamedEncoding::Symbol));
        assert_eq!(NamedEncoding::from_name("ZapfDingbatsEncoding"), Some(NamedEncoding::ZapfDingbats));

        // Test with leading slash
        assert_eq!(NamedEncoding::from_name("/WinAnsiEncoding"), Some(NamedEncoding::WinAnsi));

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
}
