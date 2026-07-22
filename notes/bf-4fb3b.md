# bf-4fb3b — Errors array format & consolidated findings for the STREAM_DECODE_ERROR assertion

**Parent:** bf-4g6dj · **Depends on (completed):** [[bf-2hjaa]] (location) · **Phase:** explore
**Role:** consolidation bead — compiles the findings of every child bead in this chain into one
actionable guide for the implementer (bf-4bx00 → parent [[bf-mzf4i]]).

---

## 0. TL;DR — the reconciled answer in one screen

There are **two different "errors" representations** in this codebase, and they are *not*
interchangeable. Confusing them is the single most likely way for the implementer to land a
vacuous or failing test:

| Representation | Type | Where it is populated | Is `truncated-flate.pdf`'s error observable here? |
|---|---|---|---|
| **JSON `Output.errors`** | `Vec<DiagnosticJson>` (`schema/mod.rs:1539`) | full extraction path (`extract_pdf` → `result_to_output`), where `emit!()` collectors are threaded | **No.** The truncated page is not enumerable → `pages == []`, `error_count == 0`, `diagnostics == []`. |
| **Decode outcome** | `Result<Vec<u8>, String>` (`stream_decoder_fixtures.rs:301`) | the `StreamDecoder` low-level path the fixture loop runs | **Yes — but as `Ok(partial)`, never `Err`, and never as a collected `DiagCode`.** |

**Therefore the STREAM_DECODE_ERROR assertion lives in
`crates/pdftract-core/tests/stream_decoder_fixtures.rs::test_all_stream_decoder_fixtures()`**,
keyed off the **decode `Result`/byte outcome**, **not** in a full-extraction test that reads
`output.errors`. This is the headline reconciliation of this chain and **retracts the
"complete test example" in the sibling note [[bf-4cyyf]]** (§2 below), which asserts
`output.errors.iter().any(|e| e.code == "STREAM_DECODE_ERROR")` on `truncated-flate.pdf` —
that pattern finds nothing on this fixture and must not be used.

---

## 1. The errors array — format and location (AC-1)

### 1a. Internal `ExtractionResult` (unit/integration-test surface)

There is **no single top-level `errors` array** on `ExtractionResult`
(`crates/pdftract-core/src/extract.rs:237`). Error/diagnostic information is spread across
three fields, verified against current source:

```rust
// src/extract.rs
pub struct ExtractionResult { ... }                       // :237

pub struct PageResult {                                   // :290
    pub error: Option<String>,  // per-page error if THAT page failed   // :336
    ...
}

pub struct ExtractionMetadata {                           // :396
    pub error_count: usize,       // number of pages that failed to extract   // :410
    pub diagnostics: Vec<String>, // document-level diagnostics as "CODE: message" strings  // :416
    ...
}
```

- `metadata.error_count` — count of pages whose extraction failed.
- `metadata.diagnostics` — `Vec<String>` of document-level diagnostics, each formatted
  `"CODE: message"` (e.g. `"STREAM_DECODE_ERROR: zlib stream truncated mid-inflation"`).
  **Skipped from JSON when empty.**
- `PageResult.error` — per-page failure message. **Skipped from JSON when `None`.**

### 1b. JSON `Output.errors` (the structured array — integration/JSON surface)

`result_to_output()` (in `output/json.rs`) converts the internal diagnostics into the
structured top-level array exposed to JSON consumers:

```rust
// src/schema/mod.rs
pub struct Output {                                       // :1487
    pub schema_version: String,
    pub metadata: DocumentMetadata,
    pub pages: Vec<PageJson>,
    pub extraction_quality: ExtractionQuality,
    pub errors: Vec<DiagnosticJson>,   // ← THE structured errors array    // :1539
}

pub struct DiagnosticJson {                               // :813
    pub code: String,                 // stable id, e.g. "STREAM_DECODE_ERROR"
    pub message: String,              // human-readable description
    pub severity: String,             // "info" | "warning" | "error" | "fatal"
    pub page_index: Option<usize>,    // None for document-level events
    pub location: Option<ObjectLocationJson>,
    pub hint: Option<String>,
}

pub struct ObjectLocationJson {                           // :841
    pub object_number: u32,           // zero-based xref index
    pub generation_number: u16,       // incremented on each save
}
```

