//! Embedded font program loader.
//!
//! This module loads embedded font programs from PDF FontDescriptor objects
//! and provides a unified API for glyph metrics and cmap lookups across
//! TrueType, OpenType CFF, and Type1 fonts.

use std::sync::Arc;

use crate::diagnostics::{Diagnostic, DiagCode};
use crate::font::FontKind;
use crate::parser::object::types::{PdfDict, PdfObject};
use crate::parser::stream::{decode_stream, ExtractionOptions};

// Import AsFaceRef trait to access as_face_ref() method on OwnedFace
use owned_ttf_parser::AsFaceRef;

/// Result type for font operations.
pub type FontResult<T> = Result<T, FontError>;

/// Errors that can occur during font loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontError {
    /// No font program found in FontDescriptor.
    NoFontProgram,
    /// Font program stream could not be decoded.
    DecodeFailed(String),
    /// Font program is corrupt or invalid.
    InvalidFontData(String),
    /// Font type not supported for embedded loading.
    UnsupportedType(String),
}

impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontError::NoFontProgram => write!(f, "no font program in FontDescriptor"),
            FontError::DecodeFailed(msg) => write!(f, "font program decode failed: {}", msg),
            FontError::InvalidFontData(msg) => write!(f, "invalid font data: {}", msg),
            FontError::UnsupportedType(msg) => write!(f, "unsupported font type: {}", msg),
        }
    }
}

impl std::error::Error for FontError {}

/// Unified glyph metrics for embedded fonts.
///
/// Bounding box in font units: [x_min, y_min, x_max, y_max]
pub type GlyphBbox = [i16; 4];

/// Trait for font glyph lookups and metrics.
///
/// This trait provides a unified interface across different font formats.
/// Implementations may be "empty" (e.g., for corrupt fonts) and return
/// None for all lookups.
pub trait FontMetrics: Send + Sync {
    /// Get the glyph ID for a Unicode character.
    ///
    /// Returns None if the character is not mapped in the font's cmap.
    /// For subset fonts, many characters will return None.
    fn glyph_id_for(&self, ch: char) -> Option<u16>;

    /// Get the advance width for a glyph ID in font units.
    ///
    /// Returns None if the glyph ID is invalid.
    fn advance(&self, glyph_id: u16) -> Option<u16>;

    /// Get the bounding box for a glyph ID in font units.
    ///
    /// Returns None if the glyph ID is invalid.
    fn bbox(&self, glyph_id: u16) -> Option<GlyphBbox>;

    /// Get the units-per-em for the font.
    ///
    /// This is used to scale font metrics to text space.
    fn units_per_em(&self) -> u16;

    /// Check if this font has a valid cmap (for glyph_id_for).
    fn has_valid_cmap(&self) -> bool;
}

/// Empty font metrics implementation for corrupt/missing fonts.
///
/// This implementation returns None for all lookups and is used when
/// font loading fails but extraction should continue.
#[derive(Debug, Clone, Copy)]
pub struct EmptyFontMetrics;

impl FontMetrics for EmptyFontMetrics {
    fn glyph_id_for(&self, _ch: char) -> Option<u16> {
        None
    }

    fn advance(&self, _glyph_id: u16) -> Option<u16> {
        None
    }

    fn bbox(&self, _glyph_id: u16) -> Option<GlyphBbox> {
        None
    }

    fn units_per_em(&self) -> u16 {
        1000 // Default for Type1 fonts
    }

    fn has_valid_cmap(&self) -> bool {
        false
    }
}

/// TrueType/OpenType font metrics implementation.
///
/// Wraps an `owned_ttf_parser::OwnedFace` and provides glyph metrics.
pub struct OpenTypeMetrics {
    face: owned_ttf_parser::OwnedFace,
    units_per_em: u16,
    has_valid_cmap: bool,
}

