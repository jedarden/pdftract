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
/// Returns an empty content stream (no drawing operations). This is useful
/// for testing fonts with glyphs that have no visible content (space characters,
/// zero-width joiners, etc.).
///
/// # Returns
///
/// Empty PDF content stream bytes.
pub fn make_empty_glyph() -> Vec<u8> {
    Vec::new()
}

/// Create a rectangle glyph with custom path commands.
///
/// Similar to `make_rect_glyph` but uses individual path operators instead of
/// the combined `re` operator. This allows testing that the rasterizer handles
/// both the `re` shorthand and explicit `moveto`/`lineto` sequences correctly.
///
/// # Arguments
///
/// * `x` - X coordinate of rectangle origin
/// * `y` - Y coordinate of rectangle origin
/// * `width` - Rectangle width
/// * `height` - Rectangle height
///
/// # Returns
///
/// PDF content stream bytes that draw a filled rectangle using explicit path commands.
pub fn make_rect_glyph_with_path_commands(x: f64, y: f64, width: f64, height: f64) -> Vec<u8> {
    format!(
        "{} {} m {} {} l {} {} l {} {} l h f",
        x,
        y,
        x + width,
        y,
        x + width,
        y + height,
        x,
        y + height
    )
    .into_bytes()
}

/// Create a test char_procs mapping.
///
/// Creates a HashMap mapping character names to their ObjRef IDs. This provides
/// a simple mapping for testing where character "/A" maps to ID 1, "/B" to ID 2, etc.
///
/// # Returns
///
/// HashMap of character names to ObjRefs for testing.
pub fn make_test_char_procs() -> HashMap<Arc<str>, ObjRef> {
    let mut char_procs = HashMap::new();
    char_procs.insert("/A".into(), ObjRef::new(1, 0));
    char_procs.insert("/B".into(), ObjRef::new(2, 0));
    char_procs.insert("/C".into(), ObjRef::new(3, 0));
    char_procs.insert("/D".into(), ObjRef::new(4, 0));
    char_procs
}

/// Create a test resolver function from a glyph map.
///
/// Creates a resolver closure that looks up glyph content bytes by character name.
/// The resolver takes an ObjRef and returns the corresponding glyph bytes if found.
///
/// # Arguments
///
/// * `glyph_map` - HashMap mapping character names (e.g., "/A") to glyph content bytes
///
/// # Returns
///
/// A resolver function that takes an ObjRef and returns Option<Vec<u8>>.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::test_glyph_helper::{make_rect_glyph, make_test_resolver};
/// use std::collections::HashMap;
///
/// let mut glyph_map = HashMap::new();
/// glyph_map.insert("/A".to_string(), make_rect_glyph(0.0, 0.0, 100.0, 100.0));
///
/// let resolver = make_test_resolver(&glyph_map);
///
/// // Assuming ObjRef(1, 0) maps to character /A in the char_procs
/// let glyph_bytes = resolver(ObjRef::new(1, 0));
/// assert!(glyph_bytes.is_some());
/// ```
pub fn make_test_resolver(glyph_map: &HashMap<String, Vec<u8>>) -> impl Fn(ObjRef) -> Option<Vec<u8>> + '_ {
    let glyph_map = Arc::new(glyph_map.clone());
    move |ref_id: ObjRef| {
        // Map ObjRef ID to character name: ID 1 -> "/A", ID 2 -> "/B", etc.
        let char_name = format!("/{}", (ref_id.object as u8 + b'A' - 1) as char);
        glyph_map.get(&char_name).cloned()
    }
}

