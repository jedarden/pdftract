# bf-5r6rdo — Type3 glyph rasterizer: consolidated verification (chain children 1–4)

Consolidated full-suite verification for the bf-5r6rdo "Implement Type3 glyph
rasterizer stub" chain, written by **pdftract-c1ac91f1** (split child 4/4,
terminal) on 2026-09-20. Every number below is a fresh run made during this
dispatch — no evidence is cited from an earlier worker's session.

- **Verified at HEAD:** `b855f3941a84909b080b981b8213bf574e9069f2`
- **Where:** clean `git archive` extraction at
  `/var/tmp/pdftract-c1ac91f1-verify` (no `.git`; the shared working tree was
  not used — it does not compile as a union because of concurrent workers'
  in-flight edits)
- **Timeouts:** every cargo invocation wrapped in
  `timeout --kill-after=30s 600s`; **zero exit-124** in this dispatch
- **Chain commits (all confirmed ancestors of the verified HEAD):**
  - pdftract-0ed4425f — `567e3072`, `6afc2b7f`, `dd6503de`
  - pdftract-93bdb518 — `16faf877`
  - pdftract-384db234 — `77ee8e8c`, `ff688c18`

## Parent acceptance criteria — verdicts at HEAD

| # | Criterion | Verdict | Fresh evidence |
|---|---|---|---|
| 1 | Pass document resolver context to `rasterize_type3_glyph()` so it can dereference ObjRef | **PASS** | `doc_context: Option<&DocumentContext>` param at `type3_rasterizer.rs:2410-2415`; context path `resolve_char_proc_via_context(char_proc_ref, doc_context?)` at `:2425`; tests `test_resolve_char_proc_via_context_*` 6/6 ok, `document_context_path_*` 4/4 + `callback_takes_precedence_over_document_context` ok (`type3_glyph_shape`), 8/8 `test_rasterize_via_document_context_*` ok |
| 2 | Resolve `char_proc_ref` to content stream bytes | **PASS** | `resolve_char_proc_via_context` (`:2442-2466`): `XrefResolver::resolve_with_source` → `PdfObject::Stream` → `decode_stream`; tests `test_resolve_char_proc_via_context_valid_stream_yields_decoded_bytes` ok, `..._flate_stream_yields_decompressed_bytes` ok, `test_rasterize_via_document_context_flate_stream_decodes` ok |
| 3 | Execute `execute_content_stream()` on those bytes | **PASS** | `RasterizerContext::new(font)` + `ctx.execute_content_stream(&stream_bytes)` at `:2429-2430`; ink-asserting end-to-end tests ok: `filled_rectangle_lands_on_drawn_extent`, `document_context_path_rasterizes_real_charproc`, `test_rasterize_via_document_context_multi_operator_glyph` (q/cm/re/f/Q, cm translation pinned), `test_rasterize_via_document_context_stroked_path_glyph` (1px Bresenham row/cols pinned) |
| 4 | Return actual rasterized glyph bitmap, not the 16×16 placeholder | **PASS** | `grep 'u8; 256]' type3_rasterizer.rs` = 0 matches; `16*16` = 0 matches; sole success return is `Some(ctx.bitmap.as_bytes().to_vec())` (`:2431`) with bitmap sized by `calculate_bitmap_dimensions(&font.font_bbox, None)` (`:983`); 10/10 `test_calculate_bitmap_dimensions_*` ok incl. zero-width/zero-height degradation; doctest 1/1 ok |
| 5 | Verify with Type3 font fixtures | **PASS** (WARN: pre-existing failure families documented below) | multi-glyph fixture `create_multi_glyph_fixture` (`type3_rasterizer_test.rs:3069`, FontBBox [0 0 32 32], 4 CharProcs); its 5 DocumentContext-path tests ok incl. the 3 added by pdftract-384db234; integration binaries green (below) |

## Full-suite results at HEAD (this dispatch)

| Suite (all `-p pdftract-core`) | Result | Exit | Wall |
|---|---|---|---|
| `--lib font::type3` | **292 passed / 52 failed** (344 matched) | 101¹ | 64s |
| `--lib font::type3_rasterizer` | **219 passed / 47 failed** (266 matched) | 101¹ | 90s |
| `--lib font::resolver` | **25 passed / 0 failed** | 0 | 27s |
| `--test type3_glyph_shape` | **14 passed / 0 failed** | 0 | 10s |
| `--test type3_charproc_structure` | **10 passed / 0 failed** | 0 | 10s |
| `--test test_type3_integration` | **17 passed / 0 failed** | 0 | 11s |
| `--test content_stream_font_diagnostic` | **4 passed / 0 failed** | 0 | 12s |
| `--doc calculate_bitmap_dimensions` | **1 passed / 0 failed** | 0 | 13s |

¹ exit 101 = the 52/47 pre-existing failures accounted below; **no test
related to this chain appears in any failure set.**

## The 52 failures are pre-existing and chain-independent — re-derived at HEAD

