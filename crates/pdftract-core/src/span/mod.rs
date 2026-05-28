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
use crate::font::UnicodeSource;
use crate::glyph::Glyph;
use crate::graphics_state::Color;
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

/// Map UnicodeSource to ConfidenceSource per plan Phase 4.1.
///
/// | UnicodeSource    | ConfidenceSource |
/// |------------------|-------------------|
/// | ToUnicode        | Native            |
/// | Agl              | Native            |
/// | Fingerprint      | Native            |
/// | ShapeMatch       | Heuristic         |
/// | Unknown (U+FFFD) | Heuristic         |
/// | Ocr              | Ocr               |
fn map_unicode_source_to_confidence(source: UnicodeSource) -> ConfidenceSource {
    match source {
        UnicodeSource::ToUnicode | UnicodeSource::Agl | UnicodeSource::Fingerprint => {
            ConfidenceSource::Native
        }
        UnicodeSource::ShapeMatch | UnicodeSource::Unknown => ConfidenceSource::Heuristic,
        UnicodeSource::Ocr => ConfidenceSource::Ocr,
    }
}

/// Normalize a Color to RGB tuple for comparison.
///
/// Returns `Some((r, g, b))` for DeviceGray, DeviceRGB, and DeviceCMYK.
/// Returns `None` for Spot and Other colors (compared by variant equality).
fn normalize_color_for_comparison(color: &Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::DeviceGray(v) => {
            let v = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            Some((v, v, v))
        }
        Color::DeviceRGB(rgb) => {
            let r = (rgb[0].clamp(0.0, 1.0) * 255.0).round() as u8;
            let g = (rgb[1].clamp(0.0, 1.0) * 255.0).round() as u8;
            let b = (rgb[2].clamp(0.0, 1.0) * 255.0).round() as u8;
            Some((r, g, b))
        }
        Color::DeviceCMYK(cmyk) => {
            // CMYK → RGB conversion: R = (1-C)*(1-K)
            let c = cmyk[0].clamp(0.0, 1.0);
            let m = cmyk[1].clamp(0.0, 1.0);
            let y = cmyk[2].clamp(0.0, 1.0);
            let k = cmyk[3].clamp(0.0, 1.0);
            let r = ((1.0 - c) * (1.0 - k) * 255.0).round() as u8;
            let g = ((1.0 - m) * (1.0 - k) * 255.0).round() as u8;
            let b = ((1.0 - y) * (1.0 - k) * 255.0).round() as u8;
            Some((r, g, b))
        }
        Color::Spot(_, _) | Color::Other => None,
    }
}

/// Check if two colors are equal using RGB-normalized comparison.
///
/// For DeviceGray, DeviceRGB, and DeviceCMYK, compares using normalized RGB values.
/// For Spot and Other, compares by variant equality (Spot colors compared by name AND tint exactly).
fn colors_equal(a: &Color, b: &Color) -> bool {
    match (normalize_color_for_comparison(a), normalize_color_for_comparison(b)) {
        (Some(rgb_a), Some(rgb_b)) => rgb_a == rgb_b,
        (None, None) => a == b, // Both Spot/Other: compare by variant (Spot by name+tint)
        _ => false,             // One normalizable, one not: different
    }
}

/// Append a glyph's codepoint to a span's text.
///
/// This function implements the per-glyph text assembly logic for Phase 4.1.
/// It appends the glyph's codepoint to the span's text field.
///
/// Per the bead pdftract-2c5sx acceptance criteria:
/// - Single codepoint glyphs: append the char directly
/// - Multi-codepoint glyphs (ligatures): Phase 2 already expands these into
///   separate Glyph structs, so per-glyph append works correctly
/// - RTL text: preserved in visual order; bidi reordering happens in Phase 4.2
///
/// # Arguments
///
/// * `span` - Mutable reference to the span to append to
/// * `glyph` - The glyph whose codepoint should be appended
///
/// # Examples
///
/// ```
/// use pdftract_core::span::assemble_text;
/// use pdftract_core::span::Span;
///
/// let mut span = Span::empty();
/// let glyph = Glyph::new('A', ...);
/// assemble_text(&mut span, &glyph);
/// assert_eq!(span.text, "A");
/// ```
fn assemble_text(span: &mut Span, glyph: &Glyph) {
    span.text.push(glyph.codepoint);
}