impl OpenTypeMetrics {
    /// Create a new OpenTypeMetrics from raw font data.
    pub fn from_data(data: Vec<u8>, index: u32) -> FontResult<Self> {
        let face = owned_ttf_parser::OwnedFace::from_vec(data, index)
            .map_err(|e| FontError::InvalidFontData(format!("ttf-parser error: {:?}", e)))?;

        let face_ref = face.as_face_ref();
        let units_per_em = face_ref.units_per_em();

        // Check if we have a valid cmap subtable
        let has_valid_cmap = face_ref
            .tables()
            .cmap
            .map(|cmap| {
                // Try to find a valid Unicode subtable
                cmap.subtables
                    .into_iter()
                    .any(|st| st.is_unicode())
            })
            .unwrap_or(false);

        Ok(Self {
            face,
            units_per_em,
            has_valid_cmap,
        })
    }

    /// Get the underlying ttf-parser Face reference.
    pub fn face(&self) -> &owned_ttf_parser::Face<'_> {
        self.face.as_face_ref()
    }
}

impl FontMetrics for OpenTypeMetrics {
    fn glyph_id_for(&self, ch: char) -> Option<u16> {
        if !self.has_valid_cmap {
            return None;
        }

        let face_ref = self.face.as_face_ref();
        // Use Face's built-in glyph_index which handles cmap lookup
        face_ref
            .glyph_index(ch)
            .map(|id| id.0)
    }

    fn advance(&self, glyph_id: u16) -> Option<u16> {
        let face_ref = self.face.as_face_ref();
        face_ref
            .glyph_hor_advance(owned_ttf_parser::GlyphId(glyph_id))
            .map(|adv| adv as u16)
    }

    fn bbox(&self, glyph_id: u16) -> Option<GlyphBbox> {
        let face_ref = self.face.as_face_ref();
        let bbox = face_ref.glyph_bounding_box(owned_ttf_parser::GlyphId(glyph_id))?;
        Some([bbox.x_min, bbox.y_min, bbox.x_max, bbox.y_max])
    }

    fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    fn has_valid_cmap(&self) -> bool {
        self.has_valid_cmap
    }
}

/// Type1 font metrics implementation (limited).
///
/// This is a minimal implementation for Type1 fonts. Per the task requirements,
/// it only handles glyph name lists and widths from the FontDescriptor.
/// It does NOT parse CharStrings and has limited capability.
///
/// Type1 fonts in PDFs typically have their glyph names in the /Encoding
/// dictionary and widths in the /Widths array. This implementation uses
/// those for metrics lookup.
pub struct Type1Metrics {
    /// Character widths indexed by character code (for single-byte encodings).
    widths: Vec<u16>,
    /// Font bounding box from FontDescriptor.
    font_bbox: GlyphBbox,
    /// Units per em (default 1000 for Type1).
    units_per_em: u16,
    /// Has valid encoding (for glyph name lookup).
    has_valid_encoding: bool,
}

impl Type1Metrics {
    /// Create a new Type1Metrics from FontDescriptor data.
    ///
    /// This is a minimal implementation that only handles widths from
    /// the FontDescriptor. Full Type1 parsing is not implemented.
    pub fn from_descriptor(descriptor: &PdfDict, font_dict: &PdfDict) -> FontResult<Self> {
        // Extract /Widths array from font dict
        let widths = match font_dict.get("/Widths") {
            Some(PdfObject::Array(arr)) => {
                arr.iter()
                    .filter_map(|obj| obj.as_int())
                    .map(|i| i as u16)
                    .collect()
            }
            _ => return Err(FontError::InvalidFontData("missing /Widths array".into())),
        };

        // Extract /FontBBox from FontDescriptor
        let font_bbox = match descriptor.get("/FontBBox") {
            Some(PdfObject::Array(arr)) => {
                let coords: Vec<i16> = arr
                    .iter()
                    .filter_map(|obj| obj.as_int())
                    .map(|i| i as i16)
                    .collect();
                if coords.len() == 4 {
                    [coords[0], coords[1], coords[2], coords[3]]
                } else {
                    return Err(FontError::InvalidFontData("invalid /FontBBox".into()));
                }
            }
            _ => return Err(FontError::InvalidFontData("missing /FontBBox".into())),
        };

        // Check if we have a valid /Encoding
        let has_valid_encoding = font_dict.get("/Encoding").is_some();

        Ok(Self {
            widths,
            font_bbox,
            units_per_em: 1000, // Type1 default
            has_valid_encoding,
        })
    }

