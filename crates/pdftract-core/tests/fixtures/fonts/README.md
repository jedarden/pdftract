# type3-glyph-shapes.pdf

First-party fixture, generated for bead `pdftract-20bf840a` (no third-party
material; no provenance entry required — this is not under the repo-root
`tests/fixtures/` ledger scope). Regenerable byte-for-byte with the object
layout below; a 1 KiB file, so it is checked in directly.

A single-page PDF whose page draws two glyphs with a Type3 font. It exists so
`crates/pdftract-core/tests/type3_glyph_rasterization_fixture.rs` can resolve
and rasterize glyphs through a real document context (`FileSource` +
`forward_scan_xref` + `XrefResolver`) instead of in-memory byte blobs.

## Design constraints

- **Identity FontMatrix** `[1 0 0 1 0 0]` + **FontBBox** `[0 0 20 20]`:
  glyph-space coordinates land on bitmap pixel indices with no scaling or
  flipping, and the rasterized bitmap is exactly 22x22 (one pixel of padding
  per side, per `calculate_bitmap_dimensions`). This is the same exact-geometry
  convention as `crates/pdftract-core/tests/type3_glyph_shape.rs`.
- **/CharProcs is a direct dictionary** in the font dict: `Type3Font::load`
  rejects an indirect `/CharProcs` ref ("not supported, treating as
  zero-glyph font"). Individual glyph entries ARE indirect refs — that is the
  supported shape.
- The xref table carries real byte offsets, so both the forward-scan path used
  by the test and any conventional startxref-based reader resolve every object.

## Object layout

| Obj | Contents |
|-----|----------|
| 1 | Catalog → Pages |
| 2 | Pages, one kid |
| 3 | Page, MediaBox [0 0 72 72], /Font /T3 → 5, /Contents → 4 |
| 4 | Page content stream: `BT /T3 12 Tf 10 30 Td (AB) Tj ET` |
| 5 | Type3 font: /FontBBox [0 0 20 20], /FontMatrix [1 0 0 1 0 0], direct /CharProcs `{/Box 7 0 R /Tri 8 0 R /Broken 9 0 R}`, /Encoding → 6, /FirstChar 65 /LastChar 66, /Widths [20 20] |
| 6 | /Encoding /Differences [65 /Box 66 /Tri] |
| 7 | "Box" CharProc: `0 0 10 10 re f` → ink exactly (0,0)..=(10,10), endpoint-inclusive |
| 8 | "Tri" CharProc: `0 0 m 20 0 l 10 20 l h f` → full-width base, apex at top-center |
| 9 | "Broken" target: a plain dictionary, deliberately **not** a stream — the malformed-CharProc failure path (`rasterize_type3_glyph` must return None, not panic) |
