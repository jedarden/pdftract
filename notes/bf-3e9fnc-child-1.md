# Catalog of Test Function Signature Issues (bf-3e9fnc child 1)

**Bead:** bf-2o61br (child of bf-3e9fnc "Standardize test function signatures across
integration tests")
**Generated:** 2026-09-05
**Tree state audited:** commit `db3c89d9` ("fix(bf-68f2x6): resolve unused and unresolved
imports"), working tree clean except `.beads/*`.
**Supersedes:** the 2026-08-09 audit in `notes/test-signature-audit.md` (bead bf-23kpx5),
which concluded "0 critical issues". That conclusion is no longer true: HEAD now has
**431 compile errors inside test code**, and **507 test functions are not compiled by
cargo at all**.

---

## Method

1. `cargo metadata --no-deps` → authoritative list of the 96 `test` targets across the 7
   workspace members.
2. Module-closure walk from every target root (handling `#[path]`, `mod.rs` vs
   `<stem>.rs` child directories, crate roots, and `#[cfg(...)] mod`) → set of files that
   actually reach the compiler. Any test-bearing file outside that set is invisible to
   `cargo test`.
3. Whole-tree static parse of every `fn` (491 files, 9,461 functions) capturing attributes
   (`#[test]`, `#[tokio::test]`, `#[cfg_attr(..., test)]`, `#[ignore]`,
   `#[should_panic]`), parameters, and return type.
4. `cargo check --workspace --all-targets --message-format=json` → machine-parsed
   compiler ground truth (434 errors), each error located and classified as test code vs
   library code.
5. Call-site search for every `test_*`-named function lacking a test attribute, to
   distinguish live helpers from dead tests.

Limitations: test-module membership for `src/` errors is detected by brace-depth tracking
of the nearest preceding `#[cfg(test)]`; attribute parsing handles nesting but not
attributes separated from the `fn` by a non-attribute item. Counts may be off by ±1 in
edge cases; every headline number was cross-checked against a second method.

---

## Executive summary

| # | Category | Count | Severity |
|---|----------|------:|----------|
| 1 | Compile errors in test code — wrong arity (`E0061`) | 37 | Blocks all of pdftract-core's unit tests |
| 2 | Compile errors in test code — missing imports/values (`E0425`/`E0433`/`E0422`) | 311 | Blocks all of pdftract-core's unit tests |
| 3 | Compile errors in test code — API drift (field/method/trait/type: `E0609`/`E0599`/`E0277`/`E0308`) | 80 | Blocks all of pdftract-core's unit tests |
| 4 | Compile errors in test code — missing macro (`json!`) | 3 | Blocks all of pdftract-core's unit tests |
| 5 | Compile error in library code (arity, `pdftract-cli`) | 1 | Blocks 30 pdftract-cli test targets |
| 6 | Test functions in files cargo never compiles | 507 (408 attributed + 99 proptest) in 53 files | Silent: harness never sees them |
| 7 | `test_*`-named functions without any test attribute (live code) | 8 | Naming/harness-recognition hazard |
| 8 | Async (`#[tokio::test]`) tests | 38, all signatures valid | 8 of them in orphaned files (cat. 6) |
| 9 | proptest properties carrying a redundant `#[test]` | 138 (137 `in strategy`, 1 typed-arg outlier) | Cosmetic/inconsistency |
| 10 | Tests silently skipped (`#[ignore]`) | 16 | Intentional, but unrecorded in one place until now |
| 11 | Tests behind non-default feature gates | ~294 gate occurrences (`proptest`, `decrypt` is default; `remote`/`ocr`/`profiles`/`cjk`/`receipts`/`grep`/`shape-db` are not) | Silent unless CI enables the feature |
| 12 | `TestState`-style shared-state test pattern | 0 occurrences | Negative finding — do not look for one |

---

## Category 1 — Compile errors in test code (431 total, 21 files)

`cargo check --workspace --all-targets` fails. The `pdftract-core` **lib test target does
not compile at all**, so *zero* unit tests in `pdftract-core` can currently run. 430 of
the 431 errors are in `#[cfg(test)]` modules; 1 is in library code
(`crates/pdftract-cli/src/grep/worker.rs:394`).

### 1a. Wrong arity — `E0061` (37 in test code + 1 in lib code)

Production signatures changed; test call sites did not.

| File | n | Detail |
|------|--:|--------|
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | 24 | Tests call `detect_char_proc_type(&obj, None, 0)` with 3 args; the function (line 192) now takes 2. The 3-arg form still exists as `detect_char_proc_type_with_depth` (line 213). 10 call sites pass 3 args to the 2-arg fn (lines 2935, 2944, 3620, 3630, 3637, 3646, 3655, …); 14 pass 2 args to the 3-arg `_with_depth` fn (lines 3739, 3752, 3765, 3792, 3806, …). **Fix:** route each call at the variant that matches its arg count. |
| `crates/pdftract-core/src/parser/marked_content_operators.rs` | 10 | "takes 7 arguments but 6 were supplied" — lines 370, 391, 409, 514, … |
| `crates/pdftract-core/src/font/type3_rasterizer_test.rs` | 2 | "takes 3 arguments but 2 were supplied" — lines 1676, 2512 |
| `crates/pdftract-core/src/content_stream.rs` | 1 | "takes 7 arguments but 6 were supplied" — line 3194 |
| `crates/pdftract-cli/src/grep/worker.rs` (**lib code, not a test**) | 1 | "takes 5 arguments but 4 were supplied" — line 394. Because this is in the library, **all 30 pdftract-cli test targets fail to type-check** until it is fixed. |

### 1b. Missing imports/values in test scope — `E0425` (209) + `E0433` (101) + `E0422` (1)

Every symbol below still exists in the crate; the test modules simply lost their `use`
lines (fallout from the import-cleanup commit `db3c89d9` and earlier pruning). **Fix is
mechanical: re-add the import.**

| File | Code | n | Missing symbol | Where it actually lives |
|------|------|--:|----------------|--------------------------|
| `crates/pdftract-core/src/parser/pages.rs` | E0425 | 108 | `intern` | `crate::parser::object::types::intern` (also re-exported at `crate::parser::object::intern`) |
| `crates/pdftract-core/src/parser/catalog.rs` | E0425 | 94 | `intern` | same |
| `crates/pdftract-core/src/layout/correction.rs` | E0433 | 43 | `UnicodeSource` | `crate::font::resolver::UnicodeSource` |
| `crates/pdftract-core/src/parser/resources.rs` | E0433 | 20 | `PdfDict` | `crate::parser::object::PdfDict` |
| `crates/pdftract-core/src/detection.rs` | E0433 | 12 | `ObjRef` | `crate::diagnostics::ObjRef` |
| `crates/pdftract-core/src/cache/key.rs` | E0433 | 10 | `Map` | module-local alias — no `type Map` exists anywhere; the alias lives in the non-test module. Restore the alias import alongside the type. |
| `crates/pdftract-core/src/cache/lru.rs` | E0425 | 7 | `entry_path` | `crate::cache::layout::entry_path` |
| `crates/pdftract-core/src/encryption/detection.rs` | E0433 | 4 | `DiagCode` | `crate::diagnostics::DiagCode` |
| `crates/pdftract-core/src/table/output.rs` | E0433 | 4 | `TableSpan` | `crate::table::cell::TableSpan` |
| `crates/pdftract-core/src/signature/mod.rs` | E0433 | 3 | `PdfDict` | `crate::parser::object::PdfDict` |
| `crates/pdftract-core/src/javascript.rs` | E0433 | 2 | `ObjRef`, `PdfDict` | as above (line 302) |
| `crates/pdftract-core/src/annotation/json.rs` | E0422 | 1 | `DestArray` | `crate::annotation::links::DestArray` |
| `crates/pdftract-core/src/layout/figure.rs` | E0433 | 1 | `Arc` | `std::sync::Arc` (line 274) |
| `crates/pdftract-core/src/output/markdown/links.rs` | E0433 | 1 | `FitType` | `crate::annotation::links::FitType` |
| `crates/pdftract-core/src/parser/catalog.rs` | E0433 | 1 | `IndexMap` | `indexmap::IndexMap` (line 1080) |

### 1c. API drift — tests calling a changed API (80)

| File | Code | n | What changed |
|------|------|--:|--------------|
| `crates/pdftract-core/src/classify.rs` | E0609 | 70 | `classify_page` was changed by `942a795e` ("add Result<> error propagation … to classify_page") to return `ClassificationResult<PageClassification>`. Tests still do `result.class`, `result.confidence`, `result.hybrid_cells` directly on the `Result`. Lines 2069–2173, 2688, 2772. **Fix:** `let result = classify_page(&ctx).unwrap();` (or `.expect(...)`) at each call site. |
| `crates/pdftract-core/src/classify.rs` | E0277 | 2 | `ClassificationError: serde::Serialize` not satisfied — `serde_json::to_string(&err)` in tests at lines 2688, 2772. |
| `crates/pdftract-core/src/font/type3_rasterizer.rs` | E0599 | 5 | `CharProcType::Unknown` no longer resolvable in test scope (missing import of the variant's enum), lines 2936, 2945, 3708, 3766, … |
| `crates/pdftract-core/src/render/scanline.rs` | E0308/E0599/E0277 | 3 | mismatched types (line 926), `is_nan` not found (929), `{float}` bound (938) — test uses a numeric literal type the helper no longer produces. |

### 1d. Missing macro (3)

| File | n | Detail |
|------|--:|--------|
| `crates/pdftract-core/src/schema/mod.rs` | 3 | `cannot find macro json` at lines 2172, 2195, 2221 — the `#[cfg(test)] use serde_json::json;` was pruned. |

---

## Category 2 — Test functions cargo never compiles (507 functions, 53 files)

The workspace root `Cargo.toml` is a **pure virtual workspace** (no `[package]`
section). Cargo therefore auto-discovers **no** test targets at the repo root, and no
`[[test]]` entry points at them either. Verified via `cargo metadata`: of the 96 `test`
targets in the workspace, **zero** are under the root `tests/` directory.

Consequences: these tests are invisible to `cargo test`, `cargo nextest`, and CI. They
still *look* alive in an editor and in grep-based audits, which is how the 2026-08-09
audit concluded everything was healthy.

### 2a. Root `tests/` tree — 43 files, 333 attributed tests + 99 proptest properties

Full list (attributed `#[test]`/`#[tokio::test]` counts in brackets):

```
tests/cli_integration.rs [2]                    tests/cli_integration_simple.rs [2]
tests/document_model.rs [15]                    tests/encryption_errors.rs [8]
tests/encryption_fixtures.rs [28]               tests/encryption_fixtures_usage_example.rs [5]
tests/fingerprint_fixtures.rs [4]               tests/fingerprint_reproducibility.rs [17]
tests/fingerprint_test_single_one.rs [2]        tests/fixture_discovery.rs [8]
tests/forms_integration.rs [5]                  tests/json_schema.rs [6]
tests/log_secret_fuzz.rs [10 + 3 proptest]      tests/object_parser.rs [1]
tests/proptest-panic-verification.rs [1]        tests/smoke_test.rs [3]
tests/stream_decoder_fixtures.rs [16]           tests/test_assertion_methods.rs [10]
tests/test_bomb_limit.rs [1]                    tests/test_cases.rs [1]
tests/test_extract_content_stream_bytes.rs [6]  tests/test_fingerprint_debug.rs [1]
tests/test_fixture_discovery_simple.rs [1]      tests/test_helpers.rs [1]
tests/test_page_access.rs [9]                   tests/test_pdftract_core_error_imports.rs [2]
tests/test_ref_type.rs [1]                      tests/test_round.rs [1]
tests/test_unmapped_glyphs.rs [2]               tests/verify_cli_helper.rs [3]
tests/verify_encryption_fixtures.rs [13]
tests/document_model/mod.rs [15]                tests/integration/hybrid_fixtures.rs [12]
tests/integration/advanced/profiles.rs [4]      tests/remote/integration.rs [13, 8 async]
tests/security/TH-05-ssrf-block.rs [7]
tests/proptest/cmap_parser.rs [10]              tests/proptest/document_model.rs [3]
tests/proptest/lexer.rs [17]                    tests/proptest/object_parser.rs [25]
tests/proptest/stream.rs [21]                   tests/proptest/stream_decoder.rs [5]
tests/proptest/xref.rs [16]
```

**At least 12 of these are name-matched, drifted near-duplicates of live crate-level
files** — root copies were forked and then left behind when tests moved into crates:

| Orphaned root copy | Live counterpart |
|---|---|
| `tests/document_model.rs` | `crates/pdftract-core/tests/document_model.rs` |
| `tests/fingerprint_reproducibility.rs` | `crates/pdftract-core/tests/fingerprint_reproducibility.rs` |
| `tests/fixture_discovery.rs` | `crates/pdftract-cli/tests/fixture_discovery.rs` |
| `tests/forms_integration.rs` | `crates/pdftract-cli/tests/forms_integration.rs` |
| `tests/json_schema.rs` | `crates/pdftract-core/tests/json_schema.rs` |
| `tests/object_parser.rs` | `crates/pdftract-core/tests/object_parser.rs` |
| `tests/stream_decoder_fixtures.rs` | `crates/pdftract-core/tests/stream_decoder_fixtures.rs` |
| `tests/test_page_access.rs` | `crates/pdftract-core/tests/test_page_access.rs` |
| `tests/proptest/{document_model,object_parser}.rs` | `crates/pdftract-core/tests/*` |
| `tests/security/TH-05-ssrf-block.rs` | `crates/pdftract-{core,cli}/tests/TH-05-ssrf-block.rs` |

All pairs were spot-checked with `cmp` and are **not** byte-identical — they have
diverged, so neither "delete the root copy" nor "the live copy covers it" is automatic.
Each pair needs an explicit reconcile decision (which copy has the newer coverage?).
Also orphaned with the tree: root `src/` (`src/lib.rs`, `src/Codegen/`,
`src/page_helper.rs`) — a support-library twin of the live
`crates/pdftract-core/src/graphics_state.rs` lives at `src/graphics_state/` and is dead.

**Fix options (decision needed, not mechanical):** (a) declare explicit `[[test]]`
targets in a package for the files worth keeping, (b) migrate unique coverage into the
crate-level test files and delete the root tree, or (c) delete the root `tests/` tree
outright if the crate copies superseded it. Do **not** just re-point targets blindly —
several root files are drifted forks, so re-enabling them can resurrect stale coverage
over newer crate-level tests.

### 2b. Dead module files inside `pdftract-core` — 31 tests

These source files are declared by no `mod` anywhere and are never compiled:

| File | Tests | Note |
|------|------:|------|
| `crates/pdftract-core/src/timeout.rs` | 3 | "subprocess timeout protection" — module not wired into `lib.rs` |
| `crates/pdftract-core/src/font/type3_charproc_test.rs` | 10 | sibling `type3_rasterizer_test.rs` *is* wired in (`font/mod.rs:24`); this one is not |
| `crates/pdftract-core/src/output/json.rs` | 4 | `output/mod.rs` declares only `inspector`, `markdown`, `ndjson`, `sink` |
| `crates/pdftract-core/src/output/multi.rs` | 7 | as above |
| `crates/pdftract-core/src/output/pipeline.rs` | 7 | superseded by `output/ndjson/pipeline.rs`, which *is* declared |

**Fix:** either add the `mod` declaration (if the module is wanted) or delete the file.
Note `output/json.rs` vs the live `annotation/json.rs`: the latter is wired in and
compiles; the former is a stranded earlier implementation.

### 2c. `xtask` — 9 tests outside the workspace

`xtask/src/migrate/mod.rs` holds 9 `#[test]` functions, but `xtask` is **not** a
workspace member (it is invoked via `--manifest-path`, e.g. the `gen-schema` alias), so
`cargo test --workspace` never runs them. **Fix:** add an opt-in member or document that
xtask tests require `cargo test --manifest-path=xtask/Cargo.toml`.

---

## Category 3 — `test_*`-named functions without a test attribute (9)

These compile but are **not** discovered by the harness — the name says "test", the
attribute says "helper". All 8 live ones were verified to be called from real tests in
the same file, so they are helpers, not lost tests; the naming is the hazard (they show
up in `grep 'fn test_'` audits and in test-runner filters as if they were tests).

| Location | Signature | Call sites |
|----------|-----------|-----------:|
| `crates/pdftract-core/tests/test_page_access.rs:19` | `fn test_fixture_path() -> PathBuf` | 6 |
| `crates/pdftract-core/tests/object_parser.rs:124` | `fn test_fixture(name: &str)` | 11 |
| `crates/pdftract-core/tests/cjk_encoding.rs:59` | `fn test_cjk_fixture(fixture: &CjkFixture) -> Result<String, Box<dyn Error>>` | 4 |
| `crates/pdftract-core/tests/schema_validate_fixtures.rs:115` | `fn test_fixture(fixture: &Fixture)` | 6 |
| `crates/pdftract-core/tests/document_model.rs:121` | `fn test_fixture(fixture: Fixture)` | 15 |
| `crates/pdftract-core/tests/encoding_recovery.rs:109` | `fn test_encoding_fixture(fixture: &EncodingFixture) -> Result<FixtureResult, Box<dyn Error>>` | 5 |
| `crates/pdftract-py/tests/test_search_integration.rs:27` | `fn test_search_scaffold(fixture_path: &Path)` | 1 |
| `crates/pdftract-core/src/profiles/match_eval.rs:383` | `fn test_signals() -> FeatureSignals` | 9 |

(The ninth, `tests/document_model/mod.rs:74 test_fixture(fixture: Fixture)`, sits in the
orphaned root tree — see 2a.)

**Fix (mechanical, low risk):** rename to non-`test_` prefixes — `fixture_path`,
`run_fixture`, `extract_fixture`, `make_fixture`, `fixture_signals`. No behaviour change;
preserve the parameters exactly as they are. This is the only place the parent bead's
"standard patterns: test functions take no args or only &mut TestState" guidance applies
literally, and no fixture-taking function should be *converted* into a test — they are
genuinely parameterised helpers.

## Category 4 — Async tests (separate list, as required)

**38 `#[tokio::test]` functions in the tree. All 38 have correct signatures** — every one
is `async fn`, takes no parameters, and returns `()`. Zero cases of `#[test]` on an
`async fn`, zero `#[tokio::test]` on a sync fn, zero `async fn` with parameters.

| File | Count | Notes |
|------|------:|-------|
| `crates/pdftract-core/tests/remote_mock_server_tests.rs` | 12 | lines 79–940; live |
| `crates/pdftract-core/tests/remote_tls_tests.rs` | 8 | lines 16–182; live |
| `crates/pdftract-core/tests/remote_integration.rs` | 5 | lines 100–457, named `critical_1`…`critical_5`; live |
| `crates/pdftract-cli/src/serve.rs` | 3 | lines 1115, 1174, 1259 (unit tests); live |
| `crates/pdftract-cli/src/middleware/csp.rs` | 1 | line 42 (unit test); live |
| `crates/pdftract-core/tests/test_416_debug.rs` | 1 | line 10; live |
| **`tests/remote/integration.rs`** | **8** | lines 249–580 — **orphaned**, in the uncompiled root `tests/` tree (Category 2a). These are the only async tests with a recognition failure, and it is the file-target problem, not the signatures. |

Note: `remote_*` test files are additionally gated behind the non-default `remote`
feature (Category 6), so they are doubly invisible: feature-off by default *and*
root-tree copies of overlapping coverage.

## Category 5 — proptest (138 properties)

- 137 properties are declared `#[test] fn prop_*(x in strategy)` inside `proptest! {}`
  blocks (root `tests/proptest/*.rs`, `crates/pdftract-core/tests/`, and unit-test
  modules in `src/parser/{catalog,pages,outline,xref,stream,object/parser,hint_stream}.rs`).
  The `#[test]` inside a `proptest!` block is redundant — the macro generates its own
  harness entry — but harmless.
- **1 outlier:** `crates/pdftract-core/src/parser/hint_stream.rs:850` uses the typed-arg
  form `fn prop_parse_hint_stream_no_panic(data: Vec<u8>)` instead of
  `data in any::<Vec<u8>>()`. It resolves today (`Vec<u8>: Strategy`), but it is the only
  property in the codebase written that way. **Fix:** normalise to the `in` form for
  consistency.

## Category 6 — Tests that compile but are skipped by default

- **16 `#[ignore]`-attributed tests** (never run without `--ignored`):
  `crates/pdftract-core/src/ocr.rs` (7), `tests/ocr_integration.rs` (3),
  `src/cache/compression.rs` (2), `src/ocr/preprocessing/{otsu,sauvola}.rs` (2),
  `tests/TH-03-mcp-no-auth.rs` (1), `tests/classify_page_error_paths.rs` (1).
  The memory-guard suite additionally gates its tests
  `#[cfg_attr(not(target_os = "windows"), test)]` with per-test `#[ignore = "memory limit
  tests interfere with each other…"]` — intentional, but worth a single documented list.
- **Feature-gated test code** (default features are only `serde`, `decrypt`,
  `quick-xml`): `proptest` ×105, `decrypt` ×92 (default on), `remote` ×70, `ocr` ×39,
  `profiles` ×35, `cjk` ×27, `receipts` ×9, `grep` ×9 gate occurrences. Any of
  `remote`/`ocr`/`profiles`/`cjk`/`receipts`/`grep` being off means the corresponding
  tests do not exist in a default build. CI must pass `--all-features` (or an explicit
  feature list) for these to run at all.

## Category 7 — Environment / build context found during the audit

- `crates/pdftract-core/build.rs` aborts the build when
  `crates/pdftract-core/build/CHECKSUMS.sha256` is absent (the TH-06 supply-chain gate).
  It was missing at the start of this audit and was restored by the concurrent worker in
  commit `577fa0cd`. It is now staged/committed — do not delete it as a "generated
  artifact"; the build cannot start without it.
- The build also warns that `/home/coding/pdftract/build/unmapped-glyph-names.json` and
  `build/glyph-shapes.json` are missing, silently degrading to an empty glyph-shape
  database. Not a signature issue, but it changes test behaviour for encoding tests.
- **No `pdftract` workflow runs exist in iad-ci**, which is why 431 test-compile errors
  could land on `main` unnoticed. Even a single `cargo check --all-targets` run would
  have caught every error in Category 1.

## `TestState` — negative finding

The task asked for `TestState` usage patterns to be identified. **No `TestState` type
exists anywhere in this codebase** (no `struct TestState`, no `&mut TestState`
parameters, no `setup() -> State` helpers). Tests here use plain local construction plus
free-function fixture helpers (Category 3). Any fix plan that assumes a shared-state
pattern can be dropped.

---

## Recommended fix order for the parent bead (bf-3e9fnc)

1. **Unblock the build** — Category 1b (mechanical `use` re-adds, 311 errors, ~30 min)
   then 1c (classify.rs `Result` unwraps, 70), then 1a (arity, 37), then 1d (3). After
   this, `cargo check --workspace --all-targets` must be clean; that is the parent bead's
   verification gate.
2. **pdftract-cli lib arity fix** (`grep/worker.rs:394`) — one line, unblocks 30 test
   targets.
3. **Decide the fate of root `tests/`** (Category 2a) — needs a human-visible decision
   per drifted pair; do not fold into the mechanical pass.
4. **Dead module files** (2b) — wire in or delete, per file.
5. **Rename the 8 `test_*` helpers** (Category 3) — mechanical, no signature change.
6. Normalise the single proptest typed-arg outlier (Category 5).
7. Re-enable CI (`pdftract-ci` template) with `--all-features` so Category 6 tests and
   this whole class of regressions are actually exercised.

## Artifacts / reproduction

```bash
# authoritative target list (proves root tests/ is in no package)
cargo metadata --format-version 1 --no-deps | jq -r '.packages[].targets[] |
  select(.kind[]=="test") | .src_path' | sort -u

# current compile error set, machine readable
cargo check --workspace --all-targets --message-format=json 2>/dev/null |
  jq -r 'select(.reason=="compiler-message") | select(.message.level=="error") |
  [.message.code.code, .message.spans[0].file_name, .message.spans[0].line_start,
   .message.message] | @tsv'
```

Companion data: `/tmp/sigscan/{fns.json,errors.csv,orphans_final.txt}` on ex44
(ephemeral; regenerate with the commands above if needed).
