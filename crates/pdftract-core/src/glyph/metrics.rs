//! Font metrics for glyph advance widths and bounding boxes.
//!
//! This module provides a unified interface for accessing font metrics
//! across different font types (Std14, Type1, TrueType, Type0, Type3).

use crate::parser::object::types::PdfDict;

/// Advance width and bbox metrics for a font.
pub trait FontMetrics {
    /// Get the advance width for a character code in font units.
    fn advance(&self, char_code: u32) -> u16;

    /// Get the bounding box for a character code in font units.
    ///
    /// Returns [x_min, y_min, x_max, y_max].
    fn glyph_bbox(&self, char_code: u32) -> [f64; 4];
}

/// No-op placeholder for metrics module.
/// Actual metrics lookup is in text/mod.rs for now.
pub fn get_advance_from_dict(_font_dict: &PdfDict, _char_code: u32) -> u16 {
    500 // Default width
}
