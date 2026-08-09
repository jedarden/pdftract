//! Test helper functions for Type3 font glyph data generation.
//!
//! This module provides helper functions to create minimal valid glyph data
//! for testing Type3Font and the rasterize_type3_glyph function. These helpers
//! generate PDF content stream bytes that can be resolved by test resolvers.
//!
//! # Example
//!
//! ```rust,no_run
//! use pdftract_core::font::test_glyph_helper::{
//!     make_rect_glyph, make_line_glyph, make_test_char_procs,
//! };
//! use pdftract_core::font::type3::Type3Font;
//! use pdftract_core::parser::object::types::ObjRef;
//! use std::collections::HashMap;
//! use std::sync::Arc;
//!
//! // Create a minimal rectangle glyph
//! let rect_bytes = make_rect_glyph(10.0, 10.0, 80.0, 80.0);
//!
//! // Create test char_procs
//! let char_procs = make_test_char_procs();
//!
//! // Create font with mock
//! let font = Type3Font::mock(Some(char_procs));
//!
//! // Create resolver that returns the glyph bytes
//! let resolver = |ref_id: ObjRef| -> Option<Vec<u8>> {
//!     if ref_id.generation() == 0 && ref_id.id() as usize <= rect_bytes.len() {
//!         Some(rect_bytes.clone())
//!     } else {
//!        None
//!     }
//! };
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use crate::parser::object::types::ObjRef;

/// Create a minimal valid rectangle glyph content stream.
///
/// Generates a PDF content stream that draws a filled rectangle using the
/// `re` operator followed by `f` (fill). This is the simplest valid glyph
/// that produces visible output.
///
/// # Arguments
///
/// * `x` - X coordinate of rectangle origin (in glyph space)
/// * `y` - Y coordinate of rectangle origin (in glyph space)
/// * `width` - Rectangle width
/// * `height` - Rectangle height
///
/// # Returns
///
/// PDF content stream bytes that draw a filled rectangle.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::test_glyph_helper::make_rect_glyph;
///
/// // 100x100 square at origin
/// let glyph = make_rect_glyph(0.0, 0.0, 100.0, 100.0);
/// assert_eq!(glyph, b"0 0 100 100 re f");
/// ```
pub fn make_rect_glyph(x: f64, y: f64, width: f64, height: f64) -> Vec<u8> {
    format!("{} {} {} {} re f", x, y, width, height).into_bytes()
}

/// Create a minimal line glyph content stream.
///
/// Generates a PDF content stream that draws a line from (x1,y1) to (x2,y2).
/// Uses `m` (moveto), `l` (lineto), `h` (closepath), and `S` (stroke).
///
/// # Arguments
///
/// * `x1` - X coordinate of line start point
/// * `y1` - Y coordinate of line start point
/// * `x2` - X coordinate of line end point
/// * `y2` - Y coordinate of line end point
///
/// # Returns
///
/// PDF content stream bytes that draw a stroked line.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::test_glyph_helper::make_line_glyph;
///
/// // Diagonal line from (0,0) to (100,100)
/// let glyph = make_line_glyph(0.0, 0.0, 100.0, 100.0);
/// assert_eq!(glyph, b"0 0 m 100 100 l h S");
/// ```
pub fn make_line_glyph(x1: f64, y1: f64, x2: f64, y2: f64) -> Vec<u8> {
    format!("{} {} m {} {} l h S", x1, y1, x2, y2).into_bytes()
}

/// Create an empty glyph content stream.
///
/// Generates a minimal but empty PDF content stream. When rasterized,
/// this produces a blank (all-white) 32x32 bitmap.
///
/// # Returns
///
/// Empty PDF content stream bytes.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::test_glyph_helper::make_empty_glyph;
///
/// let glyph = make_empty_glyph();
/// assert_eq!(glyph, b"");
/// ```
pub fn make_empty_glyph() -> Vec<u8> {
    Vec::new()
}

/// Create test char_procs dictionary with common glyph names.
///
/// Creates a HashMap of glyph names to ObjRef entries suitable for
/// Type3Font::mock(). Uses object IDs starting from 10 to avoid
/// conflicts with common document object numbers.
///
/// # Glyph Names
///
/// - "A" -> ObjRef(10, 0) - Standard letter A
/// - "B" -> ObjRef(11, 0) - Standard letter B
/// - "rect" -> ObjRef(12, 0) - Rectangle glyph
/// - "line" -> ObjRef(13, 0) - Line glyph
/// - "empty" -> ObjRef(14, 0) - Empty glyph
///
/// # Returns
///
/// HashMap<Arc<str>, ObjRef> ready for Type3Font::mock()
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::test_glyph_helper::make_test_char_procs;
/// use pdftract_core::font::type3::Type3Font;
/// use std::collections::HashMap;
/// use std::sync::Arc;
///
/// let char_procs = make_test_char_procs();
/// let font = Type3Font::mock(Some(char_procs));
///
/// assert!(font.has_glyph("A"));
/// assert!(font.has_glyph("B"));
/// assert!(font.has_glyph("rect"));
/// ```
pub fn make_test_char_procs() -> HashMap<Arc<str>, ObjRef> {
    let mut char_procs = HashMap::new();
    char_procs.insert(Arc::from("A"), ObjRef::new(10, 0));
    char_procs.insert(Arc::from("B"), ObjRef::new(11, 0));
    char_procs.insert(Arc::from("rect"), ObjRef::new(12, 0));
    char_procs.insert(Arc::from("line"), ObjRef::new(13, 0));
    char_procs.insert(Arc::from("empty"), ObjRef::new(14, 0));
    char_procs
}