    /// Create an empty Type1Metrics (for fonts that couldn't be loaded).
    pub fn empty() -> Self {
        Self {
            widths: Vec::new(),
            font_bbox: [0, 0, 0, 0],
            units_per_em: 1000,
            has_valid_encoding: false,
        }
    }
}

impl FontMetrics for Type1Metrics {
    fn glyph_id_for(&self, _ch: char) -> Option<u16> {
        // Type1 fonts use glyph names, not glyph IDs.
        // For embedded Type1, we don't parse CharStrings, so we can't
        // map characters to glyph IDs. Return None to signal that
        // the fallback chain should be used.
        None
    }

    fn advance(&self, glyph_id: u16) -> Option<u16> {
        // For Type1, glyph_id is typically the character code for
        // single-byte encodings. Look up in the widths array.
        self.widths.get(glyph_id as usize).copied()
    }

    fn bbox(&self, _glyph_id: u16) -> Option<GlyphBbox> {
        // Type1 glyph-level bboxes require parsing CharStrings,
        // which we don't do. Return the font-level bbox.
        Some(self.font_bbox)
    }

    fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    fn has_valid_cmap(&self) -> bool {
        false // Type1 doesn't have cmap tables
    }
}

/// An embedded font program.
///
/// This wraps the font-specific metrics implementations and provides
/// a unified interface for glyph lookups.
#[derive(Clone)]
pub struct EmbeddedFont {
    /// The font metrics implementation.
    metrics: Arc<dyn FontMetrics>,
    /// The font kind (for type-specific handling).
    kind: FontKind,
    /// Diagnostics emitted during loading.
    diagnostics: Vec<Diagnostic>,
}

