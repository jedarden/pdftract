# pdftract-ff642b7b — Retain direct inline /Properties dictionaries in ResourceDict

Split child 2/4 of bf-1a61w9 (Resolve marked content property references).
Depends on child 1 (pdftract-966fddc5, commit `ebc7f922` — fixtures + tests), which is in HEAD.

## Defect

`merge_resources()` (crates/pdftract-core/src/parser/resources.rs, `/Properties` merge arm)
populated `ResourceDict.properties` only for entries whose value is an indirect reference
(`obj.as_ref().is_some()`). A `/Properties << /MC0 << /MCID 0 ... >> >>` direct inline dict
was silently dropped, so `lookup_properties()` returned `None` and BDC marked-content
metadata was lost even though no cross-reference resolution is needed at all.

## Fix (minimal churn at the two callers)

- **resources.rs** — widened `ResourceDict.properties` from
  `IndexMap<Arc<str>, ObjRef>` to `IndexMap<Arc<str>, PdfObject>`, mirroring the existing
  `color_spaces` namespace which already stores refs and inline objects side by side.
  The merge arm now stores values verbatim (`obj.clone()`); `lookup_properties()` returns
  `Option<&PdfObject>` with docs covering both shapes. `is_empty()`/`total_count()` are
  key-count based and unaffected.
- **marked_content_operators.rs** — both consumers (`extract_mcid_from_props` MCID path,
  `extract_ocg_ref_from_props` OCG path) now handle the stored value by shape:
  `PdfObject::Dict` → read `/MCID`//OCG` directly, **no resolver needed** (matches the
  production `parse_bdc` call site that passes `resolver=None`); `PdfObject::Ref` → the
  pre-existing resolve path verbatim; anything else → `StructInvalidBdcOperand` diagnostic
  ("is not a property dictionary"). Missing name → unchanged
  `UnknownMarkedContentProps` diagnostic.
- **Key-spelling duality** — file-parsed dict keys arrive WITHOUT the leading slash (the
  lexer consumes it) while hand-built dicts use the slashed spelling; property dicts reach
  the BDC path through both routes. Added `prop_dict_get()` which accepts both spellings
  for `/MCID` and `/OCG` lookups. This also fixes real content-stream inline BDC dicts
  (`parse_inline_dict_from_buffer` strips the slash), and is the convention child 1's
  fixture-integrity test explicitly requires `extract_mcid_from_dict` to match.

## Acceptance criteria

| # | Criterion | Result |
|---|---|---|
| 1 | `lookup_properties()` (or successor) returns usable property data for both indirect and direct values | **PASS** — returns `Option<&PdfObject>`; `Ref` → resolver path, `Dict` → direct read, both unit- and fixture-tested |
| 2 | Child-1 direct-dict test un-ignored and passing | **PASS** — `#[ignore]` removed from `bdc_name_props_direct_inline_dict_recovers_mcid`; green |
| 3 | Existing resource-merge tests still pass, incl. the `/Properties` merge test (~line 457) and `is_empty`/`total_count` accounting | **PASS** — `parser::resources` 9/9 incl. `test_merge_all_namespaces` (`merged.properties.len() == 1` with a `Ref` value), `test_empty_resource_dict`, `test_resource_dict_not_empty` |
| 4 | fmt/clippy clean on touched crates; tests pass; no orphaned processes | **PASS** (see "Pre-existing failures" for the HEAD-broken baseline caveat) |

## Verification

Commands (timeout-wrapped per repo policy):