### 1c. Severity levels and code naming

| Severity | Impact | `STREAM_DECODE_ERROR`? |
|---|---|---|
| `info` | output unaffected | — |
| `warning` | output usable but degraded | **yes — this code is `warning`** |
| `error` | region/page invalid; others OK | — |
| `fatal` | extraction aborted | — |

Codes are `LAYER_REASON` SCREAMING_SNAKE_CASE. The code in play here is **exactly**
`"STREAM_DECODE_ERROR"` — enum `DiagCode::StreamDecodeError`
(`src/diagnostics.rs:465`, string at `:1278`). The bead chain's original title string
`STREAM_DECOMPRESS_ERROR` **does not exist in the codebase** (settled by [[bf-348zd]] §0).

### 1d. Lower-layer diagnostic vectors (NOT aggregated onto this path)

For completeness, the per-layer `diagnostics: Vec<Diagnostic>` collections in
`parser/stream.rs`, `content_stream.rs`, `font/*`, `parser/catalog.rs`, `parser/xref.rs`,
etc., are where `emit!(diagnostics, STREAM_DECODE_ERROR, ...)` actually fires — **but they are
not aggregated onto `ExtractionResult` for the code path this assertion concerns.** They only
reach `Output.errors` when the real extraction pipeline traverses the truncated stream, which
for `truncated-flate.pdf` it does not (§2).

---

## 2. The reconciliation: why `Output.errors` is the WRONG place for this fixture (AC-2)

This is the correction that this chain establishes and that [[bf-4cyyf]]'s "complete test
example" gets backwards.

**Observed (verified by [[bf-2goux]], running `dump_extraction_result` on
`tests/fixtures/malformed/truncated-flate.pdf`):**

```
ExtractionResult {
    pages: [],                       // truncated page is NOT enumerable
    metadata: ExtractionMetadata {
        page_count: 0,
        error_count: 0,              // no page-level failure recorded
        diagnostics: [],             // empty
        ...
    },
    signatures: [], form_fields: [], links: [], attachments: [],
    threads: [], javascript_actions: [],
}
```

Extraction completes cleanly (no panic, no `Err`). Because `pages` is empty, nothing ever
traverses the truncated FlateDecode stream, so **no `STREAM_DECODE_ERROR` diagnostic is ever
emitted on this path**, and `result_to_output(&result).errors` is empty. An assertion of the
form

```rust
let output = result_to_output(&result);
assert!(output.errors.iter().any(|e| e.code == "STREAM_DECODE_ERROR"));  // ← ALWAYS FAILS / vacuous
```

cannot pass for `truncated-flate.pdf` today. The existing scaffold in
`tests/test_truncated_flate_recovery.rs` already documents this:
`test_truncated_flate_materialize_pages` notes "the slice is expected to be empty" (≈:186–187),
`test_truncated_flate_extract_page_returns_result` notes `extract_page(0)` returns `Err`
index-out-of-bounds (≈:301–304), and `test_truncated_flate_emits_diagnostics` states verbatim
that "Diagnostics are not currently surfaced through parse_pdf_file" (≈:78–88).

**A true end-to-end `STREAM_DECODE_ERROR` emission assertion belongs in the integration layer,
and is blocked on the truncated page becoming enumerable** — separate work, out of scope for
this chain. For the unit test, the decode outcome is the only available signal.

---

## 3. Where to add the assertion — the canonical location (AC-2)

**File:** `crates/pdftract-core/tests/stream_decoder_fixtures.rs`
**Function:** `test_all_stream_decoder_fixtures()` (`:254–346`), the per-fixture loop.
**Insertion point:** on the **`Ok` path of the decode match** (`:302`/`:303`) and/or
**after the byte-comparison block (`continue` at `:325–326`) and before `passed += 1;`
(`:345`)**. Do **not** place the primary selector logic in the `Err` arm (`:304–307`) —
that arm is unreachable for this fixture (§4, EC2).