/// Merge consecutive glyphs into spans using the 5-trigger break detector.
///
/// This function implements Phase 4.1 glyph-to-span merging. It walks the
/// per-page glyph list and groups consecutive glyphs into spans. A new span
/// begins when any of the 5 triggers fires on the current glyph:
///
/// 1. `font_name != prev font_name`
/// 2. `(font_size - prev_font_size).abs() > 0.5`
/// 3. `rendering_mode != prev rendering_mode`
/// 4. RGB-normalized `fill_color != prev color`
/// 5. `is_word_boundary == true`
///
/// # Word boundary handling
///
/// When triggered by `is_word_boundary == true`, we append a space to the
/// PREVIOUS span's text (option a from the plan). This produces cleaner JSON
/// output and easier round-trip than emitting a 1-char " " span.
///
/// # Arguments
///
/// * `glyphs` - The per-page glyph list to merge
///
/// # Returns
///
/// A vector of spans, where each span represents a maximal run of glyphs
/// sharing the same font, size, color, and rendering mode.
///
/// # Examples
///
/// ```
/// use pdftract_core::span::merge_glyphs_to_spans;
/// use pdftract_core::glyph::Glyph;
/// use std::sync::Arc;
///
/// let glyphs = vec![
///     // "Hello" (5 glyphs)
///     Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
///                Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
///     // ... more glyphs for "ello World"
/// ];
///
/// let spans = merge_glyphs_to_spans(&glyphs);
/// // spans[0].text == "Hello "
/// // spans[1].text == "World"
/// ```
pub fn merge_glyphs_to_spans(glyphs: &[Glyph]) -> Vec<Span> {
    if glyphs.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut current_span: Option<Span> = None;
    let mut prev_fill_color: Option<&Color> = None;

    for glyph in glyphs {
        // Special case: word boundary marker - append space to current span, finalize it, and skip
        if glyph.is_word_boundary {
            if let Some(mut span) = current_span.take() {
                span.text.push(' ');
                result.push(span);
            }
            prev_fill_color = None; // Reset on word boundary
            // Skip the boundary marker glyph itself (it's synthetic, not a real glyph)
            continue;
        }

        // Check if we need to start a new span (no current span OR any trigger fires)
        let should_start_new_span = if let Some(ref span) = current_span {
            // Trigger 1: font_name changed
            let font_changed = &glyph.font_name != &span.font;

            // Trigger 2: font_size delta > 0.5pt
            let size_changed = (glyph.font_size - span.size).abs() > 0.5;

            // Trigger 3: rendering_mode changed
            let mode_changed = glyph.rendering_mode != span.rendering_mode;

            // Trigger 4: fill_color changed (RGB-normalized)
            let color_changed = if let Some(prev_color) = prev_fill_color {
                !colors_equal(&glyph.fill_color, prev_color)
            } else {
                false // No previous color, don't trigger
            };

            font_changed || size_changed || mode_changed || color_changed
        } else {
            true // No current span, must start new one
        };

        if should_start_new_span {
            // Finalize current span (if any)
            if let Some(span) = current_span.take() {
                result.push(span);
            }

            // Start new span from current glyph
            let confidence_source = map_unicode_source_to_confidence(glyph.unicode_source);
            let color = glyph.fill_color.to_css_hex().map(|s| CssHexColor(s));

            current_span = Some(Span::new(
                glyph.codepoint.encode_utf8(&mut [0; 4]).to_string(), // Start with this glyph's char
                glyph.bbox,
                glyph.font_name.clone(),
                glyph.font_size,
                color,
                glyph.rendering_mode,
                glyph.confidence,
                confidence_source,
                None,  // lang: filled in Phase 7
                0,     // flags: filled in Phase 4.1 flag detector
            ));
            prev_fill_color = Some(&glyph.fill_color);
        } else {
            // Append to current span
            if let Some(ref mut span) = current_span {
                // Append glyph codepoint to span text via assemble_text
                assemble_text(span, glyph);

                // Extend bbox to union
                span.bbox[0] = span.bbox[0].min(glyph.bbox[0]);
                span.bbox[1] = span.bbox[1].min(glyph.bbox[1]);
                span.bbox[2] = span.bbox[2].max(glyph.bbox[2]);
                span.bbox[3] = span.bbox[3].max(glyph.bbox[3]);

                // Update confidence_source to worst (lowest confidence) source
                // Must compare OLD confidence before updating span.confidence
                let glyph_source = map_unicode_source_to_confidence(glyph.unicode_source);
                if glyph.confidence < span.confidence {
                    span.confidence_source = glyph_source;
                }
                // Update confidence to minimum
                span.confidence = span.confidence.min(glyph.confidence);
            }
            // Update prev_fill_color to current glyph's color
            prev_fill_color = Some(&glyph.fill_color);
        }
    }

    // Push final span
    if let Some(span) = current_span {
        result.push(span);
    }

    result
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

    // Acceptance criteria tests for pdftract-3zz9n (merge_glyphs_to_spans)

    #[test]
    fn test_merge_glyphs_to_spans_hello_world_with_word_boundary() {
        // AC: Input "Hello World" (5 glyphs, space-boundary, 5 glyphs): output 2 spans "Hello " and "World"
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        let glyphs = vec![
            // "Hello" - 5 glyphs with same font/size/color
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [20.0, 10.0, 30.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [30.0, 10.0, 40.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('o', UnicodeSource::ToUnicode, 1.0, [40.0, 10.0, 50.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            // Word boundary marker (is_word_boundary = true)
            Glyph::new(' ', UnicodeSource::ToUnicode, 1.0, [50.0, 10.0, 60.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), true, None, false),
            // "World" - 5 glyphs with same font/size/color
            Glyph::new('W', UnicodeSource::ToUnicode, 1.0, [60.0, 10.0, 70.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('o', UnicodeSource::ToUnicode, 1.0, [70.0, 10.0, 80.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('r', UnicodeSource::ToUnicode, 1.0, [80.0, 10.0, 90.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [90.0, 10.0, 100.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('d', UnicodeSource::ToUnicode, 1.0, [100.0, 10.0, 110.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 2, "Expected 2 spans, got {}", spans.len());
        assert_eq!(spans[0].text, "Hello ", "First span should be 'Hello '");
        assert_eq!(spans[1].text, "World", "Second span should be 'World'");
    }

    #[test]
    fn test_merge_glyphs_to_spans_font_name_change_triggers_break() {
        // AC: Input "He" (regular) + "lo" (bold) at same font/color: 2 spans, font_name changes
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        let glyphs = vec![
            // "He" - regular Helvetica
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            // "lo" - Helvetica-Bold (font name change)
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [20.0, 10.0, 30.0, 20.0],
                       Arc::from("Helvetica-Bold"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('o', UnicodeSource::ToUnicode, 1.0, [30.0, 10.0, 40.0, 20.0],
                       Arc::from("Helvetica-Bold"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 2, "Expected 2 spans for font change");
        assert_eq!(spans[0].text, "He");
        assert_eq!(spans[0].font, Arc::from("Helvetica"));
        assert_eq!(spans[1].text, "lo");
        assert_eq!(spans[1].font, Arc::from("Helvetica-Bold"));
    }

    #[test]
    fn test_merge_glyphs_to_spans_font_size_within_threshold_no_break() {
        // AC: Input with font_size 12pt vs 12.2pt: 1 span (delta < 0.5pt)
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        let glyphs = vec![
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.2, 0, Color::DeviceGray(0.0), false, None, false), // delta = 0.2pt < 0.5
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [20.0, 10.0, 30.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 1, "Expected 1 span for size delta < 0.5pt");
        assert_eq!(spans[0].text, "Hel");
    }

    #[test]
    fn test_merge_glyphs_to_spans_font_size_exceeds_threshold_breaks() {
        // Verify that size delta > 0.5pt triggers a break
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        let glyphs = vec![
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.6, 0, Color::DeviceGray(0.0), false, None, false), // delta = 0.6pt > 0.5
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 2, "Expected 2 spans for size delta > 0.5pt");
        assert_eq!(spans[0].text, "H");
        assert_eq!(spans[1].text, "e");
    }

    #[test]
    fn test_merge_glyphs_to_spans_device_gray_and_rgb_normalized_same_color() {
        // AC: Input with DeviceGray(0.5) then DeviceRGB([0.5,0.5,0.5]): 1 span (RGB-normalized same)
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        let glyphs = vec![
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.5), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceRGB([0.5, 0.5, 0.5]), false, None, false),
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [20.0, 10.0, 30.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.5), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 1, "Expected 1 span for RGB-normalized same colors");
        assert_eq!(spans[0].text, "Hel");
        // DeviceGray(0.5) -> (0.5 * 255).round() = 128 -> #808080
        assert_eq!(spans[0].color.as_ref().unwrap().as_str(), "#808080");
    }

    #[test]
    fn test_merge_glyphs_to_spans_spot_vs_device_rgb_different_colors() {
        // AC: Input with Spot("PANTONE", 1.0) vs DeviceRGB([1,0,0]) with same hex: 2 spans (Spot != Device)
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        let glyphs = vec![
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::Spot(Arc::from("PANTONE-123"), 1.0), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceRGB([1.0, 0.0, 0.0]), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 2, "Expected 2 spans: Spot color != DeviceRGB even if visual appearance is similar");
        assert_eq!(spans[0].text, "H");
        assert_eq!(spans[0].color, None, "Spot color serializes as None");
        assert_eq!(spans[1].text, "e");
        assert_eq!(spans[1].color.as_ref().unwrap().as_str(), "#ff0000");
    }

    #[test]
    fn test_merge_glyphs_to_spans_empty_glyph_list() {
        // AC: Empty glyph list: returns empty Vec<Span> (no error)
        use crate::font::UnicodeSource;

        let glyphs: Vec<Glyph> = vec![];
        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 0);
    }

    #[test]
    fn test_merge_glyphs_to_spans_rendering_mode_change() {
        // Verify that rendering_mode change triggers a break
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        let glyphs = vec![
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 2, Color::DeviceGray(0.0), false, None, false), // mode 2
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 2, "Expected 2 spans for rendering_mode change");
        assert_eq!(spans[0].rendering_mode, 0);
        assert_eq!(spans[1].rendering_mode, 2);
    }

    #[test]
    fn test_merge_glyphs_to_spans_confidence_minimum() {
        // INV: confidence is the MINIMUM of all member glyphs' confidence
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        let glyphs = vec![
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ShapeMatch, 0.7, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('l', UnicodeSource::Agl, 0.9, [20.0, 10.0, 30.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 1);
        // Confidence should be minimum: min(1.0, 0.7, 0.9) = 0.7
        assert_eq!(spans[0].confidence, 0.7);
    }

    #[test]
    fn test_merge_glyphs_to_spans_confidence_source_worst_glyph() {
        // INV: confidence_source is mapped from the WORST glyph (lowest confidence) source
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        let glyphs = vec![
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ShapeMatch, 0.7, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 1);
        // ShapeMatch (0.7) is worse than ToUnicode (1.0), so confidence_source should be Heuristic
        assert_eq!(spans[0].confidence_source, ConfidenceSource::Heuristic);
    }

    #[test]
    fn test_merge_glyphs_to_spans_bbox_union() {
        // Verify bbox is the union of all member glyph bboxes
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        let glyphs = vec![
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [10.0, 20.0, 20.0, 30.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [25.0, 15.0, 35.0, 25.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [40.0, 18.0, 50.0, 28.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 1);
        // Bbox should be union: x0=min(10,25,40)=10, y0=min(20,15,18)=15, x1=max(20,35,50)=50, y1=max(30,25,28)=30
        assert_eq!(spans[0].bbox, [10.0, 15.0, 50.0, 30.0]);
    }

    #[test]
    fn test_merge_glyphs_to_spans_unicode_source_to_confidence_source_mapping() {
        // Verify UnicodeSource → ConfidenceSource mapping per plan
        use crate::font::UnicodeSource;
        use crate::graphics_state::Color;

        // Test ToUnicode → Native
        let glyphs = vec![
            Glyph::new('A', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];
        let spans = merge_glyphs_to_spans(&glyphs);
        assert_eq!(spans[0].confidence_source, ConfidenceSource::Native);

        // Test Agl → Native
        let glyphs = vec![
            Glyph::new('A', UnicodeSource::Agl, 0.9, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];
        let spans = merge_glyphs_to_spans(&glyphs);
        assert_eq!(spans[0].confidence_source, ConfidenceSource::Native);

        // Test Fingerprint → Native
        let glyphs = vec![
            Glyph::new('A', UnicodeSource::Fingerprint, 0.85, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];
        let spans = merge_glyphs_to_spans(&glyphs);
        assert_eq!(spans[0].confidence_source, ConfidenceSource::Native);

        // Test ShapeMatch → Heuristic
        let glyphs = vec![
            Glyph::new('A', UnicodeSource::ShapeMatch, 0.7, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];
        let spans = merge_glyphs_to_spans(&glyphs);
        assert_eq!(spans[0].confidence_source, ConfidenceSource::Heuristic);

        // Test Unknown → Heuristic
        let glyphs = vec![
            Glyph::new('A', UnicodeSource::Unknown, 0.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];
        let spans = merge_glyphs_to_spans(&glyphs);
        assert_eq!(spans[0].confidence_source, ConfidenceSource::Heuristic);
    }

    #[test]
    fn test_normalize_color_for_comparison_device_gray() {
        // Test DeviceGray normalization
        use crate::graphics_state::Color;

        let color = Color::DeviceGray(0.5);
        let normalized = normalize_color_for_comparison(&color);
        // 0.5 * 255.0 = 127.5, rounds to 128
        assert_eq!(normalized, Some((128, 128, 128)));
    }

    #[test]
    fn test_normalize_color_for_comparison_device_rgb() {
        // Test DeviceRGB normalization
        use crate::graphics_state::Color;

        let color = Color::DeviceRGB([1.0, 0.5, 0.0]);
        let normalized = normalize_color_for_comparison(&color);
        // 0.5 * 255.0 = 127.5, rounds to 128
        assert_eq!(normalized, Some((255, 128, 0)));
    }

    #[test]
    fn test_normalize_color_for_comparison_device_cmyk() {
        // Test DeviceCMYK normalization
        use crate::graphics_state::Color;

        // Cyan (C=1, M=0, Y=0, K=0) should map to RGB (0, 255, 255)
        let color = Color::DeviceCMYK([1.0, 0.0, 0.0, 0.0]);
        let normalized = normalize_color_for_comparison(&color);
        assert_eq!(normalized, Some((0, 255, 255)));
    }

    #[test]
    fn test_normalize_color_for_comparison_spot() {
        // Test Spot color returns None
        use crate::graphics_state::Color;

        let color = Color::Spot(Arc::from("PANTONE-123"), 1.0);
        let normalized = normalize_color_for_comparison(&color);
        assert_eq!(normalized, None);
    }

    #[test]
    fn test_normalize_color_for_comparison_other() {
        // Test Other color returns None
        use crate::graphics_state::Color;

        let color = Color::Other;
        let normalized = normalize_color_for_comparison(&color);
        assert_eq!(normalized, None);
    }

    #[test]
    fn test_colors_equal_device_gray_and_rgb_same() {
        // Test DeviceGray(0.5) equals DeviceRGB([0.5, 0.5, 0.5])
        use crate::graphics_state::Color;

        let gray = Color::DeviceGray(0.5);
        let rgb = Color::DeviceRGB([0.5, 0.5, 0.5]);
        assert!(colors_equal(&gray, &rgb));
    }

    #[test]
    fn test_colors_equal_device_gray_and_rgb_different() {
        // Test DeviceGray(0.5) does not equal DeviceRGB([1.0, 0.5, 0.5])
        use crate::graphics_state::Color;

        let gray = Color::DeviceGray(0.5);
        let rgb = Color::DeviceRGB([1.0, 0.5, 0.5]);
        assert!(!colors_equal(&gray, &rgb));
    }

    #[test]
    fn test_colors_equal_spot_different_names() {
        // Test Spot colors with different names are not equal
        use crate::graphics_state::Color;

        let spot1 = Color::Spot(Arc::from("PANTONE-123"), 1.0);
        let spot2 = Color::Spot(Arc::from("PANTONE-456"), 1.0);
        assert!(!colors_equal(&spot1, &spot2));
    }

    #[test]
    fn test_colors_equal_spot_same_name_different_tint() {
        // Test Spot colors with same name but different tint are not equal
        use crate::graphics_state::Color;

        let spot1 = Color::Spot(Arc::from("PANTONE-123"), 1.0);
        let spot2 = Color::Spot(Arc::from("PANTONE-123"), 0.5);
        assert!(!colors_equal(&spot1, &spot2));
    }

    #[test]
    fn test_colors_equal_spot_same_name_same_tint() {
        // Test Spot colors with same name and tint are equal
        use crate::graphics_state::Color;

        let spot1 = Color::Spot(Arc::from("PANTONE-123"), 1.0);
        let spot2 = Color::Spot(Arc::from("PANTONE-123"), 1.0);
        assert!(colors_equal(&spot1, &spot2));
    }

    #[test]
    fn test_colors_equal_spot_vs_device_rgb() {
        // Test Spot color is never equal to DeviceRGB (even if visual appearance is similar)
        use crate::graphics_state::Color;

        let spot = Color::Spot(Arc::from("PANTONE-RED"), 1.0);
        let rgb = Color::DeviceRGB([1.0, 0.0, 0.0]);
        assert!(!colors_equal(&spot, &rgb));
    }

    // Acceptance criteria tests for pdftract-2c5sx (span text assembly)

    #[test]
    fn test_assemble_text_five_glyphs_hello() {
        // AC: 5 glyphs "Hello" -> span.text == "Hello"
        use crate::font::UnicodeSource;

        let glyphs = vec![
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [20.0, 10.0, 30.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [30.0, 10.0, 40.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('o', UnicodeSource::ToUnicode, 1.0, [40.0, 10.0, 50.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Hello");
    }

    #[test]
    fn test_assemble_text_hello_world_with_boundary() {
        // AC: 5 glyphs "Hello" + boundary + 5 glyphs "World" -> span1.text == "Hello ", span2.text == "World"
        use crate::font::UnicodeSource;

        let glyphs = vec![
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [20.0, 10.0, 30.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [30.0, 10.0, 40.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('o', UnicodeSource::ToUnicode, 1.0, [40.0, 10.0, 50.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            // Word boundary
            Glyph::new(' ', UnicodeSource::ToUnicode, 1.0, [50.0, 10.0, 60.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), true, None, false),
            Glyph::new('W', UnicodeSource::ToUnicode, 1.0, [60.0, 10.0, 70.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('o', UnicodeSource::ToUnicode, 1.0, [70.0, 10.0, 80.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('r', UnicodeSource::ToUnicode, 1.0, [80.0, 10.0, 90.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('l', UnicodeSource::ToUnicode, 1.0, [90.0, 10.0, 100.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('d', UnicodeSource::ToUnicode, 1.0, [100.0, 10.0, 110.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].text, "Hello ", "First span should have trailing space");
        assert_eq!(spans[1].text, "World", "Second span should not have leading space");
    }

    #[test]
    fn test_assemble_text_ligature_fi_as_two_glyphs() {
        // AC: Ligature glyph emitting (f, i) as 2 glyphs with shared bbox: span.text == "fi"
        // Phase 2 already expands ligatures into separate glyphs, so we just verify per-glyph append works
        use crate::font::UnicodeSource;

        // Simulate a ligature that was expanded into two glyphs with shared bbox
        let shared_bbox = [0.0, 10.0, 12.0, 20.0];
        let glyphs = vec![
            Glyph::new('f', UnicodeSource::ToUnicode, 1.0, shared_bbox,
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('i', UnicodeSource::ToUnicode, 1.0, shared_bbox,
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "fi", "Ligature expansion should concatenate both codepoints");
    }

    #[test]
    fn test_assemble_text_rtl_arabic_preserved_in_source_order() {
        // AC: RTL Arabic span: text in source byte order (Phase 4.2 reorders at line level)
        // Arabic word "kitab" (book) in visual order: k-t-a-b (but stored in logical order)
        // For this test, we just verify that glyphs are appended in the order they appear
        use crate::font::UnicodeSource;

        // Arabic letters in their logical order (as they appear in the content stream)
        let glyphs = vec![
            Glyph::new('\u{0643}', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0], // keheh (k)
                       Arc::from("Arial"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('\u{062A}', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0], // teh (t)
                       Arc::from("Arial"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('\u{0627}', UnicodeSource::ToUnicode, 1.0, [20.0, 10.0, 30.0, 20.0], // alef (a)
                       Arc::from("Arial"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('\u{0628}', UnicodeSource::ToUnicode, 1.0, [30.0, 10.0, 40.0, 20.0], // beh (b)
                       Arc::from("Arial"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 1);
        // Text should be in source byte order (as glyphs appear in content stream)
        // Phase 4.2 will handle bidi reordering at the line level
        assert_eq!(spans[0].text, "\u{0643}\u{062A}\u{0627}\u{0628}");
    }

    #[test]
    fn test_assemble_text_boundary_at_start_of_page_no_space_injection() {
        // AC: Boundary at start of page: no space injection; first span starts cleanly
        use crate::font::UnicodeSource;

        // First glyph is a word boundary (odd but possible)
        let glyphs = vec![
            Glyph::new(' ', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), true, None, false),
            Glyph::new('H', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('e', UnicodeSource::ToUnicode, 1.0, [20.0, 10.0, 30.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        // Should produce one span with "He" (no leading space)
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "He", "No leading space when boundary is first glyph");
    }

    #[test]
    fn test_assemble_text_direct_call() {
        // Direct test of the assemble_text function
        use crate::font::UnicodeSource;

        let mut span = Span::empty();
        let glyph1 = Glyph::new('A', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                                 Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false);
        let glyph2 = Glyph::new('B', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0],
                                 Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false);

        assemble_text(&mut span, &glyph1);
        assert_eq!(span.text, "A");

        assemble_text(&mut span, &glyph2);
        assert_eq!(span.text, "AB");
    }

    #[test]
    fn test_assemble_text_preserves_special_unicode_chars() {
        // Verify that soft hyphen, ZWJ, ZWNJ, and U+FFFD are preserved
        use crate::font::UnicodeSource;

        let glyphs = vec![
            Glyph::new('a', UnicodeSource::ToUnicode, 1.0, [0.0, 10.0, 10.0, 20.0],
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('\u{00AD}', UnicodeSource::ToUnicode, 1.0, [10.0, 10.0, 20.0, 20.0], // soft hyphen
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('\u{200D}', UnicodeSource::ToUnicode, 1.0, [20.0, 10.0, 30.0, 20.0], // ZWJ
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('\u{200C}', UnicodeSource::ToUnicode, 1.0, [30.0, 10.0, 40.0, 20.0], // ZWNJ
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
            Glyph::new('\u{FFFD}', UnicodeSource::Unknown, 0.0, [40.0, 10.0, 50.0, 20.0], // replacement char
                       Arc::from("Helvetica"), 12.0, 0, Color::DeviceGray(0.0), false, None, false),
        ];

        let spans = merge_glyphs_to_spans(&glyphs);

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "a\u{00AD}\u{200D}\u{200C}\u{FFFD}");
    }

    // Acceptance criteria tests for pdftract-2etcd (map_confidence_source)

    #[test]
    fn test_map_confidence_source_to_unicode_without_correction() {
        // AC: ToUnicode + corrected=false → Native
        use crate::font::UnicodeSource;

        assert_eq!(
            map_confidence_source(UnicodeSource::ToUnicode, false),
            ConfidenceSource::Native
        );
    }

    #[test]
    fn test_map_confidence_source_to_unicode_with_correction() {
        // AC: ToUnicode + corrected=true → Heuristic (override applies)
        use crate::font::UnicodeSource;

        assert_eq!(
            map_confidence_source(UnicodeSource::ToUnicode, true),
            ConfidenceSource::Heuristic
        );
    }

    #[test]
    fn test_map_confidence_source_agl_without_correction() {
        // AC: Agl + corrected=false → Native
        use crate::font::UnicodeSource;

        assert_eq!(
            map_confidence_source(UnicodeSource::Agl, false),
            ConfidenceSource::Native
        );
    }

    #[test]
    fn test_map_confidence_source_agl_with_correction() {
        // AC: Agl + corrected=true → Heuristic (override applies)
        use crate::font::UnicodeSource;

        assert_eq!(
            map_confidence_source(UnicodeSource::Agl, true),
            ConfidenceSource::Heuristic
        );
    }

    #[test]
    fn test_map_confidence_source_fingerprint_without_correction() {
        // AC: Fingerprint + corrected=false → Native
        use crate::font::UnicodeSource;

        assert_eq!(
            map_confidence_source(UnicodeSource::Fingerprint, false),
            ConfidenceSource::Native
        );
    }

    #[test]
    fn test_map_confidence_source_fingerprint_with_correction() {
        // AC: Fingerprint + corrected=true → Heuristic (override applies)
        use crate::font::UnicodeSource;

        assert_eq!(
            map_confidence_source(UnicodeSource::Fingerprint, true),
            ConfidenceSource::Heuristic
        );
    }

    #[test]
    fn test_map_confidence_source_shape_match_any_correction() {
        // AC: ShapeMatch + (any) → Heuristic (correction flag doesn't matter)
        use crate::font::UnicodeSource;

        assert_eq!(
            map_confidence_source(UnicodeSource::ShapeMatch, false),
            ConfidenceSource::Heuristic
        );
        assert_eq!(
            map_confidence_source(UnicodeSource::ShapeMatch, true),
            ConfidenceSource::Heuristic
        );
    }

    #[test]
    fn test_map_confidence_source_unknown_any_correction() {
        // AC: Unknown + (any) → Heuristic (correction flag doesn't matter)
        use crate::font::UnicodeSource;

        assert_eq!(
            map_confidence_source(UnicodeSource::Unknown, false),
            ConfidenceSource::Heuristic
        );
        assert_eq!(
            map_confidence_source(UnicodeSource::Unknown, true),
            ConfidenceSource::Heuristic
        );
    }

    #[test]
    fn test_map_confidence_source_ocr_without_correction() {
        // AC: Ocr + corrected=false → Ocr (override does NOT apply to OCR)
        use crate::font::UnicodeSource;

        assert_eq!(
            map_confidence_source(UnicodeSource::Ocr, false),
            ConfidenceSource::Ocr
        );
    }

    #[test]
    fn test_map_confidence_source_ocr_with_correction() {
        // AC: Ocr + corrected=true → Ocr (override does NOT apply to OCR)
        use crate::font::UnicodeSource;

        assert_eq!(
            map_confidence_source(UnicodeSource::Ocr, true),
            ConfidenceSource::Ocr
        );
    }

    #[test]
    fn test_map_confidence_source_exhaustive_match() {
        // AC: Exhaustive match: adding a hypothetical UnicodeSource::Fallback
        // would cause a compiler error in this function until a match arm is added
        use crate::font::UnicodeSource;

        // Test all current variants
        for (source, expected_without_correction, expected_with_correction) in &[
            (UnicodeSource::ToUnicode, ConfidenceSource::Native, ConfidenceSource::Heuristic),
            (UnicodeSource::Agl, ConfidenceSource::Native, ConfidenceSource::Heuristic),
            (UnicodeSource::Fingerprint, ConfidenceSource::Native, ConfidenceSource::Heuristic),
            (UnicodeSource::ShapeMatch, ConfidenceSource::Heuristic, ConfidenceSource::Heuristic),
            (UnicodeSource::Unknown, ConfidenceSource::Heuristic, ConfidenceSource::Heuristic),
            (UnicodeSource::Ocr, ConfidenceSource::Ocr, ConfidenceSource::Ocr),
        ] {
            assert_eq!(
                map_confidence_source(*source, false),
                *expected_without_correction,
                "Without correction: {:?}",
                source
            );
            assert_eq!(
                map_confidence_source(*source, true),
                *expected_with_correction,
                "With correction: {:?}",
                source
            );
        }
    }

    #[test]
    fn test_map_confidence_source_correction_downgrades_native_to_heuristic() {
        // INV: Phase 4.7 correction ALWAYS overrides upward (Native -> Heuristic)
        // — never downward (Ocr -> Heuristic)
        use crate::font::UnicodeSource;

        // All Native sources should downgrade to Heuristic when corrected=true
        let native_sources = [
            UnicodeSource::ToUnicode,
            UnicodeSource::Agl,
            UnicodeSource::Fingerprint,
        ];

        for source in native_sources {
            assert_eq!(
                map_confidence_source(source, false),
                ConfidenceSource::Native,
                "{:?} should be Native without correction",
                source
            );
            assert_eq!(
                map_confidence_source(source, true),
                ConfidenceSource::Heuristic,
                "{:?} should downgrade to Heuristic with correction",
                source
            );
        }
    }
}
