//! Fixture-backed Type3 glyph rasterization coverage (bead pdftract-20bf840a).
//!
//! The earlier shape tests (`type3_glyph_shape.rs`, `test_type3_integration.rs`)
//! rasterize charproc bytes held in memory: the stream never has to survive a
//! real parse. This suite closes that gap with a checked-in fixture PDF
//! (`tests/fixtures/fonts/type3-glyph-shapes.pdf`) whose Type3 font carries
//! non-placeholder CharProc shapes. The tests resolve the CharProc streams out
//! of the real file — `FileSource` + the fixture's own xref table +
//! `XrefResolver` — the same resolver/source pair the rasterizer dereferences
//! CharProc ObjRefs against when no callback is supplied (the shape-db
//! resolver path).
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
//! - crates/pdftract-core/src/font/type3.rs - Type3Font builder (type3_font_full)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use pdftract_core::font::type3::Type3Font;
use pdftract_core::font::type3_rasterizer::{
    rasterize_type3_glyph, DocumentContext, StreamResolverFn,
};
use pdftract_core::parser::object::types::{ObjRef, PdfDict, PdfObject};
use pdftract_core::parser::stream::PdfSource as _;
use pdftract_core::parser::xref::{parse_traditional_xref, XrefResolver};
use pdftract_core::FileSource;

/// Crate-relative location of the checked-in fixture.
const FIXTURE: &str = "tests/fixtures/fonts/type3-glyph-shapes.pdf";
/// Object number of the Type3 font dictionary inside the fixture.
const FONT_OBJ: u32 = 5;
/// Object numbers of the three /CharProcs targets (fixture layout, see the
/// fixture README): Box stream, Tri stream, and the malformed non-stream.
const BOX_OBJ: u32 = 7;
const TRI_OBJ: u32 = 8;
const BROKEN_OBJ: u32 = 9;
/// Bitmap edge for a [0 0 20 20] FontBBox: 20 + 2 * 1 padding.
const DIM: usize = 22;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

/// Open the fixture and build the resolver from its own xref table.
///
/// The `startxref` offset is located by scanning the file tail (the same
/// recipe the lib's own xref tests use), then `parse_traditional_xref` reads
/// the table — the conventional primary load path — so the offsets the
/// resolver hands to the source are the fixture's real byte offsets, not
/// hand-built entries. The fixture's table is correct, which this suite
/// verifies implicitly: every object below resolves through it.
fn open_fixture() -> (FileSource, XrefResolver) {
    let source = FileSource::open(fixture_path()).expect("fixture file must open");

    // Tail-scan for the startxref keyword and parse the offset that follows.
    let len = source.len().expect("fixture length readable") as usize;
    let tail_start = len.saturating_sub(1024);
    let tail = source
        .read_at(tail_start as u64, 1024)
        .expect("fixture tail readable");
    let kw = tail
        .windows(b"startxref".len())
        .rposition(|w| w == b"startxref")
        .expect("startxref keyword must be in the fixture's last 1024 bytes");
    let offset_digits: String = tail[kw + b"startxref".len()..]
        .iter()
        .skip_while(|b| b.is_ascii_whitespace())
        .take_while(|b| b.is_ascii_digit())
        .map(|b| *b as char)
        .collect();
    let startxref: u64 = offset_digits
        .parse()
        .expect("startxref offset must be a number");

    let section = parse_traditional_xref(&source, startxref);
    let resolver = XrefResolver::from_section(section);
    (source, resolver)
}

/// Resolve an object through the document context, requiring success.
fn resolve(resolver: &XrefResolver, source: &FileSource, obj_nr: u32) -> PdfObject {
    resolver
        .resolve_with_source(ObjRef::new(obj_nr, 0), source)
        .unwrap_or_else(|e| panic!("fixture object {obj_nr} must resolve: {e:?}"))
}

/// Read a numeric array (FontMatrix / FontBBox) out of a parsed dict.
fn numeric_array(dict: &PdfDict, key: &str) -> Vec<f64> {
    match dict.get(key) {
        Some(PdfObject::Array(arr)) => arr
            .iter()
            .map(|e| {
                e.as_real()
                    .or(e.as_int().map(|i| i as f64))
                    .expect("fixture array elements must be numeric")
            })
            .collect(),
        other => panic!("fixture /{key} must be a direct array, got {other:?}"),
    }
}

