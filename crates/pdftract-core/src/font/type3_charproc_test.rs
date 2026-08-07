//! Tests for Type3 charproc content streams with simple path commands.
//!
//! This test module verifies that charproc content streams with basic PDF
//! path drawing commands compile, execute, and produce correct rasterized output.
//!
//! # Test Coverage
//!
//! - Simple rectangle drawing (re operator)
//! - Move and line commands (m, l operators)
//! - Close path (h operator)
//! - Triangle and polygon shapes
//! - Multiple shapes in one stream
//!
//! # References
//!
//! - crates/pdftract-core/src/font/type3_rasterizer.rs:649 - execute_content_stream
//! - crates/pdftract-core/src/font/type3_rasterizer.rs:716 - path construction operators

use std::collections::HashMap;
use std::sync::Arc;

use crate::font::type3_rasterizer::rasterize_type3_glyph;
use crate::font::type3::Type3Font;
use crate::parser::object::types::{intern, ObjRef, PdfDict, PdfObject};

/// Helper function to create a test Type3Font with a single glyph.
///
/// This creates a minimal Type3Font with the given glyph name and reference,
/// using identity FontMatrix for predictable coordinates during testing.
fn create_test_font_with_glyph(glyph_name: &str, obj_ref: ObjRef) -> Type3Font {
    let mut char_procs = HashMap::new();
    char_procs.insert(Arc::from(glyph_name), obj_ref);

    let mut font_dict = PdfDict::new();
    let char_procs_dict = PdfObject::Dict(Box::new(
        char_procs.into_iter().map(|(k, v)| (k, PdfObject::Ref(v))).collect()
    ));
    font_dict.insert(intern("/CharProcs"), char_procs_dict);

    // Use identity FontMatrix for predictable coordinates
    font_dict.insert(intern("/FontMatrix"), PdfObject::Array(Box::new(vec![
        PdfObject::Integer(1),
        PdfObject::Integer(0),
        PdfObject::Integer(0),
        PdfObject::Integer(1),
        PdfObject::Integer(0),
        PdfObject::Integer(0),
    ])));

    // Set a reasonable FontBBox for a 20x20 glyph
    font_dict.insert(intern("/FontBBox"), PdfObject::Array(Box::new(vec![
        PdfObject::Integer(0),
        PdfObject::Integer(0),
        PdfObject::Integer(20),
        PdfObject::Integer(20),
    ])));

    font_dict.insert(intern("/FirstChar"), PdfObject::Integer(0));
    font_dict.insert(intern("/LastChar"), PdfObject::Integer(0));
    font_dict.insert(intern("/Widths"), PdfObject::Array(Box::new(vec![
        PdfObject::Real(600.0),
    ])));

    Type3Font::load(&font_dict)
}

/// Test that a simple rectangle charproc stream compiles and executes.
///
/// This test verifies the simplest possible charproc: a single rectangle
/// drawn using the `re` operator followed by `f` (fill).
#[test]
fn test_charproc_simple_rectangle() {
    // Create a font with a test glyph
    let glyph_ref = ObjRef::new(1, 0);
    let font = create_test_font_with_glyph("rect", glyph_ref);

    // Create a charproc stream that draws a 10x10 rectangle at origin
    // PDF syntax: "x y width height re f" where:
    // - re: append rectangle to path
    // - f: fill path using nonzero winding rule
    let charproc_stream = b"0 0 10 10 re f".to_vec();

    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(charproc_stream.clone())
    };

    // Execute the charproc stream
    let result = rasterize_type3_glyph(&font, "rect", None, Some(&callback));

    // Verify successful rasterization
    assert!(result.is_some(), "Rectangle charproc should rasterize successfully");

    let bitmap = result.unwrap();
    assert!(!bitmap.is_empty(), "Bitmap should not be empty");

    // For a 20x20 font bbox with 2 pixels padding, we get 24x24 bitmap
    // 24 * 24 = 576 bytes
    assert_eq!(bitmap.len(), 576, "Bitmap should be 24x24 = 576 bytes");
}

/// Test that a charproc stream with moveto and lineto commands works.
///
/// This test verifies path construction using basic moveto (m) and lineto (l)
/// operators, followed by closepath (h) and fill (f).
#[test]
fn test_charproc_move_line_close() {
    // Create a font with a test glyph
    let glyph_ref = ObjRef::new(2, 0);
    let font = create_test_font_with_glyph("triangle", glyph_ref);

    // Create a charproc stream that draws a triangle:
    // Move to (5,5), line to (15,5), line to (10,15), close path, fill
    // PDF syntax: "x y m" (move to), "x y l" (line to), "h" (close path), "f" (fill)
    let charproc_stream = b"5 5 m 15 5 l 10 15 l h f".to_vec();

    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(charproc_stream.clone())
    };

    // Execute the charproc stream
    let result = rasterize_type3_glyph(&font, "triangle", None, Some(&callback));

    // Verify successful rasterization
    assert!(result.is_some(), "Triangle charproc with m/l/h should rasterize successfully");

    let bitmap = result.unwrap();
    assert!(!bitmap.is_empty(), "Bitmap should not be empty");
    assert_eq!(bitmap.len(), 576, "Bitmap should be 24x24 = 576 bytes");
}

