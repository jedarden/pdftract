//! Fixture-backed Type3 glyph rasterization coverage (bead pdftract-20bf840a).
//!
//! The earlier shape tests (`type3_glyph_shape.rs`, `test_type3_integration.rs`)
//! rasterize charproc bytes held in memory: the stream never has to survive a
//! real parse. This suite closes that gap with a checked-in fixture PDF
//! (`tests/fixtures/fonts/type3-glyph-shapes.pdf`) whose Type3 font carries
//! non-placeholder CharProc shapes. Every test here goes through the real
//! document context — `FileSource` + `forward_scan_xref` + `XrefResolver` —
//! the same resolver/source pair the rasterizer dereferences CharProc ObjRefs
//! against when no callback is supplied (the shape-db resolver path).
//!
//! Geometry is exact for the same reason as `type3_glyph_shape.rs`: the
//! fixture font uses an identity FontMatrix with a [0 0 20 20] FontBBox, so
//! the bitmap is 22x22 (one pixel of padding per side per
//! `calculate_bitmap_dimensions`) and glyph-space coordinates land on bitmap
//! pixel indices with no scaling or flipping. That keeps the assertions
//! stable inputs for font fingerprinting, which compares these bitmaps.
//!
//! # References
//!
//! - crates/pdftract-core/tests/fixtures/fonts/type3-glyph-shapes.pdf - fixture
//! - crates/pdftract-core/src/font/type3_rasterizer.rs - rasterize_type3_glyph
//! - crates/pdftract-core/src/font/type3.rs - Type3Font::load (real parse path)

use std::path::PathBuf;

use pdftract_core::font::type3::Type3Font;
use pdftract_core::font::type3_rasterizer::{
    rasterize_type3_glyph, DocumentContext, StreamResolverFn,
};
use pdftract_core::parser::object::types::{ObjRef, PdfObject};
use pdftract_core::parser::xref::{forward_scan_xref, XrefResolver};
use pdftract_core::FileSource;

/// Crate-relative location of the checked-in fixture.
const FIXTURE: &str = "tests/fixtures/fonts/type3-glyph-shapes.pdf";
/// Object number of the Type3 font dictionary inside the fixture.
const FONT_OBJ: u32 = 5;
/// Bitmap edge for a [0 0 20 20] FontBBox: 20 + 2 * 1 padding.
const DIM: usize = 22;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

/// Open the fixture and build the resolver the way a real document load does.
///
/// The xref comes from `forward_scan_xref` over the actual file, so the
/// offsets the resolver hands to the source are the fixture's real byte
/// offsets — not hand-built entries.
fn open_fixture() -> (FileSource, XrefResolver) {
    let source = FileSource::open(fixture_path()).expect("fixture file must open");
    let section = forward_scan_xref(&source, false);
    let resolver = XrefResolver::from_section(section);
    (source, resolver)
}

/// Resolve the fixture's Type3 font dictionary through the document context
/// and parse it with the production `Type3Font::load` path.
fn load_fixture_font(resolver: &XrefResolver, source: &FileSource) -> Type3Font {
    let obj = resolver
        .resolve_with_source(ObjRef::new(FONT_OBJ, 0), source)
        .expect("fixture font object must resolve through the document context");
    match obj {
        PdfObject::Dict(dict) => Type3Font::load(&dict),
        other => panic!("fixture font object must be a dict, got {other:?}"),
    }
}

fn document_context<'a>(resolver: &'a XrefResolver, source: &'a FileSource) -> DocumentContext<'a> {
    DocumentContext {
        resolver: Some(resolver),
        source: Some(source),
    }
}

/// Read one bitmap pixel; ink is 0 (black), paper is 255 (white).
fn pixel(bitmap: &[u8], x: usize, y: usize) -> u8 {
    bitmap[y * DIM + x]
}

/// The fixture font must parse to exactly the geometry the assertions below
/// are written against: identity FontMatrix, [0 0 20 20] FontBBox, and all
/// three /CharProcs entries (two drawable glyphs plus the malformed target).
#[test]
fn fixture_font_parses_with_identity_geometry() {
    let (source, resolver) = open_fixture();
    let font = load_fixture_font(&resolver, &source);

    assert!(
        font.font_matrix.is_identity(),
        "fixture FontMatrix must be identity so glyph coords map 1:1 to pixels"
    );
    assert_eq!(
        font.font_bbox,
        [0.0, 0.0, 20.0, 20.0],
        "fixture FontBBox must be [0 0 20 20] so the bitmap is 22x22"
    );
    assert!(font.has_glyph("Box"), "Box glyph must be in /CharProcs");
    assert!(font.has_glyph("Tri"), "Tri glyph must be in /CharProcs");
    assert!(
        font.has_glyph("Broken"),
        "Broken glyph (malformed target) must be in /CharProcs"
    );
    assert_eq!(font.glyph_count(), 3);
}