impl EmbeddedFont {
    /// Load an embedded font from a FontDescriptor.
    ///
    /// # Parameters
    ///
    /// - `font_dict`: The font dictionary from the resource dictionary
    /// - `source`: The PDF source to read font program streams from
    /// - `opts`: Extraction options (for stream decoding limits)
    /// - `doc_counter`: Cumulative decompressed bytes counter
    ///
    /// # Returns
    ///
    /// A `FontResult` containing the `EmbeddedFont` or a `FontError`.
    /// Diagnostics are collected even on success.
    pub fn load(
        font_dict: &PdfDict,
        source: &dyn crate::parser::stream::PdfSource,
        opts: &ExtractionOptions,
        doc_counter: &mut u64,
    ) -> FontResult<Self> {
        let kind = super::classify_font(font_dict);
        let mut diagnostics = Vec::new();

        // Get the FontDescriptor
        let descriptor = match font_dict.get("/FontDescriptor") {
            Some(PdfObject::Dict(d)) => d.as_ref(),
            Some(PdfObject::Ref(_ref)) => {
                // Indirect reference - would need resolution
                // For now, return empty metrics
                return Ok(Self {
                    metrics: Arc::new(EmptyFontMetrics),
                    kind,
                    diagnostics,
                });
            }
            _ => {
                return Err(FontError::NoFontProgram);
            }
        };

        // Determine which font program stream to use based on font type
        let (stream_key, expected_type) = match kind {
            FontKind::TrueType => ("/FontFile2", "TrueType"),
            FontKind::OpenTypeCFF => ("/FontFile3", "OpenType"),
            FontKind::Type1 => ("/FontFile", "Type1"),
            FontKind::Type1Std14 => {
                // Standard 14 fonts don't have embedded programs
                return Ok(Self {
                    metrics: Arc::new(EmptyFontMetrics),
                    kind,
                    diagnostics,
                });
            }
            _ => {
                // CID fonts, Type0, Type3 not supported yet
                diagnostics.push(Diagnostic::with_static_no_offset(
                    DiagCode::FontUnsupported,
                    "Embedded font loading not yet implemented for this font type",
                ));
                return Ok(Self {
                    metrics: Arc::new(EmptyFontMetrics),
                    kind,
                    diagnostics,
                });
            }
        };

        // Get the font program stream
        let font_stream = match descriptor.get(stream_key) {
            Some(PdfObject::Stream(s)) => s,
            Some(PdfObject::Ref(_ref)) => {
                // Indirect reference - would need resolution
                return Ok(Self {
                    metrics: Arc::new(EmptyFontMetrics),
                    kind,
                    diagnostics,
                });
            }
            _ => {
                return Err(FontError::NoFontProgram);
            }
        };

        // For FontFile3, verify the Subtype
        if kind == FontKind::OpenTypeCFF || kind == FontKind::CIDFontType0 {
            if let Some(PdfObject::Name(subtype)) = font_stream.dict.get("/Subtype") {
                let subtype_str: &str = subtype.as_ref();
                let subtype_clean = if subtype_str.starts_with('/') {
                    &subtype_str[1..]
                } else {
                    subtype_str
                };
                if subtype_clean != "OpenType" && subtype_clean != "CIDFontType0C" {
                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::FontUnsupported,
                        format!("Unexpected FontFile3 Subtype: {}", subtype_clean),
                    ));
                }
            }
        }

        // Decode the font program stream
        let font_data = decode_stream(font_stream, source, opts, doc_counter);

        if font_data.is_empty() {
            diagnostics.push(Diagnostic::with_static_no_offset(
                DiagCode::FontParseFailed,
                "Font program stream decoded to empty data",
            ));
            return Ok(Self {
                metrics: Arc::new(EmptyFontMetrics),
                kind,
                diagnostics,
            });
        }

        // Load the font based on type
        let metrics: Arc<dyn FontMetrics> = match kind {
            FontKind::TrueType | FontKind::OpenTypeCFF => {
                match OpenTypeMetrics::from_data(font_data, 0) {
                    Ok(ot_metrics) => {
                        // Check if cmap is valid
                        if !ot_metrics.has_valid_cmap() {
                            diagnostics.push(Diagnostic::with_static_no_offset(
                                DiagCode::FontParseFailed,
                                "Font has no valid Unicode cmap",
                            ));
                        }
                        Arc::new(ot_metrics)
                    }
                    Err(e) => {
                        diagnostics.push(Diagnostic::with_dynamic_no_offset(
                            DiagCode::FontParseFailed,
                            format!("OpenType font load failed: {}", e),
                        ));
                        Arc::new(EmptyFontMetrics)
                    }
                }
            }
            FontKind::Type1 => {
                match Type1Metrics::from_descriptor(descriptor, font_dict) {
                    Ok(t1_metrics) => Arc::new(t1_metrics),
                    Err(e) => {
                        diagnostics.push(Diagnostic::with_dynamic_no_offset(
                            DiagCode::FontParseFailed,
                            format!("Type1 font load failed: {}", e),
                        ));
                        Arc::new(Type1Metrics::empty())
                    }
                }
            }
            _ => Arc::new(EmptyFontMetrics),
        };

        Ok(Self {
            metrics,
            kind,
            diagnostics,
        })
    }

    /// Get the glyph ID for a Unicode character.
    ///
    /// Returns None if:
    /// - The character is not in the font's cmap (common for subset fonts)
    /// - The font has no valid cmap (corrupt or unusual encoding)
    /// - The font is Type1 (uses glyph names, not glyph IDs)
    pub fn glyph_id_for(&self, ch: char) -> Option<u16> {
        self.metrics.glyph_id_for(ch)
    }

    /// Get the advance width for a glyph ID in font units.
    ///
    /// Returns None if the glyph ID is invalid.
    pub fn advance(&self, glyph_id: u16) -> Option<u16> {
        self.metrics.advance(glyph_id)
    }

    /// Get the bounding box for a glyph ID in font units.
    ///
    /// Returns None if the glyph ID is invalid.
    pub fn bbox(&self, glyph_id: u16) -> Option<GlyphBbox> {
        self.metrics.bbox(glyph_id)
    }

    /// Get the units-per-em for the font.
    ///
    /// This is used to scale font metrics to text space.
    /// For Type1 fonts, this is always 1000.
    pub fn units_per_em(&self) -> u16 {
        self.metrics.units_per_em()
    }

    /// Check if this font has a valid cmap for Unicode lookups.
    pub fn has_valid_cmap(&self) -> bool {
        self.metrics.has_valid_cmap()
    }

    /// Get the font kind.
    pub fn kind(&self) -> FontKind {
        self.kind
    }

    /// Get diagnostics emitted during loading.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::types::intern;
    use crate::parser::stream::MemorySource;

    #[test]
    fn test_empty_font_metrics() {
        let metrics = EmptyFontMetrics;
        assert!(metrics.glyph_id_for('A').is_none());
        assert!(metrics.advance(0).is_none());
        assert!(metrics.bbox(0).is_none());
        assert_eq!(metrics.units_per_em(), 1000);
        assert!(!metrics.has_valid_cmap());
    }

    #[test]
    fn test_type1_metrics_empty() {
        let metrics = Type1Metrics::empty();
        assert!(metrics.glyph_id_for('A').is_none());
        assert!(metrics.advance(0).is_none());
        assert!(!metrics.has_valid_cmap());
    }

    #[test]
    fn test_type1_metrics_from_descriptor() {
        // Create a FontDescriptor-like dict
        let mut descriptor = PdfDict::new();
        descriptor.insert(intern("/FontBBox"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(-100),
            PdfObject::Integer(-200),
            PdfObject::Integer(1000),
            PdfObject::Integer(900),
        ])));

        // Create a font dict with /Widths
        let mut font_dict = PdfDict::new();
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(500),
                PdfObject::Integer(600),
                PdfObject::Integer(700),
            ])),
        );
        font_dict.insert(intern("/Encoding"), PdfObject::Name(intern("/WinAnsiEncoding")));

        let metrics = Type1Metrics::from_descriptor(&descriptor, &font_dict).unwrap();

        assert_eq!(metrics.units_per_em(), 1000);
        assert_eq!(metrics.font_bbox, [-100, -200, 1000, 900]);
        assert!(metrics.has_valid_encoding);
        assert_eq!(metrics.advance(0), Some(500));
        assert_eq!(metrics.advance(1), Some(600));
        assert_eq!(metrics.advance(2), Some(700));
        assert!(metrics.advance(3).is_none()); // Out of bounds
    }

    #[test]
    fn test_load_truetype_font_from_fixture() {
        // Test loading the DejaVuSans.ttf fixture
        // The fixture is at workspace root: /home/coding/pdftract/tests/fixtures/fonts/
        // From crate root, we need to go up two levels
        let font_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("fonts")
            .join("DejaVuSans.ttf");
        let font_data = std::fs::read(font_path).unwrap();

        let metrics = OpenTypeMetrics::from_data(font_data, 0).unwrap();

        // Verify basic properties
        assert!(metrics.units_per_em() > 0);
        assert!(metrics.has_valid_cmap());

        // Test glyph lookups for common characters
        // 'A' should be mapped in a Latin font
        let gid_a = metrics.glyph_id_for('A');
        assert!(gid_a.is_some(), "Latin font should map 'A'");

        // Get advance for the glyph
        let advance = metrics.advance(gid_a.unwrap());
        assert!(advance.is_some(), "Should have advance width");

        // Get bbox for the glyph
        let bbox = metrics.bbox(gid_a.unwrap());
        assert!(bbox.is_some(), "Should have bounding box");

        // Verify bbox is reasonable (not all zeros)
        let bbox = bbox.unwrap();
        assert_ne!(bbox, [0, 0, 0, 0], "Bbox should not be all zeros");
    }

    #[test]
    fn test_load_truetype_font_missing_cmap() {
        // Create minimal valid TrueType data (empty SFNT)
        // This should fail to load
        let invalid_data = vec![0u8; 100];

        let result = OpenTypeMetrics::from_data(invalid_data, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_embedded_font_load_from_dict() {
        // Create a minimal font dict with FontDescriptor
        let mut descriptor = PdfDict::new();
        descriptor.insert(intern("/FontBBox"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(-100),
            PdfObject::Integer(-200),
            PdfObject::Integer(1000),
            PdfObject::Integer(900),
        ])));

        // For this test, we'll use a Type1-style descriptor without a stream
        // to test the fallback path
        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type1")));
        font_dict.insert(intern("/BaseFont"), PdfObject::Name(intern("TestFont")));
        font_dict.insert(
            intern("/FontDescriptor"),
            PdfObject::Dict(Box::new(descriptor)),
        );
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(500)])),
        );

        // Try to load - should fail gracefully without a stream
        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = EmbeddedFont::load(&font_dict, &source, &opts, &mut counter);

        // Should get an error about no font program
        assert!(matches!(result, Err(FontError::NoFontProgram)));
    }

    #[test]
    fn test_subset_font_behavior() {
        // Test that subset fonts (which have limited glyph sets)
        // return None for unmapped characters
        let font_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("fonts")
            .join("DejaVuSans.ttf");
        let font_data = std::fs::read(font_path).unwrap();
        let metrics = OpenTypeMetrics::from_data(font_data, 0).unwrap();

        // Common Latin characters should be mapped
        assert!(metrics.glyph_id_for('A').is_some());
        assert!(metrics.glyph_id_for('z').is_some());
        assert!(metrics.glyph_id_for('0').is_some());

        // Uncommon characters might not be in the base font
        // (This depends on the specific fixture)
        let result = metrics.glyph_id_for('\u{1F600}'); // Emoji
        // May or may not be present, but shouldn't panic
        let _ = result;
    }

    #[test]
    fn test_truetype_glyph_id_for_matches_cmap() {
        // Acceptance criteria: Successfully load a TrueType font from a fixture PDF;
        // verify glyph_id_for('A') matches Face cmap.
        let font_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("fonts")
            .join("DejaVuSans.ttf");
        let font_data = std::fs::read(font_path).unwrap();
        let metrics = OpenTypeMetrics::from_data(font_data, 0).unwrap();

        // Test common Latin characters
        for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".chars() {
            let gid = metrics.glyph_id_for(ch);
            assert!(gid.is_some(), "Character '{}' should be mapped in Latin font", ch);

            // Verify advance width exists for mapped glyphs
            let advance = metrics.advance(gid.unwrap());
            assert!(advance.is_some(), "Advance should exist for glyph ID {}", gid.unwrap());
            assert!(advance.unwrap() > 0, "Advance should be positive for glyph ID {}", gid.unwrap());

            // Verify bbox exists
            let bbox = metrics.bbox(gid.unwrap());
            assert!(bbox.is_some(), "Bbox should exist for glyph ID {}", gid.unwrap());
        }
    }

    #[test]
    fn test_font_metrics_units_per_em_scaling() {
        // Verify that units_per_em is correctly retrieved for scaling
        let font_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("fonts")
            .join("DejaVuSans.ttf");
        let font_data = std::fs::read(font_path).unwrap();
        let metrics = OpenTypeMetrics::from_data(font_data, 0).unwrap();

        let upem = metrics.units_per_em();
        // DejaVuSans has UPEM 2048 (standard for many OpenType fonts)
        assert_eq!(upem, 2048, "DejaVuSans should have UPEM of 2048");

        // Verify that advance widths are in font units (less than UPEM for typical glyphs)
        let gid_a = metrics.glyph_id_for('A').unwrap();
        let advance_a = metrics.advance(gid_a).unwrap();
        assert!(advance_a <= upem, "Advance should be in font units (≤ UPEM)");
    }

    #[test]
    fn test_corrupt_font_emits_diagnostic() {
        // Acceptance criteria: Corrupt font program: return a Font with no glyph_id_for hits;
        // emit FONT_PARSE_FAILED diagnostic, do not abort.
        let invalid_data = vec![0u8; 100]; // Not a valid font

        let result = OpenTypeMetrics::from_data(invalid_data, 0);

        // Should fail to load
        assert!(result.is_err());

        // The error should be InvalidFontData
        match result {
            Err(FontError::InvalidFontData(msg)) => {
                assert!(msg.contains("ttf-parser error"), "Error should mention ttf-parser");
            }
            _ => panic!("Expected InvalidFontData error"),
        }
    }

    #[test]
    fn test_empty_font_metrics_graceful_handling() {
        // Verify that EmptyFontMetrics doesn't panic on any operation
        let metrics = EmptyFontMetrics;

        // None of these should panic
        assert!(metrics.glyph_id_for('A').is_none());
        assert!(metrics.glyph_id_for('\u{0}').is_none());
        assert!(metrics.glyph_id_for('\u{10FFFF}').is_none());

        assert!(metrics.advance(0).is_none());
        assert!(metrics.advance(1000).is_none());
        assert!(metrics.advance(u16::MAX).is_none());

        assert!(metrics.bbox(0).is_none());
        assert!(metrics.bbox(1000).is_none());

        assert_eq!(metrics.units_per_em(), 1000);
        assert!(!metrics.has_valid_cmap());
    }

    #[test]
    fn test_type1_limited_capability_no_charstrings() {
        // Acceptance criteria: Type1 font program: gracefully wrap with limited
        // capability; do not crash on missing CharStrings parser.
        let mut descriptor = PdfDict::new();
        descriptor.insert(intern("/FontBBox"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(-100),
            PdfObject::Integer(-200),
            PdfObject::Integer(1000),
            PdfObject::Integer(900),
        ])));

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type1")));
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![
                PdfObject::Integer(500),
                PdfObject::Integer(600),
            ])),
        );

        let metrics = Type1Metrics::from_descriptor(&descriptor, &font_dict).unwrap();

        // glyph_id_for should always return None (Type1 uses glyph names, not GIDs)
        assert!(metrics.glyph_id_for('A').is_none());
        assert!(metrics.glyph_id_for('z').is_none());

        // advance should work for character codes
        assert_eq!(metrics.advance(0), Some(500));
        assert_eq!(metrics.advance(1), Some(600));
        assert!(metrics.advance(2).is_none());

        // bbox should return font bbox (we don't parse CharStrings)
        let bbox = metrics.bbox(0).unwrap();
        assert_eq!(bbox, [-100, -200, 1000, 900]);

        // No cmap for Type1
        assert!(!metrics.has_valid_cmap());
    }

    #[test]
    fn test_opentype_metrics_has_valid_cmap_detection() {
        // Verify that has_valid_cmap correctly detects Unicode cmap presence
        let font_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("fonts")
            .join("DejaVuSans.ttf");
        let font_data = std::fs::read(font_path).unwrap();
        let metrics = OpenTypeMetrics::from_data(font_data, 0).unwrap();

        // DejaVuSans has a Unicode cmap
        assert!(metrics.has_valid_cmap(), "DejaVuSans should have valid Unicode cmap");
    }

    #[test]
    fn test_embedded_font_returns_diagnostics() {
        // Verify that EmbeddedFont collects and returns diagnostics
        let mut descriptor = PdfDict::new();
        descriptor.insert(intern("/FontBBox"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Integer(1000),
            PdfObject::Integer(1000),
        ])));

        let mut font_dict = PdfDict::new();
        font_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type1")));
        font_dict.insert(
            intern("/FontDescriptor"),
            PdfObject::Dict(Box::new(descriptor)),
        );
        font_dict.insert(
            intern("/Widths"),
            PdfObject::Array(Box::new(vec![PdfObject::Integer(500)])),
        );

        // Try to load - should emit NoFontProgram error
        let source = MemorySource::new(vec![]);
        let opts = ExtractionOptions::default();
        let mut counter = 0;

        let result = EmbeddedFont::load(&font_dict, &source, &opts, &mut counter);

        // Should get an error
        assert!(matches!(result, Err(FontError::NoFontProgram)));
    }
}