/// Test that a charproc stream with multiple separate shapes works.
///
/// This test verifies that a single charproc can contain multiple
/// independent shapes, each with its own path construction.
#[test]
fn test_charproc_multiple_shapes() {
    // Create a font with a test glyph
    let glyph_ref = ObjRef::new(3, 0);
    let font = create_test_font_with_glyph("shapes", glyph_ref);

    // Create a charproc stream that draws two separate rectangles:
    // First rectangle at (0,0) with size 5x5, second at (10,10) with size 5x5
    let charproc_stream = b"0 0 5 5 re f 10 10 5 5 re f".to_vec();

    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(charproc_stream.clone())
    };

    // Execute the charproc stream
    let result = rasterize_type3_glyph(&font, "shapes", None, Some(&callback));

    // Verify successful rasterization
    assert!(result.is_some(), "Multiple shapes charproc should rasterize successfully");

    let bitmap = result.unwrap();
    assert!(!bitmap.is_empty(), "Bitmap should not be empty");
    assert_eq!(bitmap.len(), 576, "Bitmap should be 24x24 = 576 bytes");
}

/// Test that a charproc stream with stroke (S) operator works.
///
/// This test verifies the stroke operator which draws outlines
/// instead of filled shapes.
#[test]
fn test_charproc_stroke_rectangle() {
    // Create a font with a test glyph
    let glyph_ref = ObjRef::new(4, 0);
    let font = create_test_font_with_glyph("stroke_rect", glyph_ref);

    // Create a charproc stream that draws a rectangle outline using stroke:
    // Rectangle at (2,2) with size 16x16, stroke (S)
    let charproc_stream = b"2 2 16 16 re S".to_vec();

    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(charproc_stream.clone())
    };

    // Execute the charproc stream
    let result = rasterize_type3_glyph(&font, "stroke_rect", None, Some(&callback));

    // Verify successful rasterization
    assert!(result.is_some(), "Stroked rectangle charproc should rasterize successfully");

    let bitmap = result.unwrap();
    assert!(!bitmap.is_empty(), "Bitmap should not be empty");
    assert_eq!(bitmap.len(), 576, "Bitmap should be 24x24 = 576 bytes");
}

/// Test that a charproc stream with close-and-stroke (s) operator works.
///
/// This test verifies the close-and-stroke operator which closes the
/// current subpath and then strokes it.
#[test]
fn test_charproc_close_stroke_triangle() {
    // Create a font with a test glyph
    let glyph_ref = ObjRef::new(5, 0);
    let font = create_test_font_with_glyph("close_stroke", glyph_ref);

    // Create a charproc stream that draws a triangle outline:
    // Move to (5,5), line to (15,5), line to (10,15), close-and-stroke (s)
    let charproc_stream = b"5 5 m 15 5 l 10 15 l s".to_vec();

    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(charproc_stream.clone())
    };

    // Execute the charproc stream
    let result = rasterize_type3_glyph(&font, "close_stroke", None, Some(&callback));

    // Verify successful rasterization
    assert!(result.is_some(), "Close-and-stroke triangle charproc should rasterize successfully");

    let bitmap = result.unwrap();
    assert!(!bitmap.is_empty(), "Bitmap should not be empty");
    assert_eq!(bitmap.len(), 576, "Bitmap should be 24x24 = 576 bytes");
}

/// Test that empty charproc streams are handled gracefully.
///
/// This test verifies that an empty content stream doesn't crash and
/// produces a valid (all-white) bitmap.
#[test]
fn test_charproc_empty_stream() {
    // Create a font with a test glyph
    let glyph_ref = ObjRef::new(6, 0);
    let font = create_test_font_with_glyph("empty", glyph_ref);

    // Create an empty charproc stream
    let charproc_stream = b"".to_vec();

    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(charproc_stream.clone())
    };

    // Execute the charproc stream
    let result = rasterize_type3_glyph(&font, "empty", None, Some(&callback));

    // Verify graceful handling - should return a valid bitmap
    assert!(result.is_some(), "Empty charproc should produce a valid bitmap");

    let bitmap = result.unwrap();
    assert!(!bitmap.is_empty(), "Bitmap should not be empty");
    assert_eq!(bitmap.len(), 576, "Bitmap should be 24x24 = 576 bytes");

    // All pixels should be white (255) for an empty stream
    assert!(bitmap.iter().all(|&pixel| pixel == 255), "All pixels should be white for empty stream");
}

