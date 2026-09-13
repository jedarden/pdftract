//! Shape-level verification for Type3 glyph rasterization.
//!
//! The earlier tests for `rasterize_type3_glyph` only asserted that a bitmap
//! came back and had the expected length, so a hardcoded placeholder bitmap
//! would have passed them. These tests pin the actual pixel geometry instead:
//! for a known charproc stream, ink must land exactly where the path was drawn
//! and nowhere else.
//!
//! Geometry is kept exact by using an identity FontMatrix and a 20x20
//! FontBBox, which yields a 22x22 bitmap (one pixel of padding on each side,
//! per `calculate_bitmap_dimensions`). Glyph-space coordinates therefore map
//! straight onto bitmap pixel indices with no scaling or flipping.
//!
//! # References
//!
//! - crates/pdftract-core/src/font/type3_rasterizer.rs - rasterize_type3_glyph
//! - crates/pdftract-core/src/font/type3_rasterizer.rs - fill_polygon (scanline fill)
//! - docs/plan/plan.md Phase 2.5 Level 4 (Type3 glyph rasterization)

use std::collections::HashMap;
use std::sync::Arc;

use pdftract_core::font::type3::Type3Font;
use pdftract_core::font::type3_rasterizer::{
    rasterize_type3_glyph, DocumentContext, StreamResolverFn,
};
use pdftract_core::parser::object::types::ObjRef;
use pdftract_core::parser::stream::MemorySource;
use pdftract_core::parser::xref::{XrefEntry, XrefResolver};

/// Glyph space extent covered by the test FontBBox.
const BBOX_MAX: i32 = 20;
/// Padding applied by `calculate_bitmap_dimensions` (defaults to 1).
const PADDING: i32 = 1;
/// Bitmap edge length for a 20x20 FontBBox: 20 + 2 * 1.
const DIM: i32 = BBOX_MAX + 2 * PADDING;

/// Build a Type3 font whose glyph space is 1:1 with bitmap pixels.
///
/// Identity FontMatrix plus a [0,0,20,20] FontBBox means a point (x, y) in a
/// charproc stream lands on bitmap pixel (x, y), so expectations below can be
/// written in glyph-space coordinates directly.
fn font_with_glyphs(glyphs: &[(&str, u32)]) -> Type3Font {
    let char_procs: HashMap<Arc<str>, ObjRef> = glyphs
        .iter()
        .map(|(name, obj)| (Arc::from(*name), ObjRef::new(*obj, 0)))
        .collect();

    Type3Font::type3_font_full()
        .with_char_procs(char_procs)
        .with_font_bbox([0.0, 0.0, BBOX_MAX as f32, BBOX_MAX as f32])
        .build()
}

/// Rasterize `stream` as the content of the single glyph `name`.
fn rasterize(font: &Type3Font, name: &str, stream: &[u8]) -> Option<Vec<u8>> {
    let owned = stream.to_vec();
    let resolver = move |_reference: ObjRef| -> Option<Vec<u8>> { Some(owned.clone()) };
    rasterize_type3_glyph(font, name, None, Some(&resolver))
}

/// Collect the coordinates of every inked (black) pixel.
///
/// Returns `(x, y)` pairs sorted by row then column, so callers can assert on
/// both the extent and the exact membership of the inked region.
fn black_pixels(bitmap: &[u8]) -> Vec<(i32, i32)> {
    let mut inked = Vec::new();
    for y in 0..DIM {
        for x in 0..DIM {
            if bitmap[(y * DIM + x) as usize] == 0 {
                inked.push((x, y));
            }
        }
    }
    inked
}

/// Assert the inked region is exactly the inclusive rectangle x0..=x1, y0..=y1.
///
/// Only valid for a glyph whose entire ink is one rectangle; see
/// [`assert_region_inked`] for a per-region check that tolerates other ink
/// elsewhere in the bitmap.
fn assert_inked_exactly(bitmap: &[u8], x0: i32, y0: i32, x1: i32, y1: i32) {
    let expected: Vec<(i32, i32)> = (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).collect();
    let actual = black_pixels(bitmap);

    assert_eq!(
        actual.len(),
        expected.len(),
        "inked pixel count mismatch: got {:?} .. {:?}, wanted {:?} .. {:?}",
        actual.first(),
        actual.last(),
        expected.first(),
        expected.last()
    );
    for (index, pixel) in actual.iter().enumerate() {
        assert_eq!(
            *pixel, expected[index],
            "inked pixel set differs at index {index}"
        );
    }
}

