//! Standard 14 font metrics registry.
//!
//! This module provides compile-time metrics for the 14 Adobe Standard fonts
//! as defined in PDF 1.7. When a font is classified as `Type1Std14`, all
//! metric lookups come from this registry without embedding a font program.

include!(concat!(env!("OUT_DIR"), "/std14_registry.rs"));

/// Named encoding for Standard 14 fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedEncoding {
    /// StandardEncoding (most Standard 14 fonts)
    Standard,
    /// SymbolEncoding (Symbol font)
    Symbol,
    /// ZapfDingbatsEncoding (ZapfDingbats font)
    ZapfDingbats,
}

/// AFM-derived metrics for a Standard 14 font.
///
/// These metrics are compiled into the binary from Adobe's public AFM files
/// for the Core 14 fonts. Widths are indexed by character code (not glyph ID).
pub struct Std14Metrics {
    /// Character widths indexed by character code (0-255)
    pub widths: &'static [u16; 256],
    /// Font ascent (typographic ascent from AFM)
    pub ascent: i16,
    /// Font descent (typographic descent from AFM, typically negative)
    pub descent: i16,
    /// Italic angle in degrees (negative = oblique to the right)
    pub italic_angle: f32,
    /// Font bounding box [llx, lly, urx, ury] in font units
    pub font_bbox: [i16; 4],
    /// Cap height (height of uppercase H from baseline)
    pub cap_height: i16,
    /// StemV (vertical stem width for PDF font dictionaries)
    pub stem_v: i16,
    /// Named encoding type
    pub encoding: NamedEncoding,
}

impl Std14Metrics {
    /// Get the width for a character code.
    ///
    /// Returns 0 for codes outside 0-255 (should not happen with
    /// properly encoded PDF text).
    pub fn char_width(&self, code: u8) -> u16 {
        self.widths[code as usize]
    }

    /// Get the width for a 16-bit character code.
    ///
    /// Standard 14 fonts use single-byte encodings, so codes >= 256
    /// return the width of code 0 (typically undefined).
    pub fn char_width_16(&self, code: u16) -> u16 {
        if code < 256 {
            self.widths[code as usize]
        } else {
            self.widths[0]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_all_14_fonts() {
        let fonts = [
            "Courier",
            "Courier-Bold",
            "Courier-Oblique",
            "Courier-BoldOblique",
            "Times-Roman",
            "Times-Bold",
            "Times-Italic",
            "Times-BoldItalic",
            "Helvetica",
            "Helvetica-Bold",
            "Helvetica-Oblique",
            "Helvetica-BoldOblique",
            "Symbol",
            "ZapfDingbats",
        ];

        for font in fonts {
            let metrics = get_std14_metrics(font);
            assert!(metrics.is_some(), "Font {} not found in registry", font);
            let m = metrics.unwrap();
            assert_eq!(m.widths.len(), 256, "{}: widths array length", font);
        }
    }

    #[test]
    fn test_subset_prefix_resolution() {
        // Test that subset-prefixed names resolve after stripping
        use super::super::strip_subset_prefix;

        let prefixed = "ABCDEF+Times-Roman";
        let stripped = strip_subset_prefix(prefixed);
        let metrics = get_std14_metrics(stripped);
        assert!(metrics.is_some(), "Subset-prefixed font not found");
    }

    #[test]
    fn test_char_width() {
        let metrics = get_std14_metrics("Times-Roman").unwrap();

        // Space (code 32) should have a non-zero width
        assert!(metrics.char_width(32) > 0, "Space width should be > 0");

        // Courier is monospace - all printable chars should have same width
        let courier = get_std14_metrics("Courier").unwrap();
        let width_65 = courier.char_width(65); // 'A'
        let width_66 = courier.char_width(66); // 'B'
        assert_eq!(width_65, width_66, "Courier should be monospace");
        assert_eq!(width_65, 600, "Courier glyph width should be 600");
    }

    #[test]
    fn test_symbol_font_encoding() {
        let metrics = get_std14_metrics("Symbol").unwrap();
        assert_eq!(metrics.encoding, NamedEncoding::Symbol);
    }

    #[test]
    fn test_zapfdingbats_font_encoding() {
        let metrics = get_std14_metrics("ZapfDingbats").unwrap();
        assert_eq!(metrics.encoding, NamedEncoding::ZapfDingbats);
    }

    #[test]
    fn test_helvetica_metrics() {
        let metrics = get_std14_metrics("Helvetica").unwrap();

        // Helvetica from Adobe AFM
        assert_eq!(metrics.ascent, 718);
        assert_eq!(metrics.descent, -207);
        assert_eq!(metrics.italic_angle, 0.0);
        assert_eq!(metrics.cap_height, 718);
        assert_eq!(metrics.stem_v, 51);
    }

    #[test]
    fn test_courier_monospace() {
        let fonts = [
            "Courier",
            "Courier-Bold",
            "Courier-Oblique",
            "Courier-BoldOblique",
        ];

        for font in fonts {
            let metrics = get_std14_metrics(font).unwrap();
            // All Courier variants are monospace at 600 units
            for code in 32..127 {
                let w = metrics.char_width(code);
                assert_eq!(w, 600, "{}: code {} should be 600 wide", font, code);
            }
        }
    }

    #[test]
    fn test_italic_angles() {
        let regular = get_std14_metrics("Helvetica").unwrap();
        let oblique = get_std14_metrics("Helvetica-Oblique").unwrap();
        let bold_oblique = get_std14_metrics("Helvetica-BoldOblique").unwrap();

        assert_eq!(regular.italic_angle, 0.0);
        assert_eq!(oblique.italic_angle, -12.0);
        assert_eq!(bold_oblique.italic_angle, -12.0);
    }

    #[test]
    fn test_font_bbox() {
        let times = get_std14_metrics("Times-Roman").unwrap();
        // From Adobe Times-Roman AFM: FontBBox -168 -218 1000 898
        assert_eq!(times.font_bbox, [-168, -218, 1000, 898]);
    }

    #[test]
    fn test_invalid_font_returns_none() {
        let metrics = get_std14_metrics("NonExistentFont");
        assert!(metrics.is_none());
    }

    #[test]
    fn test_char_width_16() {
        let metrics = get_std14_metrics("Times-Roman").unwrap();

        // Valid single-byte code
        assert!(metrics.char_width_16(65) > 0);

        // Code >= 256 returns width of code 0 for Standard 14
        let w = metrics.char_width_16(256);
        assert_eq!(w, metrics.widths[0]);
    }
}