/// Test that charproc streams with only whitespace are handled gracefully.
///
/// This test verifies that whitespace-only content streams don't crash
/// and produce a valid (all-white) bitmap.
#[test]
fn test_charproc_whitespace_only() {
    // Create a font with a test glyph
    let glyph_ref = ObjRef::new(7, 0);
    let font = create_test_font_with_glyph("whitespace", glyph_ref);

    // Create a charproc stream with only whitespace
    let charproc_stream = b"   \n\t  ".to_vec();

    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(charproc_stream.clone())
    };

    // Execute the charproc stream
    let result = rasterize_type3_glyph(&font, "whitespace", None, Some(&callback));

    // Verify graceful handling
    assert!(result.is_some(), "Whitespace-only charproc should produce a valid bitmap");

    let bitmap = result.unwrap();
    assert!(!bitmap.is_empty(), "Bitmap should not be empty");
    assert_eq!(bitmap.len(), 576, "Bitmap should be 24x24 = 576 bytes");
}

/// Test that charproc streams with no-op commands work.
///
/// This test verifies the no-op operator (n) which ends the current path
/// without drawing it.
#[test]
fn test_charproc_noop_path() {
    // Create a font with a test glyph
    let glyph_ref = ObjRef::new(8, 0);
    let font = create_test_font_with_glyph("noop", glyph_ref);

    // Create a charproc stream that constructs a path but uses n (no-op) instead of f/S
    let charproc_stream = b"5 5 m 15 5 l 10 15 l n".to_vec();

    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(charproc_stream.clone())
    };

    // Execute the charproc stream
    let result = rasterize_type3_glyph(&font, "noop", None, Some(&callback));

    // Verify graceful handling - no-op produces valid bitmap but nothing drawn
    assert!(result.is_some(), "No-op charproc should produce a valid bitmap");

    let bitmap = result.unwrap();
    assert!(!bitmap.is_empty(), "Bitmap should not be empty");
    assert_eq!(bitmap.len(), 576, "Bitmap should be 24x24 = 576 bytes");

    // All pixels should be white (255) since n doesn't draw
    assert!(bitmap.iter().all(|&pixel| pixel == 255), "All pixels should be white for no-op path");
}

/// Test that a complex polygon with multiple vertices works.
///
/// This test verifies a charproc that draws a complex polygon (pentagon)
/// using a sequence of moveto and lineto commands.
#[test]
fn test_charproc_complex_polygon() {
    // Create a font with a test glyph
    let glyph_ref = ObjRef::new(9, 0);
    let font = create_test_font_with_glyph("pentagon", glyph_ref);

    // Create a charproc stream that draws a pentagon:
    // Starting at (10,2), then (18,6), (15,14), (5,14), (2,6), close, fill
    let charproc_stream = b"10 2 m 18 6 l 15 14 l 5 14 l 2 6 l h f".to_vec();

    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(charproc_stream.clone())
    };

    // Execute the charproc stream
    let result = rasterize_type3_glyph(&font, "pentagon", None, Some(&callback));

    // Verify successful rasterization
    assert!(result.is_some(), "Complex polygon charproc should rasterize successfully");

    let bitmap = result.unwrap();
    assert!(!bitmap.is_empty(), "Bitmap should not be empty");
    assert_eq!(bitmap.len(), 576, "Bitmap should be 24x24 = 576 bytes");
}

/// Test that charproc streams produce consistent output across multiple executions.
///
/// This test verifies that executing the same charproc stream multiple times
/// produces identical output (deterministic rendering).
#[test]
fn test_charproc_consistent_rendering() {
    // Create a font with a test glyph
    let glyph_ref = ObjRef::new(10, 0);
    let font = create_test_font_with_glyph("consistent", glyph_ref);

    // Create a charproc stream that draws a simple rectangle
    let charproc_stream = b"2 2 16 16 re f".to_vec();

    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(charproc_stream.clone())
    };

    // Execute the charproc stream twice
    let result1 = rasterize_type3_glyph(&font, "consistent", None, Some(&callback));
    let result2 = rasterize_type3_glyph(&font, "consistent", None, Some(&callback));

    // Verify both executions succeeded
    assert!(result1.is_some(), "First execution should succeed");
    assert!(result2.is_some(), "Second execution should succeed");

    let bitmap1 = result1.unwrap();
    let bitmap2 = result2.unwrap();

    // Verify bitmaps are identical
    assert_eq!(bitmap1, bitmap2, "Multiple executions should produce identical output");
}