/// Assert every pixel of the inclusive rectangle x0..=x1, y0..=y1 is inked,
/// without requiring the rest of the bitmap to be white.
fn assert_region_inked(bitmap: &[u8], x0: i32, y0: i32, x1: i32, y1: i32) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            assert_eq!(
                bitmap[(y * DIM + x) as usize],
                0,
                "pixel ({x}, {y}) should be inked"
            );
        }
    }
}

/// Assert every pixel of the inclusive rectangle x0..=x1, y0..=y1 is white.
fn assert_region_white(bitmap: &[u8], x0: i32, y0: i32, x1: i32, y1: i32) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            assert_eq!(
                bitmap[(y * DIM + x) as usize],
                255,
                "pixel ({x}, {y}) should be white"
            );
        }
    }
}

/// A filled rectangle charproc must rasterize to ink over exactly that rectangle.
#[test]
fn filled_rectangle_lands_on_drawn_extent() {
    let font = font_with_glyphs(&[("rect", 1)]);
    // "x y w h re f" draws from (5,5) to (15,15); the scanline fill is
    // endpoint-inclusive, so the inked run is 11x11 rather than 10x10.
    let bitmap = rasterize(&font, "rect", b"5 5 10 10 re f").expect("rectangle should rasterize");

    assert_eq!(bitmap.len(), (DIM * DIM) as usize, "bitmap is 22x22 bytes");
    assert_inked_exactly(&bitmap, 5, 5, 15, 15);
}

/// Ink must not bleed outside the drawn rectangle.
#[test]
fn rectangle_leaves_background_untouched() {
    let font = font_with_glyphs(&[("rect", 1)]);
    let bitmap = rasterize(&font, "rect", b"5 5 10 10 re f").expect("rectangle should rasterize");

    // Corners and the padding ring are outside the drawn path.
    for (x, y) in [(0, 0), (DIM - 1, 0), (0, DIM - 1), (DIM - 1, DIM - 1), (4, 4), (16, 16)] {
        assert_eq!(
            bitmap[(y * DIM + x) as usize],
            255,
            "pixel ({x}, {y}) is outside the drawn rectangle and should stay white"
        );
    }
}

/// Two separate rectangles must each be inked, with the gap between them white.
#[test]
fn multiple_shapes_stay_separate() {
    let font = font_with_glyphs(&[("pair", 1)]);
    // Two 4x4 squares: one at the origin, one at (12,12). Endpoint-inclusive
    // fill makes each an inked 5x5 block.
    let bitmap = rasterize(&font, "pair", b"0 0 4 4 re f 12 12 4 4 re f")
        .expect("two-square charproc should rasterize");

    // Together the squares account for 2 * 5 * 5 inked pixels.
    assert_eq!(
        black_pixels(&bitmap).len(),
        50,
        "two disjoint 5x5 squares should ink exactly 50 pixels"
    );
    assert_region_inked(&bitmap, 0, 0, 4, 4);
    assert_region_inked(&bitmap, 12, 12, 16, 16);

    // The region between the squares must be white.
    assert_region_white(&bitmap, 5, 5, 11, 11);
}

/// Slanted paths must be traceable, and filling must stay bounded by them.
///
/// This test deliberately asserts stroke geometry rather than fill geometry:
/// `draw_line` rasterizes diagonals with Bresenham and so tracks the true
/// slope, whereas `fill_polygon` advances each active edge's x by
/// `(dx / dy) as i32` per scanline. That truncation discards the fractional
/// part, so an edge with |dx| < |dy| never moves left or right at all and a
/// filled triangle comes out as a rectangle. Slanted *fill* geometry therefore
/// cannot be asserted until that accumulation is fixed (see bead
/// pdftract-type3-aet-slope).
#[test]
fn slanted_stroke_tracks_both_endpoints() {
    let font = font_with_glyphs(&[("diag", 1)]);
    // A single diagonal stroke across the glyph space.
    let bitmap = rasterize(&font, "diag", b"5 15 m 15 5 l S")
        .expect("diagonal stroke should rasterize");

    let inked = black_pixels(&bitmap);
    assert!(!inked.is_empty(), "diagonal stroke should deposit ink");

    // The stroke is a 1-pixel line, so it must ink far fewer pixels than the
    // 11x11 = 121 a filled rectangle over the same extent would.
    assert!(
        inked.len() <= 22,
        "a 1px diagonal should ink at most ~22 pixels, got {}",
        inked.len()
    );

    // Both declared endpoints must be inked, and nothing may fall outside the
    // segment's bounding box.
    for (x, y) in [(5, 15), (15, 5)] {
        assert_eq!(
            bitmap[(y * DIM + x) as usize],
            0,
            "declared endpoint ({x}, {y}) should be inked"
        );
    }
    for (x, y) in &inked {
        assert!(
            (5..=15).contains(x) && (5..=15).contains(y),
            "inked pixel ({x}, {y}) lies outside the stroke's bounding box"
        );
    }
}

