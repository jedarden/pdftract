# pdftract-3923eb56 — Type3 CharProc stream resolution (DocumentContext path)

## What this bead asked for

Resolve a Type3 font character-procedure ObjRef through the `DocumentContext`
introduced by the plumbing bead (pdftract-3ce2cd32 / parent bf-5v0bgu) and
decode its content stream for rasterization, handling missing, indirect,
non-stream, and malformed objects safely.

## State found at HEAD

The implementation already landed in commit `567e3072`
(`fix(bf-5v0bgu): activate DocumentContext resolution in Type3 glyph
rasterizer`): `rasterize_type3_glyph` dereferences the char_proc ObjRef
through the context's `XrefResolver` + `PdfSource` and decodes with
`decode_stream` whenever no `resolve_stream` callback is supplied
(`resolve_char_proc_via_context`, type3_rasterizer.rs). The production
consumer (`resolve_type3_level4` in font/resolver.rs) threads both a callback
(authoritative, carries the document-wide decompression counter) and the
context.

What was **missing** was any test coverage of that context path: every
existing `rasterize_type3_glyph` test either supplied a callback or passed a
`DocumentContext { resolver: None, source: None }`, so the context-resolution
half of the contract was never exercised against a real resolver and source.

## What this dispatch added

Test-only changes (implementation was verified complete at HEAD):

- `crates/pdftract-core/src/font/type3_rasterizer.rs` (mod tests) — six
  helper-level tests for the private `resolve_char_proc_via_context`:
  - valid unfiltered stream → byte-exact payload (`Ok` bytes == content)
  - FlateDecode stream → decompressed plaintext (proves real inflation,
    built byte-wise because the fixture helper is `from_utf8_lossy`-based)
  - missing xref entry → `None`
  - non-stream targets (dict, integer, ref-to-ref chain) → `None`
  - unparseable bytes at the target offset → `None`
  - context missing resolver or source → `None`
- `crates/pdftract-core/src/font/type3_rasterizer_test.rs` — six public-API
  tests calling `rasterize_type3_glyph` with `None::<&StreamResolverFn>` and a
  real `DocumentContext` built by `create_valid_dereference_context`:
  - valid stream rasterizes a real glyph: rect (8,8)–(20,20) (geometry unique
    to this test), exact 13×13 inked-pixel count, white padding — assertions
    no hardcoded/synthetic bitmap could satisfy
  - FlateDecode stream → 21×21 inked rect (compressed bytes contain no valid
    operators, so ink proves `decode_stream` ran)
  - missing ref / non-stream target / malformed object → `None`
  - no context and no callback → `None`

The ref-to-ref case documents the single-hop convention deliberately: a
charproc target that resolves to another `PdfObject::Ref` degrades to `None`,
mirroring the production `resolve_stream` callback in font/resolver.rs.

## Verification (clean `git archive HEAD` extraction + these files, /var/tmp)

- `cargo test -p pdftract-core --lib resolve_char_proc_via_context` →
  6 passed, 0 failed
- `cargo test -p pdftract-core --lib via_document_context` →
  5 passed, 0 failed
- `cargo test -p pdftract-core --lib test_rasterize_without_context_or_callback_returns_none` →
  1 passed, 0 failed
- `cargo build --all-targets -p pdftract-core` → exit 0
- Regression sweep `cargo test -p pdftract-core --lib type3_rasterizer`:
  with my tests 216 passed / 47 failed; restoring the two pristine HEAD files
  in the same extraction gives 204 passed / **the same 47 failed** — all 47
  (fill/edge/slope/AET/`detect_char_proc_type` families, e.g.
  `render::scanline::tests::test_fill_polygon_rectangle` at scanline.rs:826)
  are pre-existing at HEAD and unaffected by this change. The wider tree also
  carries ~300 pre-existing test failures (plan TH notes), which is why the
  fence cites precise filters rather than a blanket `cargo test`.

## PASS/WARN

- PASS: valid CharProc ObjRef yields decoded content-stream bytes (byte-exact,
  uncompressed and FlateDecode).
- PASS: missing/invalid/non-stream/malformed references return `None`
  (established non-panicking failure result) via both helper and public API.
- PASS: resolution honors the document resolver/context — tests dereference
  real objects through `XrefResolver` + `MemorySource`; pixel-exact assertions
  on unique geometry rule out hardcoded bitmaps/synthetic streams.
- PASS: focused tests cover ≥1 valid stream and ≥1 invalid/missing reference
  (6 + 6 tests).
- WARN (pre-existing, out of scope): 47 type3_rasterizer-module test failures
  and ~300 tree-wide failures exist at HEAD independent of this change.