/// Create a custom char_procs dictionary with specified glyph names.
///
/// Helper for creating custom glyph sets. The object IDs are auto-generated
/// starting from the specified base to ensure uniqueness.
///
/// # Arguments
///
/// * `glyph_names` - Slice of glyph name strings
/// * `obj_id_base` - Starting object ID (default: 100 for custom glyphs)
///
/// # Returns
///
/// HashMap<Arc<str>, ObjRef> mapping glyph names to object references.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::test_glyph_helper::make_custom_char_procs;
/// use pdftract_core::font::type3::Type3Font;
///
/// let char_procs = make_custom_char_procs(&["g1", "g2", "g3"], 100);
/// let font = Type3Font::mock(Some(char_procs));
///
/// assert!(font.has_glyph("g1"));
/// assert!(font.has_glyph("g2"));
/// assert!(font.has_glyph("g3"));
/// ```
pub fn make_custom_char_procs(glyph_names: &[&str], obj_id_base: u32) -> HashMap<Arc<str>, ObjRef> {
    glyph_names
        .iter()
        .enumerate()
        .map(|(i, name)| (Arc::from(*name), ObjRef::new(obj_id_base + i as u32, 0)))
        .collect()
}

/// Create a test resolver that maps object IDs to glyph content.
///
/// Helper for creating resolvers for rasterize_type3_glyph tests.
/// The resolver maps a subset of object IDs to their corresponding
/// content stream bytes.
///
/// # Arguments
///
/// * `glyph_map` - HashMap mapping object IDs to content stream bytes
///
/// # Returns
///
/// A resolver function compatible with rasterize_type3_glyph.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::test_glyph_helper::{
///     make_rect_glyph, make_test_resolver,
/// };
/// use pdftract_core::parser::object::types::ObjRef;
/// use std::collections::HashMap;
///
/// // Create glyph data
/// let mut glyph_map = HashMap::new();
/// glyph_map.insert(10, make_rect_glyph(0.0, 0.0, 100.0, 100.0));
/// glyph_map.insert(11, make_rect_glyph(50.0, 50.0, 100.0, 100.0));
///
/// // Create resolver
/// let resolver = make_test_resolver(&glyph_map);
///
/// // Use with rasterize_type3_glyph
/// let bytes = resolver(ObjRef::new(10, 0));
/// assert!(bytes.is_some());
/// ```
pub fn make_test_resolver(
    glyph_map: &HashMap<u32, Vec<u8>>,
) -> impl Fn(ObjRef) -> Option<Vec<u8>> + 'static {
    let glyph_map = glyph_map.clone();
    move |obj_ref: ObjRef| {
        if obj_ref.generation == 0 {
            glyph_map.get(&obj_ref.object).cloned()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_rect_glyph() {
        let glyph = make_rect_glyph(10.0, 20.0, 100.0, 200.0);
        assert_eq!(glyph, b"10 20 100 200 re f");
    }

    #[test]
    fn test_make_line_glyph() {
        let glyph = make_line_glyph(0.0, 0.0, 50.0, 50.0);
        assert_eq!(glyph, b"0 0 m 50 50 l h S");
    }

    #[test]
    fn test_make_empty_glyph() {
        let glyph = make_empty_glyph();
        assert!(glyph.is_empty());
    }

    #[test]
    fn test_make_test_char_procs() {
        let char_procs = make_test_char_procs();
        assert_eq!(char_procs.len(), 5);
        assert!(char_procs.contains_key("A"));
        assert!(char_procs.contains_key("B"));
        assert!(char_procs.contains_key("rect"));
        assert!(char_procs.contains_key("line"));
        assert!(char_procs.contains_key("empty"));
    }

    #[test]
    fn test_make_custom_char_procs() {
        let char_procs = make_custom_char_procs(&["g1", "g2", "g3"], 100);
        assert_eq!(char_procs.len(), 3);
        assert_eq!(char_procs.get("g1"), Some(&ObjRef::new(100, 0)));
        assert_eq!(char_procs.get("g2"), Some(&ObjRef::new(101, 0)));
        assert_eq!(char_procs.get("g3"), Some(&ObjRef::new(102, 0)));
    }

    #[test]
    fn test_make_test_resolver() {
        let mut glyph_map = HashMap::new();
        glyph_map.insert(10, make_rect_glyph(0.0, 0.0, 100.0, 100.0));
        glyph_map.insert(11, make_line_glyph(0.0, 0.0, 50.0, 50.0));

        let resolver = make_test_resolver(&glyph_map);

        // Test valid references
        assert_eq!(
            resolver(ObjRef::new(10, 0)),
            Some(b"0 0 100 100 re f".to_vec())
        );
        assert_eq!(
            resolver(ObjRef::new(11, 0)),
            Some(b"0 0 m 50 50 l h S".to_vec())
        );

        // Test invalid references
        assert!(resolver(ObjRef::new(99, 0)).is_none());
        assert!(resolver(ObjRef::new(10, 1)).is_none()); // Wrong generation
    }
}