/// A filled triangle must ink fewer pixels than a rectangle over the same
/// bounding box, and must not exceed that bounding box.
///
/// This holds even with the slanted-edge fill defect described above: the
/// over-wide rectangle the defect produces is still narrower than the full
/// bounding box, so the bounding-box bound and the area bound are meaningful
/// without pinning the exact hypotenuse.
#[test]
fn filled_triangle_stays_within_its_bounding_box() {
    let font = font_with_glyphs(&[("tri", 1)]);
    let bitmap = rasterize(&font, "tri", b"5 15 m 15 15 l 10 5 l h f")
        .expect("triangle charproc should rasterize");

    let inked = black_pixels(&bitmap);
    assert!(!inked.is_empty(), "triangle should deposit ink");

    for (x, y) in &inked {
        assert!(
            (5..=15).contains(x) && (5..=15).contains(y),
            "inked pixel ({x}, {y}) lies outside the triangle's bounding box"
        );
    }
    assert!(
        inked.len() < 121,
        "a filled triangle must ink fewer pixels than the 11x11 bounding box, got {}",
        inked.len()
    );
}

/// Two glyphs with different content must rasterize differently.
///
/// This is the direct regression guard against a shared placeholder bitmap:
/// whatever the function returns has to depend on the charproc stream.
#[test]
fn distinct_glyphs_produce_distinct_bitmaps() {
    let font = font_with_glyphs(&[("small", 1), ("large", 2)]);
    let small = rasterize(&font, "small", b"2 2 4 4 re f").expect("small rect rasterizes");
    let large = rasterize(&font, "large", b"2 2 14 14 re f").expect("large rect rasterizes");

    assert_ne!(
        small, large,
        "glyphs with different charproc content must not share a bitmap"
    );
    assert_inked_exactly(&small, 2, 2, 6, 6);
    assert_inked_exactly(&large, 2, 2, 16, 16);
}

/// The same glyph rasterized twice must be byte-identical.
#[test]
fn rasterization_is_deterministic() {
    let font = font_with_glyphs(&[("rect", 1)]);
    let first = rasterize(&font, "rect", b"5 5 10 10 re f").expect("first rasterization");
    let second = rasterize(&font, "rect", b"5 5 10 10 re f").expect("second rasterization");

    assert_eq!(first, second, "repeated rasterization must be stable");
}

/// A stroked rectangle inks only its outline, not its interior.
#[test]
fn stroked_rectangle_inks_outline_only() {
    let font = font_with_glyphs(&[("outline", 1)]);
    let bitmap = rasterize(&font, "outline", b"5 5 10 10 re S").expect("stroke should rasterize");

    let inked = black_pixels(&bitmap);
    assert!(!inked.is_empty(), "stroke should deposit ink");

    // The interior pixel must stay white; the boundary must be inked.
    let interior = (10 * DIM + 10) as usize;
    assert_eq!(bitmap[interior], 255, "centre of a stroked rectangle should be white");

    let xs: Vec<i32> = inked.iter().map(|(x, _)| *x).collect();
    let ys: Vec<i32> = inked.iter().map(|(_, y)| *y).collect();
    assert_eq!(*xs.iter().min().unwrap(), 5, "outline x-min");
    assert_eq!(*xs.iter().max().unwrap(), 15, "outline x-max");
    assert_eq!(*ys.iter().min().unwrap(), 5, "outline y-min");
    assert_eq!(*ys.iter().max().unwrap(), 15, "outline y-max");
}

/// Absent glyphs and absent resolvers must yield None, not a fallback bitmap.
#[test]
fn failure_paths_return_none() {
    let font = font_with_glyphs(&[("rect", 1)]);
    let resolver = |_reference: ObjRef| -> Option<Vec<u8>> { Some(b"5 5 10 10 re f".to_vec()) };

    assert!(
        rasterize_type3_glyph(&font, "no-such-glyph", None, Some(&resolver)).is_none(),
        "an unregistered glyph must return None"
    );

    assert!(
        rasterize_type3_glyph::<dyn Fn(ObjRef) -> Option<Vec<u8>>>(
            &font,
            "rect",
            None,
            None,
        )
        .is_none(),
        "a glyph with no stream resolver must return None rather than guess"
    );
}