/// The "Box" glyph (`0 0 10 10 re f`) rasterizes through the document context
/// to ink over exactly (0,0)..=(10,10) — the scanline fill is
/// endpoint-inclusive — with the rest of the bitmap, including the padding
/// ring, untouched. This is the checked-in non-placeholder shape: a hardcoded
/// placeholder bitmap cannot land ink in exactly this region and nowhere else.
#[test]
fn box_glyph_rasterizes_exact_geometry_through_document_context() {
    let (source, resolver) = open_fixture();
    let font = load_fixture_font(&resolver, &source);
    let ctx = document_context(&resolver, &source);

    let bitmap = rasterize_type3_glyph(&font, "Box", Some(&ctx), None::<&StreamResolverFn>)
        .expect("Box must rasterize through the document context");

    assert_eq!(bitmap.len(), DIM * DIM, "bitmap is 22x22 bytes");

    for y in 0..DIM {
        for x in 0..DIM {
            let expected = if x <= 10 && y <= 10 { 0 } else { 255 };
            assert_eq!(
                pixel(&bitmap, x, y),
                expected,
                "Box pixel ({x}, {y}) must be {}",
                if expected == 0 { "inked" } else { "white" }
            );
        }
    }
}

/// The "Tri" glyph (`0 0 m 20 0 l 10 20 l h f`) produces a different bitmap
/// from "Box" through the same document context, with geometry matching the
/// drawn triangle: full-width base at the bottom, apex at the top-center, and
/// white outside the hypotenuses. Distinct shapes rasterizing distinctly is
/// what makes Type3 font fingerprints separable.
#[test]
fn tri_glyph_differs_from_box_with_triangle_geometry() {
    let (source, resolver) = open_fixture();
    let font = load_fixture_font(&resolver, &source);
    let ctx = document_context(&resolver, &source);

    let tri = rasterize_type3_glyph(&font, "Tri", Some(&ctx), None::<&StreamResolverFn>)
        .expect("Tri must rasterize through the document context");
    let boxt = rasterize_type3_glyph(&font, "Box", Some(&ctx), None::<&StreamResolverFn>)
        .expect("Box must rasterize through the document context");

    assert_eq!(tri.len(), DIM * DIM);
    assert_ne!(
        tri, boxt,
        "two different CharProcs must rasterize differently"
    );

    // Base of the triangle spans the full width along the bottom row.
    assert_eq!(pixel(&tri, 0, 0), 0, "base-left endpoint must be inked");
    assert_eq!(pixel(&tri, 10, 0), 0, "base midpoint must be inked");
    assert_eq!(pixel(&tri, 20, 0), 0, "base-right endpoint must be inked");
    // Apex at the top-center.
    assert_eq!(pixel(&tri, 10, 20), 0, "apex must be inked");
    // Outside the hypotenuses: top corners and upper side margins stay white.
    assert_eq!(pixel(&tri, 0, 20), 255, "top-left corner must stay white");
    assert_eq!(pixel(&tri, 20, 20), 255, "top-right corner must stay white");
    assert_eq!(
        pixel(&tri, 2, 19),
        255,
        "left of the left edge must stay white"
    );
    assert_eq!(
        pixel(&tri, 19, 19),
        255,
        "right of the hypotenuse must stay white"
    );
    assert_eq!(pixel(&tri, 21, 21), 255, "padding ring must stay white");
    // Interior of the triangle is inked.
    assert_eq!(pixel(&tri, 10, 10), 0, "triangle interior must be inked");
}

/// A glyph name absent from /CharProcs returns None instead of panicking —
/// the missing-glyph failure path fingerprinting relies on to skip fonts
/// gracefully.
#[test]
fn missing_glyph_returns_none() {
    let (source, resolver) = open_fixture();
    let font = load_fixture_font(&resolver, &source);
    let ctx = document_context(&resolver, &source);

    let outcome =
        rasterize_type3_glyph(&font, "NoSuchGlyph", Some(&ctx), None::<&StreamResolverFn>);
    assert!(outcome.is_none(), "unknown glyph name must yield None");

    // A whole class of absent names behaves the same way.
    assert!(rasterize_type3_glyph(&font, "", Some(&ctx), None::<&StreamResolverFn>).is_none());
    assert!(
        rasterize_type3_glyph(&font, ".notdef", Some(&ctx), None::<&StreamResolverFn>).is_none()
    );
}

/// The fixture's "Broken" /CharProcs entry points at object 9, which resolves
/// successfully through the document context but is a plain dictionary, not a
/// stream. Rasterization must fail soft (None, no panic) — the
/// malformed-CharProc failure path — and the failure must not poison the
/// resolver: a valid glyph still rasterizes afterwards.
#[test]
fn malformed_charproc_target_returns_none_without_poisoning_context() {
    let (source, resolver) = open_fixture();
    let font = load_fixture_font(&resolver, &source);
    let ctx = document_context(&resolver, &source);

    // The target really is there and really is not a stream.
    let broken = resolver
        .resolve_with_source(ObjRef::new(9, 0), &source)
        .expect("the Broken target object must resolve");
    assert!(
        !matches!(broken, PdfObject::Stream(_)),
        "fixture object 9 must not be a stream for this to be the malformed path"
    );

    let outcome = rasterize_type3_glyph(&font, "Broken", Some(&ctx), None::<&StreamResolverFn>);
    assert!(
        outcome.is_none(),
        "a CharProc ref that resolves to a non-stream must yield None, not panic"
    );

    // The document context still serves valid glyphs after the failure.
    let bitmap = rasterize_type3_glyph(&font, "Box", Some(&ctx), None::<&StreamResolverFn>)
        .expect("valid glyph must still rasterize after a malformed-CharProc failure");
    assert_eq!(bitmap.len(), DIM * DIM);
    assert_eq!(pixel(&bitmap, 5, 5), 0, "Box interior must still be inked");
}