The loop flow (verified against current source):

1. resolve fixtures dir + read `.bin`/`.expected` (`:264–298`) — missing-file guards first
2. `decode_fixture(&fixture, &input)` (`:301`) → the "extraction"
3. `Err` arm → `failures.push(...)` + `continue` (`:304–307`) — **dead for `flate_truncated`**
4. byte-compare decoded vs `.expected`; mismatch → `failures.push` + `continue` (`:318–326`)
5. bomb-specific checks, `flate_bomb_3gb` only (`:329–343`)
6. **← selector/contract assertion goes here (after `:326`, before `:345`)**
7. `passed += 1;` (`:345`)

This satisfies "after extraction (decode) but before test completion (`passed += 1`)" — the
placement criterion from [[bf-junlj]].

### Why the `Ok` path, not the `Err` arm

`decode_impl` (`src/parser/stream.rs:520–554`) breaks out of its read loop on
`UnexpectedEof` (`:542–544`) **or any decoder error** (`:546–548`) and returns the bytes
accumulated so far (`output`, `:553`), wrapped in `Ok` by every caller. `Err(FilterError)` is
reserved for "couldn't even start decoding" (`:42–55` — `UnknownFilter`, `InvalidParams`,
`EncryptionUnsupported`), none of which a `FixtureFilter::Single("FlateDecode", None)` fixture
can produce. So `decode_fixture` for `flate_truncated` **always returns `Ok(partial)`** (the
26-byte `.bin` inflates to ~13 partial bytes) — the `Err` arm never runs. This is the headline
finding of [[bf-60qj2]] §0 and the key strategy pivot.

---

## 4. The false pass the assertion closes, and the edge cases (AC-2)

`flate_truncated` **currently passes for the wrong reason** (verified):
`tests/stream_decoder/fixtures/flate_truncated.expected` is **0 bytes** (`ls -l` → 0), so the
byte-compare (`:318`, `&decoded[..0] != &[]` → `false`) is vacuous; decode returns
`Ok(partial)`; the loop falls through to `passed += 1`. With `expected_diags: vec![]` (the
current declaration at `:62`) there is also no diag check. **The fixture asserts nothing.**

The edge-case taxonomy (full detail in [[bf-60qj2]] §1, EC1–EC14):

- **No `DiagCode` is collected on this low-level path** (EC1) — `StreamDecoder::decode`
  (`stream.rs:74`) returns `Result<Vec<u8>, FilterError>` with no diagnostics channel. Soft
  errors become `Ok(partial)`; the diag is emitted only where `emit!()` is threaded (full
  extraction), which this test bypasses. ⇒ the selector is a **contract over fixture
  metadata + "did not hard-fail"**, not a runtime observation that the diagnostic fired.
- **The `Err` arm is unreachable here** (EC2) — see §3.
- **Byte-compare is vacuous** (EC3) — 0-byte `.expected`; byte-diff gives zero regression
  protection; the decode-error signal is the *only* meaningful assertion.