/// Build the font under test from the fixture's own font dict.
///
/// The CharProcs ObjRefs are read out of the dict the document context
/// resolved, so the glyph stream references exercised below are the fixture's
/// real indirect references. The remaining font fields come from the builder
/// defaults — the same construction the sibling shape tests use
/// (`type3_glyph_shape.rs`): identity FontMatrix and a [0 0 20 20] FontBBox,
/// which is exactly what the fixture declares.
///
/// `Type3Font::load` is deliberately not used: the lexer stores dict keys
/// without the leading slash while `Type3Font::load` looks keys up with it,
/// so against any parser-produced dict it builds a zero-glyph font
/// (pre-existing key-convention defect, unrelated to the resolver→rasterizer
/// pipeline this suite covers; recorded on the bead).
fn fixture_font(resolver: &XrefResolver, source: &FileSource) -> Type3Font {
    let font_dict = match resolve(resolver, source, FONT_OBJ) {
        PdfObject::Dict(d) => d,
        other => panic!("fixture font object must be a dict, got {other:?}"),
    };

    // The lexer strips the leading '/' from names, so keys are slash-less.
    let char_procs_dict = match font_dict.get("CharProcs") {
        Some(PdfObject::Dict(d)) => d,
        other => panic!("fixture /CharProcs must be a direct dict, got {other:?}"),
    };

    let mut char_procs: HashMap<Arc<str>, ObjRef> = HashMap::new();
    for (name, value) in char_procs_dict.iter() {
        match value {
            PdfObject::Ref(r) => {
                char_procs.insert(Arc::clone(name), *r);
            }
            other => panic!("glyph {name} must be an indirect ref, got {other:?}"),
        }
    }

    Type3Font::type3_font_full()
        .with_char_procs(char_procs)
        .with_font_bbox([0.0, 0.0, 20.0, 20.0])
        .build()
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

/// The parsed fixture must be exactly the document the assertions below are
/// written against: the font dict declares an identity FontMatrix and a
/// [0 0 20 20] FontBBox (so glyph coords map 1:1 onto a 22x22 bitmap), and
/// /CharProcs carries the two drawable glyphs plus the malformed target at
/// the documented object numbers.
#[test]
fn fixture_font_dict_parses_with_identity_geometry() {
    let (source, resolver) = open_fixture();

    let font_dict = match resolve(&resolver, &source, FONT_OBJ) {
        PdfObject::Dict(d) => d,
        other => panic!("fixture font object must be a dict, got {other:?}"),
    };

    assert_eq!(
        numeric_array(&font_dict, "FontMatrix"),
        vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        "fixture FontMatrix must be identity so glyph coords map 1:1 to pixels"
    );
    assert_eq!(
        numeric_array(&font_dict, "FontBBox"),
        vec![0.0, 0.0, 20.0, 20.0],
        "fixture FontBBox must be [0 0 20 20] so the bitmap is 22x22"
    );

    let font = fixture_font(&resolver, &source);
    assert_eq!(
        font.char_proc("Box"),
        Some(ObjRef::new(BOX_OBJ, 0)),
        "Box must reference the Box charproc stream object"
    );
    assert_eq!(
        font.char_proc("Tri"),
        Some(ObjRef::new(TRI_OBJ, 0)),
        "Tri must reference the Tri charproc stream object"
    );
    assert_eq!(
        font.char_proc("Broken"),
        Some(ObjRef::new(BROKEN_OBJ, 0)),
        "Broken must reference the malformed non-stream object"
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
    let font = fixture_font(&resolver, &source);
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

/// The "Tri" glyph (`0 0 m 20 0 l 0 20 l h f`, a right triangle with its
/// legs on the axes) produces a different bitmap from "Box" through the same
/// document context, with the exact triangle geometry: full base along row 0,
/// vertical leg on x=0, and the hypotenuse stepping down to the apex pixel.
/// Distinct shapes rasterizing distinctly is what makes Type3 font
/// fingerprints separable.
#[test]
fn tri_glyph_differs_from_box_with_triangle_geometry() {
    let (source, resolver) = open_fixture();
    let font = fixture_font(&resolver, &source);
    let ctx = document_context(&resolver, &source);

    let tri = rasterize_type3_glyph(&font, "Tri", Some(&ctx), None::<&StreamResolverFn>)
        .expect("Tri must rasterize through the document context");
    let box_bitmap = rasterize_type3_glyph(&font, "Box", Some(&ctx), None::<&StreamResolverFn>)
        .expect("Box must rasterize through the document context");

    assert_eq!(tri.len(), DIM * DIM);
    assert_ne!(
        tri, box_bitmap,
        "two different CharProcs must rasterize differently"
    );

    // Exact inked set: each row y in 0..=19 fills x in 0..=(19 - y), then the
    // apex pixel (0, 20); the rest — including the padding ring and the
    // base's right endpoint (20, 0), which fill_polygon's per-row stepping
    // leaves unpainted — stays white.
    for y in 0..DIM {
        for x in 0..DIM {
            let inked = (y <= 19 && x <= 19 - y) || (x == 0 && y == 20);
            let expected = if inked { 0 } else { 255 };
            assert_eq!(
                pixel(&tri, x, y),
                expected,
                "Tri pixel ({x}, {y}) must be {}",
                if inked { "inked" } else { "white" }
            );
        }
    }
}

/// A glyph name absent from /CharProcs returns None instead of panicking —
/// the missing-glyph failure path fingerprinting relies on to skip fonts
/// gracefully.
#[test]
fn missing_glyph_returns_none() {
    let (source, resolver) = open_fixture();
    let font = fixture_font(&resolver, &source);
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
    let font = fixture_font(&resolver, &source);
    let ctx = document_context(&resolver, &source);

    // The target really is there and really is not a stream.
    let broken = resolve(&resolver, &source, BROKEN_OBJ);
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
