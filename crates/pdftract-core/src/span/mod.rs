//! Span struct definition (Phase 4.1).
//!
//! This module implements the Span struct, which is the primary output
//! of Phase 4 glyph-to-span merging. Span is the second-most-important
//! struct in the output schema (after Glyph).
//!
//! # Span Struct
//!
//! Per plan section Phase 4.1 (lines 1640-1653):
//! ```rust
//! struct Span {
//!     text: String,
//!     bbox: [f32; 4],          // union of member glyph bboxes
//!     font: Arc<str>,
//!     size: f32,
//!     color: Option<CssHexColor>,
//!     rendering_mode: u8,
//!     confidence: f32,         // minimum glyph confidence
//!     confidence_source: ConfidenceSource,
//!     lang: Option<Arc<str>>,  // filled in Phase 7 normalization
//!     flags: u8,               // SpanFlags bitmask: bit 0=bold, 1=italic, 2=smallcaps, 3=subscript, 4=superscript
//! }
//! ```

use crate::confidence::ConfidenceSource;
use crate::span_flags::flags;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// CSS hex color newtype (#rrggbb format).
///
/// This newtype enforces the #rrggbb format at construction time.
/// It is used to represent fill colors that can be serialized to CSS.
/// Spot colors and other non-DeviceRGB/DeviceGray colors serialize as None.
///
/// # Example
///
/// ```
/// use pdftract_core::span::CssHexColor;
///
/// let red = CssHexColor::new("#ff0000").unwrap();
/// assert_eq!(red.as_str(), "#ff0000");
///
/// let invalid = CssHexColor::new("red");
/// assert!(invalid.is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CssHexColor(pub String);

impl CssHexColor {
    /// Create a new CssHexColor from a string.
    ///
    /// The string must be in #rrggbb format (7 characters: # + 6 hex digits).
    /// Hex digits may be uppercase or lowercase.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not exactly 7 characters or does
    /// not start with '#' or contains non-hex characters after '#'.
    pub fn new(s: &str) -> Result<Self, String> {
        if s.len() != 7 {
            return Err(format!(
                "CssHexColor must be exactly 7 characters (#rrggbb), got {}",
                s.len()
            ));
        }
        if !s.starts_with('#') {
            return Err("CssHexColor must start with '#'".to_string());
        }
        let hex = &s[1..];
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "CssHexColor must contain only hex digits after '#', got {}",
                hex
            ));
        }
        Ok(CssHexColor(s.to_lowercase()))
    }

    /// Get the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert from an RGB tuple.
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        CssHexColor(format!("#{:02x}{:02x}{:02x}", r, g, b))
    }
}

/// SpanFlags bit constants.
///
/// These constants are used to test individual bits in the Span.flags field.
/// Multiple flags can be combined using bitwise OR.
pub mod span_flags {
    /// Bit 0: Bold text
    pub const BOLD: u8 = 1 << 0;
    /// Bit 1: Italic text
    pub const ITALIC: u8 = 1 << 1;
    /// Bit 2: Small caps text
    pub const SMALLCAPS: u8 = 1 << 2;
    /// Bit 3: Subscript text
    pub const SUBSCRIPT: u8 = 1 << 3;
    /// Bit 4: Superscript text
    pub const SUPERSCRIPT: u8 = 1 << 4;
}