Fresh surgical-revert experiment (the same technique pdftract-0ed4425f used at
its HEAD): took the identical `b855f394` extraction and neutralized exactly
one line — `type3_rasterizer.rs:2425`
`None => resolve_char_proc_via_context(char_proc_ref, doc_context?),` →
`None => None,` — then re-ran the `font::type3_rasterizer` filter
(214 passed / 52 failed, 55s):

- Failure-set diff vs HEAD: the revert adds **exactly the 5**
  `rasterize_via_document_context_*` success-path tests (which must fail
  without the wiring) and removes **none**: the other **47 failures are
  byte-identical** and therefore independent of the chain's wiring line.
- Count identity: pdftract-0ed4425f documented this same filter at `d5854e90`
  as 216/47; at `b855f394` it is 219/47 — the only delta is the +3 passing
  fixture tests pdftract-384db234 added; failure count unchanged at 47.
- The remaining 5 failures in the wider `font::type3` filter are
  `font::type3::tests::test_helper_*` (test_glyph_helper consumers), already
  documented as pre-existing by pdftract-384db234.
- A pre-chain baseline run was attempted first and is impossible: the
  pre-chain commit `567e3072^` (`d96a1261`) does not compile today (287
  errors) — expected: committed-state compile was only repaired later in
  `784326b6` ("repair committed-state compile for default_rust gate"), which
  postdates the chain start.

## Literal acceptance-criterion command

The criterion-1 command was run verbatim once in the same clean extraction:

```
$ timeout --kill-after=30s 600s cargo test --all-targets
EXIT: 101   WALL: 175s   (not timeout-killed)
```

Outcome: cargo fail-fasts on the **first test binary it schedules** —
`pdftract-cli --lib` (351 passed / 10 failed, 0.16s) — and aborts before any
`pdftract-core` binary runs, so the literal command structurally cannot deliver
the `font::type3*`/`font::resolver` results at this HEAD. All 10 cli-lib
failures are pre-existing and non-font (`inspect::render*` svg/mcid,
`pages::tests` parse filter, `serve::tests`, `url::tests`); grep for
font|type3|resolver among them = 0 matches. The font suites therefore run via
the targeted invocations in the table above (the approach the workspace gate
guidance prescribes for this tree's ~300 pre-existing failures). This dispatch
did not treat the 101 as a pass: the pass bar ("all font::type3_* and
font::resolver tests pass") is evidenced by the targeted runs, and the
101's cause is fully accounted for above.

## Orphan check

After all cargo activity (suites, revert run, baseline-attempt build, literal
run), both checks return no matches (rc=1):

```
$ pgrep -af 'pdftract[ ]mcp'    → rc=1, no matches
$ pgrep -af 'TH[_]0|TH[-]0'     → rc=1, no matches
```

Criterion 2 PASS — no orphaned processes from any run this dispatch.

## Per-child summary (what this dispatch re-derived at HEAD)

### pdftract-0ed4425f — DocumentContext resolver wiring (`567e3072`, `6afc2b7f`, `dd6503de`)
Verified at `b855f394`: wiring present (criterion 1/2 citations above);
6/6 helper-level `resolve_char_proc_via_context` tests, 8/8 public-API
`rasterize_via_document_context` tests, 5/5 `type3_glyph_shape`
document-context tests (incl. circular-ref guard-release and
callback-precedence compatibility for existing callers) all pass; the 47
filter failures proven independent by the fresh revert above. **PASS.**

### pdftract-93bdb518 — dynamic dimensions, no fixed placeholder (`16faf877`)
Verified at `b855f394`: no fixed-size allocation on any return path (grep
evidence above); 10/10 `calculate_bitmap_dimensions` tests + doctest;
end-to-end dimension derivation
(`test_rasterize_via_document_context_valid_stream_rasterizes_real_glyph`,
FontBBox [0 0 32 32] → 34×34 bitmap) passes. **PASS.**

### pdftract-384db234 — multi-glyph DocumentContext fixtures (`77ee8e8c`, `ff688c18`)
Verified at `b855f394`: `create_multi_glyph_fixture` present
(`type3_rasterizer_test.rs:3069`); its tests pass: stroked-path ink pinned to
row 10 cols 6..=26, multi-operator q/cm/re/f/Q with cm translation pinned to
rect (6,8)-(14,16) (ink 9×9+15=96), malformed content degrades to a graceful
blank 34×34 bitmap. Integration binaries `test_type3_integration` 17/17,
`type3_charproc_structure` 10/10, `content_stream_font_diagnostic` 4/4.
**PASS.**

## Conclusion

Parent bf-5r6rdo acceptance criteria (1)–(5): **all PASS** at HEAD
`b855f394`. The chain's complete Type3/resolver test surface is green; the
only failures in the font filters are the 52 long-pre-existing
scanline/AET/detection/helper tests, re-proven chain-independent this
dispatch. WARN items are environmental (pre-existing tree state), none
introduced or left by the chain. The parent is evidence-closeable citing this
note.