- **`expected_diags` is dead data until the loop reads it** (EC4 / [[bf-junlj]] blocker #1)
  — the one-line `vec![]` → `vec![DiagCode::StreamDecodeError]` change (`:62`) is *necessary
  but not sufficient*; the loop must also be extended to consult `fixture.expected_diags`.
- **Per-fixture selector; no cross-contamination** (EC5) — other fixtures carry `StreamBomb`,
  `StreamInvalidJpeg`, `OcrJbig2Unsupported`, `StreamUnknownFilter`. Evaluate
  `fixture.expected_diags.contains(&DiagCode::StreamDecodeError)` per fixture.
- **Partial output may be empty and is not byte-stable** (EC6/EC7) — never use
  `decoded.len()` as an error proxy; never byte-assert partial content.
- **Missing-fixture guards must precede the assertion** (EC9) — place after `:298`.
- **Honor the aggregated-loop failure convention** (EC10) — the test collects
  `failures: Vec<String>` and panics once at the end (`:349+`). **Push to `failures`; do not
  use a loop-aborting bare `assert!`** for the expected outcome.
- **Reach `passed += 1`** (EC11) — the error-expecting branch must still count as passed, or
  the `{passed}/{total}` summary silently undercounts.
- **INV-8 regression guard** (EC12) — if a future change makes `FlateDecoder` return `Err`
  for truncation, the (today-dead) `Err` arm should push a message that explicitly names the
  INV-8 regression ("fixture expects soft partial-recovery, but decode returned hard Err").
- **Unfixable gap** (EC13) — without a length hint, checksum, or collected diag, the assertion
  *cannot* distinguish "correct partial recovery" from "decoder silently completed clean
  output." Document the gap in the test/commit comment.

---

## 5. Complete implementation guide for the implementer (AC-2, AC-3)

The assertion cannot be dropped in verbatim. Ordered steps, all prerequisites already
documented by [[bf-junlj]] / [[bf-348zd]] / [[bf-60qj2]]:

**Step 1 — make `expected_diags` live + set the fixture's expectation (hard prerequisite).**

```rust
// stream_decoder_fixtures.rs:62  (currently vec![])
expected_diags: vec![DiagCode::StreamDecodeError],
```

…and extend the loop body to read `fixture.expected_diags` (it currently never does).

**Step 2 — add the selector on the `Ok` path** (after `:303` / before the byte-compare, or
between `:326` and `:345`). This is the *contract* assertion — the fixture is declared to
expect a decode error and decode did not hard-fail (we are in the `Ok` arm):

```rust
let expects_decode_error =
    fixture.expected_diags.contains(&DiagCode::StreamDecodeError);

if expects_decode_error {
    // Contract assertion (bf-60qj2 EC1/EC13): STREAM_DECODE_ERROR is NOT collected on this
    // low-level StreamDecoder path — soft errors return Ok(partial) per INV-8
    // (stream.rs:542-544). We assert the *contract*: the fixture declares a decode error and
    // decode did not hard-fail (Ok arm). Falls through to passed += 1 (EC11).
    // (do NOT byte-assert partial content — EC7; do NOT use decoded.len() — EC6)
}
```

**Step 3 — INV-8 regression guard in the (today-dead) `Err` arm** (`:304`):

```rust
Err(e) => {
    if fixture.expected_diags.contains(&DiagCode::StreamDecodeError) {
        failures.push(format!(
            "{}: expected soft partial-recovery (Ok) for a decode-error fixture, \
             but decode returned hard Err (INV-8 regression?): {}",
            fixture.name, e
        ));
        continue;
    }
    failures.push(format!("{}: {}", fixture.name, e));
    continue;
}
```

**Step 4 — failure message contract** (from [[bf-348zd]] §3): lead with `fixture.name`,
name **both** the string (`STREAM_DECODE_ERROR`) and the enum (`DiagCode::StreamDecodeError`),
state expected-vs-observed, push to `failures` (not bare `assert!`), and do not interpolate
raw decoded bytes (a byte **count** is fine).

**Pattern to mirror (existing codebase style):** `error_recovery_integration.rs::
test_truncated_mid_stream` (≈:164–188) — filter the expected codes and
`assert!(!…is_empty(), "…")`.

> **Do NOT run the full `test_all_stream_decoder_fixtures` to verify** (per [[bf-60qj2]] §6 /
> `~/CLAUDE.md` disk rules): it includes the ~2 GB `flate_bomb_3gb` fixture and is slow /
> disk-heavy. The `Ok(partial)` outcome for `flate_truncated` is unambiguous from
> `decode_impl:542–544`; a scoped check of the single fixture is sufficient.

---

## 6. Source-of-truth line references (all verified against current source)

| Artifact | Location |
|---|---|
| `DiagCode::StreamDecodeError` enum variant | `src/diagnostics.rs:465` |
| `"STREAM_DECODE_ERROR"` string | `src/diagnostics.rs:1278` |
| `ExtractionResult` / `PageResult.error` / `ExtractionMetadata` / `error_count` / `diagnostics` | `src/extract.rs:237, 290, 336, 396, 410, 416` |
| `Output.errors` / `DiagnosticJson` / `ObjectLocationJson` | `src/schema/mod.rs:1539, 813, 841` |
| `FlateDecoder` soft-error → `Ok(partial)` (`UnexpectedEof` break) | `src/parser/stream.rs:542–544` (decode_impl `:520–554`) |
| `FixtureInfo` struct / `expected_diags` field | `tests/stream_decoder_fixtures.rs:18, 22` |
| `flate_truncated` declaration / `expected_diags: vec![]` | `tests/stream_decoder_fixtures.rs:60, 62` |
| `decode_fixture` / `test_all_stream_decoder_fixtures` loop | `tests/stream_decoder_fixtures.rs:220, 254` |
| loop: decode `:301` · `Err` arm `:304–307` · byte-compare `:318–326` · bomb block `:329–343` · `passed += 1` `:345` | same file |
| fixture files (26-byte `.bin`, **0-byte `.expected`**, `.meta`) | `tests/stream_decoder/fixtures/flate_truncated.{bin,expected,meta}` |
| full-extraction emptiness (the "wrong home") | `tests/test_truncated_flate_recovery.rs:78–88, 186–187, 301–304` |

---

## 7. Acceptance criteria status (this bead)

- [x] **Complete documentation of errors array format** — §1: internal `ExtractionResult`
  (`error_count`, `diagnostics`, `PageResult.error`) and structured JSON `Output.errors`
  (`DiagnosticJson` / `ObjectLocationJson`), with verified line refs and severity/naming.
- [x] **Clear guide for assertion implementation** — §3 (canonical location: the fixture-loop
  `Ok` path, *not* the full-extraction `Output.errors` path) + §5 (ordered steps, ready-to-adapt
  selector + INV-8 guard code, failure-message contract) + §4 (false-pass + EC1–EC13 taxonomy).
- [x] **All findings consolidated and actionable** — §0 reconciliation table + §2 retraction of
  the `output.errors` approach; cross-links to every child note below.

## 8. Child-bead findings consolidated

| Bead | Contribution | Status |
|---|---|---|
| [[bf-2goux]] | Ran extraction on `truncated-flate.pdf`; documented empty result + 3 error/diag locations | incorporated §1a, §2 |
| [[bf-junlj]] | Assertion **location**: `stream_decoder_fixtures.rs` loop after `:326`/before `:345`; naming correction; dead-`expected_diags` blocker | incorporated §3, §5 |
| [[bf-348zd]] | Assertion **requirements**: selector/`expected_diags.contains`, polarity flip, failure-message template, E1–E5 | incorporated §4, §5 |
| [[bf-60qj2]] | **Strategy pivot** to the `Ok` path (E3 resolved: `Ok(partial)`, not `Err`); EC1–EC14 taxonomy; the false-pass root cause | incorporated §3, §4 |
| [[bf-2hjaa]] | Location decision reconciled (fixture loop, not `test_truncated_flate_recovery.rs`); final handoff | incorporated §0, §2 |
| [[bf-4ix4m]] / [[bf-4482v]] | Error-structure analysis (`ExtractionMetadata.diagnostics: Vec<String>`, `Output.errors: Vec<DiagnosticJson>`) | incorporated §1 |
| [[bf-303t6]] | Current test assertions & coverage gaps | incorporated §2 |

## 9. References

- Parent: [[bf-4g6dj]] (scaffold/extraction-result examination) → [[bf-mzf4i]] (umbrella) →
  genesis `pdftract-qkc77`.
- Implementer chain: bf-4bx00 (implement assertion), bf-2897m (failure messages + compile).
- Sibling (use with care): [[bf-4cyyf]] — errors-array guide is accurate, but its "complete
  test example" that asserts `output.errors` on `truncated-flate.pdf` is **retracted by this
  note's §0/§2** (that path emits nothing for this fixture).