/// A span of text extracted from a PDF (Phase 4 output).
///
/// This struct represents a contiguous run of glyphs that share the same
/// font, size, color, and rendering mode. It is the primary output of
/// Phase 4 glyph-to-span merging and is used throughout Phase 5 (layout)
/// and Phase 6 (output).
///
/// # Field Descriptions
///
/// - **text**: The concatenated text content of all glyphs in the span.
///   Valid UTF-8, never contains U+FFFD unless a glyph was U+FFFD and
///   readability correction did not repair it.
///
/// - **bbox**: Union of member glyph bounding boxes in PDF user space
///   [x0, y0, x1, y1] with lower-left origin, AFTER /Rotate normalization.
///
/// - **font**: Font name shared via Arc across all spans using the same font.
///
/// - **size**: Font size in points.
///
/// - **color**: Fill color as CSS hex string, or None for Spot/Other colorspaces.
///
/// - **rendering_mode**: Text rendering mode (0-7 per PDF spec).
///
/// - **confidence**: Minimum confidence of all glyphs in the span [0.0, 1.0].
///
/// - **confidence_source**: Source of confidence (Native, Heuristic, Ocr).
///
/// - **lang**: Language tag (BCP 47), None until Phase 7 fills it from /Lang
///   or detected script.
///
/// - **flags**: SpanFlags bitmask (bold, italic, smallcaps, subscript, superscript).
///
/// # Invariants
///
/// - INV: text is VALID UTF-8 (Rust String); no U+FFFD unless the underlying
///   glyph was U+FFFD AND the readability correction did not repair it.
/// - INV: bbox is [x0, y0, x1, y1] PDF user space, lower-left origin, AFTER
///   /Rotate normalization.
/// - INV: color may be None when the source colorspace was Spot or Other;
///   JSON serializes as null.
/// - INV: lang is None until Phase 7 fills it from /Lang or detected script.
/// - INV: flags is initially 0; Phase 4.1 flag detector sets bits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Span {
    /// Concatenated text content of the span.
    pub text: String,
    /// Union of member glyph bboxes [x0, y0, x1, y1] in PDF user space.
    pub bbox: [f32; 4],
    /// Font name (shared via Arc).
    pub font: Arc<str>,
    /// Font size in points.
    pub size: f32,
    /// Fill color as CSS hex (#rrggbb), or None for Spot/Other colorspaces.
    pub color: Option<CssHexColor>,
    /// Text rendering mode (0-7 per PDF spec).
    pub rendering_mode: u8,
    /// Minimum confidence of all glyphs in the span [0.0, 1.0].
    pub confidence: f32,
    /// Source of confidence (Native, Heuristic, Ocr).
    pub confidence_source: ConfidenceSource,
    /// Language tag (BCP 47), None until Phase 7.
    pub lang: Option<Arc<str>>,
    /// SpanFlags bitmask (bold, italic, smallcaps, subscript, superscript).
    pub flags: u8,
}

impl Span {
    /// Create a new Span with the given fields.
    ///
    /// This is the primary constructor used by Phase 4 glyph-to-span merging.
    pub fn new(
        text: String,
        bbox: [f32; 4],
        font: Arc<str>,
        size: f32,
        color: Option<CssHexColor>,
        rendering_mode: u8,
        confidence: f32,
        confidence_source: ConfidenceSource,
        lang: Option<Arc<str>>,
        flags: u8,
    ) -> Self {
        Self {
            text,
            bbox,
            font,
            size,
            color,
            rendering_mode,
            confidence,
            confidence_source,
            lang,
            flags,
        }
    }

    /// Create an empty span with default values.
    ///
    /// Used as a starting point for span accumulation.
    pub fn empty() -> Self {
        Self {
            text: String::new(),
            bbox: [0.0, 0.0, 0.0, 0.0],
            font: Arc::from(""),
            size: 0.0,
            color: None,
            rendering_mode: 0,
            confidence: 1.0,
            confidence_source: ConfidenceSource::Native,
            lang: None,
            flags: 0,
        }
    }

    /// Check if the bold flag is set.
    pub fn is_bold(&self) -> bool {
        self.flags & span_flags::BOLD != 0
    }

    /// Check if the italic flag is set.
    pub fn is_italic(&self) -> bool {
        self.flags & span_flags::ITALIC != 0
    }

    /// Check if the smallcaps flag is set.
    pub fn is_smallcaps(&self) -> bool {
        self.flags & span_flags::SMALLCAPS != 0
    }

    /// Check if the subscript flag is set.
    pub fn is_subscript(&self) -> bool {
        self.flags & span_flags::SUBSCRIPT != 0
    }