// ===========================================================================
// DocumentContext resolution path (bf-5v0bgu)
// ===========================================================================
//
// The tests above feed the rasterizer pre-decoded bytes through a callback.
// `rasterize_type3_glyph` also accepts a DocumentContext (XrefResolver +
// PdfSource) and dereferences the char_proc ObjRef itself when no callback is
// supplied. These tests build real indirect objects and pin that path to the
// same geometry, so the placeholder era cannot return through either route.

/// Build a DocumentContext over `N 0 obj … endobj` bodies with a real
/// XrefResolver.
///
/// Offsets are recorded while the bytes are concatenated rather than recovered
/// from a finished document (the approach type3_charproc_structure.rs takes),
/// keeping the fixture independent of xref repair paths. The source and
/// resolver are leaked so the context is `'static`; each test builds its own
/// copy.
fn document_context_over(objects: &[(u32, &str)]) -> DocumentContext<'static> {
    let mut bytes = Vec::new();
    let mut resolver = XrefResolver::new();

    for (obj_nr, body) in objects {
        let offset = bytes.len() as u64;
        bytes.extend_from_slice(body.as_bytes());
        resolver.add_entry(*obj_nr, XrefEntry::InUse { offset, gen_nr: 0 });
    }

    DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: Some(Box::leak(Box::new(MemorySource::new(bytes)))),
    }
}

/// A charproc resolved through the DocumentContext must rasterize to the same
/// ink as the callback path.
#[test]
fn document_context_path_rasterizes_real_charproc() {
    // Object 7 is what a real Type3 charproc looks like: a bare stream whose
    // dictionary carries no keys — no /Type, /Subtype, /Width or /Height.
    let ctx = document_context_over(&[(
        7,
        "7 0 obj\n<< >>\nstream\n5 5 10 10 re f\nendstream\nendobj\n",
    )]);

    let font = font_with_glyphs(&[("rect", 7)]);
    let bitmap = rasterize_type3_glyph(&font, "rect", Some(&ctx), None::<&StreamResolverFn>)
        .expect("the DocumentContext alone must resolve and rasterize the charproc");

    assert_eq!(bitmap.len(), (DIM * DIM) as usize, "bitmap is 22x22 bytes");
    assert_inked_exactly(&bitmap, 5, 5, 15, 15);
}

/// A charproc that resolves to a non-stream cannot be executed: None, not a
/// fallback bitmap.
#[test]
fn document_context_path_rejects_non_stream_target() {
    let ctx = document_context_over(&[(8, "8 0 obj\n42\nendobj\n")]);

    let font = font_with_glyphs(&[("bad", 8)]);
    assert!(
        rasterize_type3_glyph(&font, "bad", Some(&ctx), None::<&StreamResolverFn>).is_none(),
        "an integer char_proc target must return None"
    );
}

/// A char_proc ref with no xref entry is unresolvable: None.
#[test]
fn document_context_path_returns_none_for_unresolvable_ref() {
    let ctx = document_context_over(&[]);

    let font = font_with_glyphs(&[("missing", 99)]);
    assert!(
        rasterize_type3_glyph(&font, "missing", Some(&ctx), None::<&StreamResolverFn>).is_none(),
        "a char_proc ref with no xref entry must return None"
    );
}

/// When both resolution routes are supplied the callback wins — its result is
/// final, so callers that thread state through it are not silently re-routed
/// through the DocumentContext.
#[test]
fn callback_takes_precedence_over_document_context() {
    let ctx = document_context_over(&[(
        9,
        "9 0 obj\n<< >>\nstream\n5 5 10 10 re f\nendstream\nendobj\n",
    )]);

    let font = font_with_glyphs(&[("rect", 9)]);

    // A failing callback suppresses the (resolvable) DocumentContext target.
    let failing = |_reference: ObjRef| -> Option<Vec<u8>> { None };
    assert!(
        rasterize_type3_glyph(&font, "rect", Some(&ctx), Some(&failing)).is_none(),
        "a supplied callback is authoritative even when it resolves nothing"
    );

    // A succeeding callback's stream is the one rasterized.
    let overriding = move |_reference: ObjRef| -> Option<Vec<u8>> { Some(b"0 0 4 4 re f".to_vec()) };
    let bitmap = rasterize_type3_glyph(&font, "rect", Some(&ctx), Some(&overriding))
        .expect("callback bytes must rasterize");
    assert_inked_exactly(&bitmap, 0, 0, 4, 4);
}