/// Create custom char_procs with specific character names to ObjRef mappings.
///
/// This function allows creating a custom char_procs mapping for testing scenarios
/// that need non-standard character names or ObjRef IDs.
///
/// # Arguments
///
/// * `mappings` - Slice of (character_name, obj_ref_id) tuples
///
/// # Returns
///
/// HashMap of character names to ObjRefs.
///
/// # Example
///
/// ```rust,no_run
/// use pdftract_core::font::test_glyph_helper::make_custom_char_procs;
/// use pdftract_core::parser::object::types::ObjRef;
///
/// let char_procs = make_custom_char_procs(&[
///     ("/space", ObjRef::new(1, 0)),
///     ("/A", ObjRef::new(2, 0)),
///     ("/Z", ObjRef::new(26, 0)),
/// ]);
/// ```
pub fn make_custom_char_procs(mappings: &[(&str, ObjRef)]) -> HashMap<Arc<str>, ObjRef> {
    mappings
        .iter()
        .map(|(name, ref_id)| ((*name).into(), *ref_id))
        .collect()
}

/// Create char_procs from glyph names and base ID.
///
/// Convenience function that creates a HashMap mapping glyph names to ObjRefs,
/// automatically generating sequential IDs starting from a base value.
///
/// # Arguments
///
/// * `glyph_names` - Slice of glyph names (e.g., &["g1", "g2", "g3"])
/// * `base_id` - Starting ObjRef ID (will use base_id, base_id+1, base_id+2, ...)
///
/// # Returns
///
/// HashMap of glyph names to ObjRefs for testing.
pub fn make_custom_char_procs_from_names(glyph_names: &[&str], base_id: u32) -> HashMap<Arc<str>, ObjRef> {
    glyph_names
        .iter()
        .enumerate()
        .map(|(i, name)| ((*name).into(), ObjRef::new(base_id + i as u32, 0)))
        .collect()
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
    fn test_make_rect_glyph_with_path_commands() {
        let glyph = make_rect_glyph_with_path_commands(5.0, 5.0, 95.0, 95.0);
        // Should be: 5 5 m 100 5 l 100 100 l 5 100 l h f
        assert!(glyph.starts_with(b"5 5 m"));
        assert!(glyph.ends_with(b"h f"));
    }

    #[test]
    fn test_make_test_char_procs() {
        let char_procs = make_test_char_procs();
        assert_eq!(char_procs.get("/A"), Some(&ObjRef::new(1, 0)));
        assert_eq!(char_procs.get("/B"), Some(&ObjRef::new(2, 0)));
        assert_eq!(char_procs.get("/C"), Some(&ObjRef::new(3, 0)));
        assert_eq!(char_procs.get("/D"), Some(&ObjRef::new(4, 0)));
        assert_eq!(char_procs.len(), 4);
    }

    #[test]
    fn test_make_custom_char_procs() {
        let char_procs = make_custom_char_procs(&[
            ("/space", ObjRef::new(1, 0)),
            ("/A", ObjRef::new(2, 0)),
        ]);

        assert_eq!(char_procs.get("/space"), Some(&ObjRef::new(1, 0)));
        assert_eq!(char_procs.get("/A"), Some(&ObjRef::new(2, 0)));
        assert_eq!(char_procs.len(), 2);
    }

    #[test]
    fn test_make_test_resolver() {
        let mut glyph_map = HashMap::new();
        glyph_map.insert("/A".to_string(), make_rect_glyph(0.0, 0.0, 100.0, 100.0));
        glyph_map.insert("/B".to_string(), make_line_glyph(0.0, 0.0, 50.0, 50.0));

        let resolver = make_test_resolver(&glyph_map);

        // Test ID 1 maps to "/A" (ASCII A is 65, so 65 - 65 + 1 = 1)
        let result = resolver(ObjRef::new(1, 0));
        assert_eq!(result, Some(make_rect_glyph(0.0, 0.0, 100.0, 100.0)));

        // Test ID 2 maps to "/B" (ASCII B is 66, so 66 - 65 + 1 = 2)
        let result = resolver(ObjRef::new(2, 0));
        assert_eq!(result, Some(make_line_glyph(0.0, 0.0, 50.0, 50.0)));

        // Test non-existent ID
        let result = resolver(ObjRef::new(99, 0));
        assert!(result.is_none());
    }
}