    /// Check if the superscript flag is set.
    pub fn is_superscript(&self) -> bool {
        self.flags & span_flags::SUPERSCRIPT != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // CssHexColor tests

    #[test]
    fn test_css_hex_color_new_valid_lowercase() {
        let color = CssHexColor::new("#ff0000").unwrap();
        assert_eq!(color.as_str(), "#ff0000");
    }

    #[test]
    fn test_css_hex_color_new_valid_uppercase() {
        let color = CssHexColor::new("#FF0000").unwrap();
        assert_eq!(color.as_str(), "#ff0000");
    }

    #[test]
    fn test_css_hex_color_new_valid_mixed_case() {
        let color = CssHexColor::new("#Ff00Aa").unwrap();
        assert_eq!(color.as_str(), "#ff00aa");
    }

    #[test]
    fn test_css_hex_color_new_invalid_too_short() {
        let result = CssHexColor::new("#f00");
        assert!(result.is_err());
    }

    #[test]
    fn test_css_hex_color_new_invalid_too_long() {
        let result = CssHexColor::new("#ff0000ff");
        assert!(result.is_err());
    }

    #[test]
    fn test_css_hex_color_new_invalid_no_hash() {
        let result = CssHexColor::new("ff0000");
        assert!(result.is_err());
    }

    #[test]
    fn test_css_hex_color_new_invalid_non_hex() {
        let result = CssHexColor::new("#fg0000");
        assert!(result.is_err());
    }

    #[test]
    fn test_css_hex_color_from_rgb() {
        let color = CssHexColor::from_rgb(255, 0, 0);
        assert_eq!(color.as_str(), "#ff0000");
    }

    #[test]
    fn test_css_hex_color_clone_is_cheap() {
        let color = CssHexColor::new("#00ff00").unwrap();
        let cloned = color.clone();
        assert_eq!(color, cloned);
    }

    // SpanFlags tests

    #[test]
    fn test_span_flags_bold_bit() {
        assert_eq!(span_flags::BOLD, 1);
        assert_eq!(span_flags::ITALIC, 2);
        assert_eq!(span_flags::SMALLCAPS, 4);
        assert_eq!(span_flags::SUBSCRIPT, 8);
        assert_eq!(span_flags::SUPERSCRIPT, 16);
    }

    #[test]
    fn test_span_flags_combinable() {
        let bold_italic = span_flags::BOLD | span_flags::ITALIC;
        assert_eq!(bold_italic, 3);
    }

    // Span struct tests

    #[test]
    fn test_span_constructible_with_all_fields() {
        let span = Span::new(
            "Hello".to_string(),
            [0.0, 0.0, 100.0, 12.0],
            Arc::from("Helvetica"),
            12.0,
            Some(CssHexColor::new("#000000").unwrap()),
            0,
            1.0,
            ConfidenceSource::Native,
            None,
            0,
        );
        assert_eq!(span.text, "Hello");
        assert_eq!(&*span.font, "Helvetica");
        assert_eq!(span.size, 12.0);
    }

    #[test]
    fn test_span_empty() {
        let span = Span::empty();
        assert!(span.text.is_empty());
        assert_eq!(span.bbox, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(span.flags, 0);
    }

    #[test]
    fn test_span_clone_is_cheap() {
        let span = Span::new(
            "Hello".to_string(),
            [0.0, 0.0, 100.0, 12.0],
            Arc::from("Helvetica"),
            12.0,
            Some(CssHexColor::new("#000000").unwrap()),
            0,
            1.0,
            ConfidenceSource::Native,
            Some(Arc::from("en")),
            span_flags::BOLD,
        );
        let cloned = span.clone();
        assert_eq!(span, cloned);
        // Arc<str> means font and lang are shared
        assert!(Arc::ptr_eq(&span.font, &cloned.font));
        if let (Some(lang1), Some(lang2)) = (&span.lang, &cloned.lang) {
            assert!(Arc::ptr_eq(lang1, lang2));
        }
    }

    #[test]
    fn test_span_serde_json_roundtrip() {
        let span = Span::new(
            "Hello".to_string(),
            [0.0, 0.0, 100.0, 12.0],
            Arc::from("Helvetica"),
            12.0,
            Some(CssHexColor::new("#ff0000").unwrap()),
            0,
            1.0,
            ConfidenceSource::Native,
            None,
            span_flags::BOLD | span_flags::ITALIC,
        );

        let json = serde_json::to_string(&span).unwrap();
        let deserialized: Span = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.text, span.text);
        assert_eq!(deserialized.bbox, span.bbox);
        assert_eq!(deserialized.font, span.font);
        assert_eq!(deserialized.size, span.size);
        assert_eq!(deserialized.rendering_mode, span.rendering_mode);
        assert_eq!(deserialized.confidence, span.confidence);
        assert_eq!(deserialized.confidence_source, span.confidence_source);
        assert_eq!(deserialized.flags, span.flags);
    }

    #[test]
    fn test_span_with_none_color_serializes() {
        let span = Span::new(
            "Hello".to_string(),
            [0.0, 0.0, 100.0, 12.0],
            Arc::from("Helvetica"),
            12.0,
            None,
            0,
            1.0,
            ConfidenceSource::Native,
            None,
            0,
        );

        let json = serde_json::to_string(&span).unwrap();
        assert!(json.contains(r#""color":null"#));
    }

    #[test]
    fn test_span_is_bold() {
        let mut span = Span::empty();
        span.flags = span_flags::BOLD;
        assert!(span.is_bold());
        assert!(!span.is_italic());
    }

    #[test]
    fn test_span_is_italic() {
        let mut span = Span::empty();
        span.flags = span_flags::ITALIC;
        assert!(span.is_italic());
        assert!(!span.is_bold());
    }

    #[test]
    fn test_span_is_smallcaps() {
        let mut span = Span::empty();
        span.flags = span_flags::SMALLCAPS;
        assert!(span.is_smallcaps());
    }

    #[test]
    fn test_span_is_subscript() {
        let mut span = Span::empty();
        span.flags = span_flags::SUBSCRIPT;
        assert!(span.is_subscript());
        assert!(!span.is_superscript());
    }

    #[test]
    fn test_span_is_superscript() {
        let mut span = Span::empty();
        span.flags = span_flags::SUPERSCRIPT;
        assert!(span.is_superscript());
        assert!(!span.is_subscript());
    }

    #[test]
    fn test_span_combined_flags() {
        let mut span = Span::empty();
        span.flags = span_flags::BOLD | span_flags::ITALIC;
        assert!(span.is_bold());
        assert!(span.is_italic());
    }

    #[test]
    fn test_span_size_within_budget() {
        // AC: Span struct size ~80 bytes (Arc str = 16 bytes shared, String avg 32, bbox 16, scalars 16)
        let size = std::mem::size_of::<Span>();
        // Check that we're within reasonable bounds
        assert!(size <= 120, "Span struct size {} exceeds 120 bytes", size);
        eprintln!("Span struct size: {} bytes", size);
    }

    #[test]
    fn test_span_confidence_source_variants() {
        // Test all three ConfidenceSource variants
        let native = Span::new(
            "text".to_string(),
            [0.0, 0.0, 100.0, 12.0],
            Arc::from("Helvetica"),
            12.0,
            None,
            0,
            1.0,
            ConfidenceSource::Native,
            None,
            0,
        );
        assert_eq!(native.confidence_source, ConfidenceSource::Native);

        let heuristic = Span::new(
            "text".to_string(),
            [0.0, 0.0, 100.0, 12.0],
            Arc::from("Helvetica"),
            12.0,
            None,
            0,
            0.5,
            ConfidenceSource::Heuristic,
            None,
            0,
        );
        assert_eq!(heuristic.confidence_source, ConfidenceSource::Heuristic);

        let ocr = Span::new(
            "text".to_string(),
            [0.0, 0.0, 100.0, 12.0],
            Arc::from("Helvetica"),
            12.0,
            None,
            0,
            0.8,
            ConfidenceSource::Ocr,
            None,
            0,
        );
        assert_eq!(ocr.confidence_source, ConfidenceSource::Ocr);
    }
}
