# pdftract-966fddc5 — tagged PDF fixtures with indirect and direct /Properties entries

Split child 1/4 of bf-1a61w9 (Resolve marked content property references).
Test material only — no parser changes.

Date: 2026-09-14

## What was produced

| Artifact | Path |
|---|---|
| Fixture (indirect `/Properties` entry) | `tests/fixtures/tagged/mc_properties_indirect.pdf` (779 B) |
| Fixture (direct inline `/Properties` dict) | `tests/fixtures/tagged/mc_properties_direct.pdf` (736 B) |
| Fixture README (object tables, provenance, gap analysis) | `tests/fixtures/tagged/README.md` |
| PROVENANCE registry entries | `tests/fixtures/PROVENANCE.md` (two appended blocks, SHA256 recorded) |
| Integration tests | `crates/pdftract-core/tests/marked_content_properties_fixtures.rs` |

Correction honored: the parent's cited `extract_properties_from_dict()` does
not exist; the real chain is `parse_bdc`
(`crates/pdftract-core/src/content_stream.rs:959` call site) →
`extract_mcid_from_props` / `extract_ocg_ref_from_props`
(`crates/pdftract-core/src/parser/marked_content_operators.rs`), fed by
`ResourceDict::lookup_properties()` (`crates/pdftract-core/src/parser/resources.rs:93`).

## Fixture design

Both fixtures are one-page PDF 1.7 files whose content stream runs
`/P /MC0 BDC BT /F1 12 Tf 72 700 Td (Tagged content) Tj ET EMC`. They differ
only in how `/MC0` is bound in the page `/Resources`:

- **indirect**: `/Properties << /MC0 4 0 R >>`, object 4 =
  `<< /MCID 0 /ActualText (Tagged content) >>` (objects 1–6, `/Size 7`).
- **direct**: `/Properties << /MC0 << /MCID 0 /ActualText (Tagged content) >> >>`
  (objects 1–5, no property object).

Full object tables live in `tests/fixtures/tagged/README.md`.

## Test results (actual)

Command: `timeout --kill-after=30s 600s cargo test -p pdftract-core --test
marked_content_properties_fixtures` (nextest is not installed on this box; the
timeout-wrapped `cargo test` fallback is the documented alternative). Tests
locate the fixtures via `CARGO_MANIFEST_DIR/../../tests/fixtures/tagged`.

```
running 3 tests
test bdc_name_props_direct_inline_dict_recovers_mcid ... ignored, merge_resources() (crates/pdftract-core/src/parser/resources.rs, /Properties merge) drops direct inline property dicts because it only stores entries where obj.as_ref() is Some; /MC0 -> << /MCID 0 ... >> cannot be recovered until the child-2 fix (bf-1a61w9) lands
test fixture_indirect_property_object_parses_to_mcid_dict ... ok
test bdc_name_props_indirect_reference_recovers_mcid ... FAILED

---- bdc_name_props_indirect_reference_recovers_mcid stdout ----
assertion `left == right` failed: MCID must be recovered from the /MC0 target dict (4 0 R -> << /MCID 0 /ActualText ... >>); diagnostics: ["BDC property 'MC0' resolved to non-dict object"]
  left: None
 right: Some(0)

test result: FAILED. 1 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out
```

The dispatch requires the indirect-ref test's actual result to be recorded;
it **FAILS**, and the failure is the finding. With `--include-ignored`, the
direct-dict test fails at its first assertion:
`the direct inline /MC0 dict was dropped by merge_resources (child-2)` —
exactly the documented gap.

### Why the indirect case fails (two stacked defects, both in-tree today)