- `cargo check -p pdftract-core --all-targets` — clean (only pre-existing dead-code warnings in unrelated test helpers)
- `cargo test -p pdftract-core --lib parser::marked_content_operators` — **50 passed, 0 failed** (7 new: direct dict × 2, slashless keys × 2, key-spelling duality, non-dict-non-ref diagnostic, OCG direct dict)
- `cargo test -p pdftract-core --lib parser::resources` — **9 passed, 0 failed**
- `cargo test -p pdftract-core --test marked_content_properties_fixtures` — **3 passed, 0 failed, 0 ignored**:
  - `bdc_name_props_direct_inline_dict_recovers_mcid` — un-ignored, green (was ignored)
  - `bdc_name_props_indirect_reference_recovers_mcid` — green (**was already RED at HEAD** before this bead: `load_fixture` never warmed object 4 through the cache-only `XrefResolver::resolve`, so it resolved to Null → "resolved to non-dict object". Fixed here as part of the harness: `load_fixture` now warms indirect `/Properties` targets via `resolve_with_source`, mirroring the file's documented page-tree warming pattern; plus the dual-key lookup. This test is child 1's headline test and was one of the known pre-existing HEAD failures.)
  - `fixture_indirect_property_object_parses_to_mcid_dict` — green (unchanged)
- `rustfmt --edition 2021 --check` on the three touched files — clean (rustfmt on this box rewrites in place, so `--check` was used, scoped to my files only; a bare `cargo fmt` would have reformatted other workers' in-flight edits)
- `cargo clippy -p pdftract-core --all-targets` — zero diagnostics in the touched files; the one `unused mut` in `resources.rs` `test_merge_colorspace_inline_array` is pre-existing untouched code. Two `explicit_auto_deref` nits in blocks this diff rewrote were fixed.
- Orphan scan `pgrep -af 'pdftract mcp|TH_0|TH-0'` — empty (only the scan's own bash wrapper self-matches)

### Pre-existing failures (not caused by this diff)

Broader suite runs (post-fix) still show the known HEAD-broken baseline
(see memory: ~300 pre-existing failures at HEAD; `pdftract extract` 0/1377 fixtures as of
2026-09-14). Failure attribution for everything adjacent to this diff:

- `cargo test --lib parser::` → 834 passed, **18 failed**: all in `parser::xref`
  (forward-scan / xref-stream parsing), `parser::hint_stream`, `parser::object::cache`.
  None of those files references `ResourceDict`, `lookup_properties`, `parse_bdc`, or
  `merge_resources` (grep-verified) — the failing tests never execute this diff's code.
  `xref.rs` additionally carries another worker's in-flight working-tree edits.
- `content_stream` filter → 131 passed, **1 failed**:
  `font::type3_rasterizer::tests::test_execute_content_stream_with_invalid_tokens_does_not_crash`
  (bitmap assertion). Its stream contains no BDC token and `type3_rasterizer.rs` has zero
  references to the changed symbols; the file is clean vs HEAD. It appears FAILED in this
  session's transcript at the pre-edit baseline (line 314, session started 10:36Z, first
  edit much later). `content_stream.rs` is MM from another worker's edits.

## Files changed

- `crates/pdftract-core/src/parser/resources.rs` — field widening, merge arm, `lookup_properties`, docs
- `crates/pdftract-core/src/parser/marked_content_operators.rs` — Dict/Ref dispatch in both consumers, `prop_dict_get` dual-key helper, 7 new unit tests, coverage-map update
- `crates/pdftract-core/tests/marked_content_properties_fixtures.rs` — harness warming of indirect `/Properties` targets, assertion updated to new API, `#[ignore]` removed from the direct-dict test

Note: `resources.rs` also carries a benign stranded working-tree edit from a previous
worker (`#[cfg(test)] use PdfDict` hoisted to the top of the file); it rides along in this
pathspec commit since the file is touched anyway.

## Blast radius

`grep -rln 'parse_bdc|lookup_properties|merge_resources|.properties'` across
`crates/*/src` and `crates/*/tests` matches only `pdftract-core` files: the two touched
modules, `parser/mod.rs`, `parser/pages.rs`, `content_stream.rs` (its two `parse_bdc`
call sites are signature-compatible and unchanged), and the fixture test. No other crate
builds against the changed API (`cargo check --all-targets` on the workspace-relevant
crate is clean).