1. `XrefResolver::resolve` (`crates/pdftract-core/src/parser/xref.rs:272`) is
   cache-only: for an entry not in the cache it returns `PdfObject::Null`
   ("stub: return Null for now — use resolve_with_source instead").
   `extract_mcid_from_props` therefore gets Null back for object 4 and emits
   `BDC property 'MC0' resolved to non-dict object`. Full byte-backed
   resolution exists only as `resolve_with_source` (xref.rs:306), which
   `parse_bdc` does not receive; `process_with_mode` passes `None` outright
   (content_stream.rs) and `execute_with_do` threads only an
   `Option<&XrefResolver>`.
2. Even with a resolved dict in hand, parsed dictionary keys carry **no
   leading slash** (the lexer's `lex_name` consumes `/`; the object parser
   interns the bare bytes), so object 4 parses to keys `MCID`/`ActualText`,
   while `extract_mcid_from_dict` (marked_content_operators.rs) reads
   `dict.get("/MCID")` — a guaranteed miss on any real file. The unit tests
   mask this by hand-building cached dicts with `intern("/MCID")` keys.

The passing integrity test
`fixture_indirect_property_object_parses_to_mcid_dict` proves the fixture and
xref are well-formed (object 4 resolves via `resolve_with_source` to a dict
with `MCID` = `Integer(0)`), so both defects are squarely in the resolution
path, not the test material.

### Direct-dict drop (confirmed as dispatched)

`merge_resources` (resources.rs, `/Properties` merge) gates each entry on
`obj.as_ref()`; a direct inline dict yields `None` and is silently dropped,
so `lookup_properties("MC0")` misses and `parse_bdc` emits
`UnknownMarkedContentProps`. Structural note for child-2:
`ResourceDict.properties` is `IndexMap<Arc<str>, ObjRef>` — keeping a direct
dict requires widening that type; the `/ColorSpace` namespace in the same
function already stores raw `PdfObject`s unconditionally (in-tree precedent).

## Acceptance criteria

- PASS — fixtures committed under `tests/fixtures/tagged/` with README; both
  registered in `tests/fixtures/PROVENANCE.md` with generator, purpose, and
  SHA256. Files are 779/736 bytes; no binaries, no renders.
- PASS — indirect-ref fixture test committed and run; actual result (FAIL
  with captured output above) recorded here. The test is intentionally left
  live/red as the TDD-red target for child-2 (repo precedent: `649e76ec`).
- PASS — direct-dict test committed, `#[ignore]`d with a message naming
  `merge_resources()` and the `obj.as_ref()` mechanism.
- PASS — no parser changes: the diff touches only `tests/fixtures/tagged/*`,
  `tests/fixtures/PROVENANCE.md`, and
  `crates/pdftract-core/tests/marked_content_properties_fixtures.rs`.
- WARN — full `cargo test --all-targets` was not used as the gate: the tree
  is shared with concurrent workers and carries ~300 pre-existing test
  failures, bare `cargo test` fail-fasts before integration tests run, and a
  full-suite run would execute other workers' in-flight test code (the exact
  TH-03 socket/orphan hazard this repo's test-hygiene rules prohibit leaving
  behind). The new test file was therefore run in isolation
  (`-p pdftract-core --test marked_content_properties_fixtures`) under the
  mandated `timeout --kill-after=30s 600s` wrapper; it compiles clean and
  reports 1 passed / 1 failed (expected red) / 1 ignored.
- PASS — no server subprocesses, no bound sockets; the only processes run
  were cargo/rustc test invocations, all exited.

## Pointers for child-2 (bf-1a61w9 remainder)

1. Make marked-content property resolution source-backed (thread a
   `&dyn PdfSource` / `resolve_with_source` path into `parse_bdc`, or warm
   `/Properties` targets into the resolver cache when pages load).
2. Read `/MCID` (and `/OCG`) from resolved dicts using slash-free keys, or
   normalize keys at parse time — the two conventions currently disagree.
3. Widen `ResourceDict.properties` (or mirror `/ColorSpace`) so direct inline
   property dicts survive `merge_resources`, then un-ignore
   `bdc_name_props_direct_inline_dict_recovers_mcid` — the fixture is ready.
