# bf-2hjaa — Assertion location for the STREAM_DECODE_ERROR validation

**Parent:** bf-4g6dj · **Blocked on:** bf-4ix4m (error-structure analysis), bf-60qj2 (edge-case / false-pass analysis) · phase: explore

## TL;DR (the reconciled answer)

The STREAM_DECODE_ERROR assertion belongs in the **`stream_decoder_fixtures.rs`
per-fixture loop**, on the **`flate_truncated`** fixture — **not** in the
full-extraction test `test_truncated_flate_recovery.rs`, which is what an earlier
draft of this note proposed and is the key correction below.

- **Location:** `crates/pdftract-core/tests/stream_decoder_fixtures.rs`,
  inside `test_all_stream_decoder_fixtures()`, in the per-fixture loop **after the
  byte-comparison block (the `continue` at line 326) and before `passed += 1;`
  (line 345)**.
- **What it validates:** a **decode-error contract** for fixtures declared to
  expect one — i.e. that a fixture whose `expected_diags` contains
  `StreamDecodeError` did in fact fail/partial-decode, and did *not* pass cleanly.
  It is **not** a `metadata.diagnostics` presence check.
- **Purpose/scope:** guard against the current **false-pass** on `flate_truncated`
  (0-byte `.expected` makes byte-compare vacuous → `passed += 1` for the wrong
  reason), and pin the `DiagCode::StreamDecodeError` expectation to the fixture.

This bead is the *location* decision. The assertion's detailed logic, failure
message, and edge cases are owned by [[bf-348zd]] (requirements) and
[[bf-60qj2]] (edge cases); the implementer is [[bf-4bx00]] / parent [[bf-mzf4i]].

## §0 Correction of this note's earlier (Jul 6) draft

The previous version of this note proposed asserting on
`extraction_result.metadata.diagnostics` inside a new
`test_truncated_flate_recovery()` built on `extract_pdf()`. **That location does
not work** and the premise is retracted. Three verified reasons:

1. **The truncated page is not enumerable under full extraction.** The existing
   scaffold in `test_truncated_flate_recovery.rs` already establishes this:
   `test_truncated_flate_materialize_pages` documents "the slice is expected to be
   empty" (lines 186–187), and `test_truncated_flate_extract_page_returns_result`
   documents that `extract_page(0)` returns `Err` (index out of bounds) on this
   fixture (lines 301–304). With an empty page slice, nothing traverses the
   truncated FlateDecode stream, so no diagnostic is ever produced on that path.

2. **Diagnostics are not surfaced through the parse path the scaffold uses.**
   `test_truncated_flate_emits_diagnostics` (lines 78–88) states verbatim:
   "Diagnostics are not currently surfaced through parse_pdf_file … Once
   diagnostics are exposed, this test should check for truncation warnings." So
   even structurally, the full-extraction test has no field to assert against
   today.

3. **`FlateDecoder` returns `Ok(partial)` on truncation, never `Err`.**
   `stream.rs:45` documents "Soft errors (corrupt data, EOF mid-stream) return
   `Ok(partial_bytes)`", and `UnexpectedEof` is swallowed at `stream.rs:542`. So
   even at the decoder level there is no `Err` signal for the full pipeline to
   propagate into a diagnostic on this fixture. (This is [[bf-60qj2]]'s headline
   finding.)

The API types the draft referenced are real but irrelevant to the location
decision: `extract_pdf` (extract.rs:547) returns `ExtractionResult` (extract.rs:237)
with `ExtractionMetadata` (extract.rs:396) whose `diagnostics: Vec<String>`
(extract.rs:416) is genuinely a `Vec<String>` — see [[bf-4ix4m]]. They just stay
empty for this fixture, so an assertion there is vacuous.

## §1 Assertion purpose and scope (AC-1)

**Purpose:** make `flate_truncated` assert what it is *for* — that a truncated
FlateDecode stream is detected as a decode failure rather than silently passing.

**Scope of the check:** the single `flate_truncated` fixture in the stream-decoder
fixture loop. The assertion is a **selector/contract** assertion:

- The fixture is *declared* to expect a decode error
  (`expected_diags: vec![DiagCode::StreamDecodeError]`).
- The decode of that fixture must reflect that expectation — it must **not**
  complete as a clean pass.

It is explicitly **out of scope** for this bead (and flagged as an unfixable gap
in [[bf-60qj2]]) to assert that a `DiagCode::StreamDecodeError` value is *emitted*
on this path: `StreamDecoder::decode()` returns `Result<Vec<u8>, String>` and has
no diagnostics collector, and `FlateDecoder` converts EOF to `Ok(partial)`, so the
code is never produced here. A true emission assertion belongs in the integration
layer once the truncated page is enumerable — separate work.

## §2 Optimal assertion location (AC-2)

**File:** `crates/pdftract-core/tests/stream_decoder_fixtures.rs`
**Function:** `test_all_stream_decoder_fixtures()` (lines 253–360)
**Insertion point:** in the per-fixture loop, **after the byte-comparison block
(`continue` at line 326) and before `passed += 1;` (line 345)** — i.e. right where
line 344 sits today.

The loop flow (verified):

1. read `.bin` + `.expected` (264–298)
2. `decode_fixture(&fixture, &input)` (301)
3. `Err` arm → `failures.push(...)` + `continue` (302–308)
4. byte-compare decoded vs `.expected`; mismatch → `failures.push` + `continue` (310–326)
5. bomb-specific checks, `flate_bomb_3gb` only (328–343)
6. **← assertion goes here (line 344)**
7. `passed += 1;` (345)

This satisfies "after extraction (decode) but before test completion
(`passed += 1`)" — the placement criterion from the sibling [[bf-junlj]] analysis.

**Why here and not `test_truncated_flate_recovery.rs`:** §0 above. That file has
no error signal to assert against; this loop is the only place the truncated
FlateDecode bytes are actually decoded.

## §3 Assertion requirements (AC-3)

The assertion cannot be dropped in verbatim. It requires these prerequisites,
all already documented by [[bf-junlj]] and [[bf-60qj2]]:

1. **Make `expected_diags` live.** Today `FixtureInfo.expected_diags` (declared
   line 22, populated for several fixtures) is **dead data** — the loop never
   reads it; it only diffs bytes. The loop must be extended to consult
   `fixture.expected_diags` before the assertion has any meaning.

2. **Set the fixture's expectation.** One-line change at line 62:
   `expected_diags: vec![]` → `expected_diags: vec![DiagCode::StreamDecodeError]`.

3. **Key the assertion off the decode outcome, not a diagnostics vector.**
   `StreamDecoder::decode()` has no diagnostics channel on this low-level path,
   and `FlateDecoder` returns `Ok(partial)` on EOF. So the contract is observed
   via the `Result`/byte outcome (the `Err` arm, or a short/partial byte count),
   not via a collected `DiagCode`. See [[bf-60qj2]] §3–§4 for the ready-to-adapt
   selector code.

4. **Handle the polarity flip.** For fixtures expecting `StreamDecodeError`, an
   `Err`/partial outcome is the *passing* result, and a clean pass is the
   *failure*. The current `Err` arm (302–308) treats `Err` as failure — it must
   be special-cased for `StreamDecodeError`-expecting fixtures. Note
   [[bf-60qj2]]'s refinement: for `flate_truncated` specifically the `Err` arm is
   *dead* (decode yields `Ok(partial)`), so in practice the selector keys off the
   `Ok`-with-short-bytes outcome, and the `Err`-arm flip is an INV-8 regression
   guard for the day the decoder is changed to surface EOF as `Err`.

5. **Failure message.** Descriptive, with attribution and expected-vs-observed
   phrasing; template from [[bf-348zd]] AC-3:
   `"{name}: expected STREAM_DECODE_ERROR (DiagCode::StreamDecodeError) from
   truncated FlateDecode stream, but decode completed cleanly"`.

6. **Pattern.** Mirror `error_recovery_integration.rs::test_truncated_mid_stream`
   (lines 164–188): filter the expected codes and `assert!(!…is_empty(), "…")`.

## §4 Edge cases (summary — full taxonomy in [[bf-60qj2]])

- **EC1** single expected diag, no ordering concern.
- **EC2** 0-byte `.expected` makes the byte-diff vacuous (the false-pass root
  cause the assertion exists to close).
- **EC3** `Err` vs `Ok(partial)` ambiguity — resolved by [[bf-60qj2]]:
  `flate_truncated` yields `Ok(partial)`, so the selector targets the `Ok` path.
- **EC4** missing-fixture guard ordering — keep the existing `fixture_path.exists()`
  guards (268–277) ahead of the new check.
- **EC5** selector is a no-op until prerequisite §3.1 lands — the assertion must
  not silently pass while `expected_diags` is still unread.

## Acceptance criteria status

- [x] **Assertion purpose and scope defined** (§1): a decode-error contract for the
  `flate_truncated` fixture, closing the current false-pass; explicitly *not* a
  `metadata.diagnostics` emission assertion.
- [x] **Optimal assertion location identified** (§2):
  `stream_decoder_fixtures.rs::test_all_stream_decoder_fixtures()` loop, after the
  byte-compare block (line 326) / before `passed += 1` (line 345). Not the
  full-extraction test.
- [x] **Assertion requirements documented** (§3): make `expected_diags` live, set
  the fixture's expectation, key off the decode outcome, polarity-flip with
  INV-8 guard, descriptive message, follow the `error_recovery_integration.rs`
  pattern.

## References

- Sibling notes: [[bf-junlj]] (location), [[bf-348zd]] (requirements),
  [[bf-60qj2]] (edge cases + false-pass), [[bf-4ix4m]] (error structure).
- Implementer / parent: [[bf-4bx00]], [[bf-mzf4i]], [[bf-4g6dj]].
- `DiagCode::StreamDecodeError` — `src/diagnostics.rs:465`; string
  `"STREAM_DECODE_ERROR"` — `src/diagnostics.rs:1278`.
- Fixture loop — `tests/stream_decoder_fixtures.rs:253-360`; `flate_truncated`
  declaration — `:59-64` (`expected_diags: vec![]` at `:62`).
- `FlateDecoder` soft-error → `Ok(partial)` — `src/parser/stream.rs:45`,
  `UnexpectedEof` swallow at `:542`.
- Full-extraction emptiness — `tests/test_truncated_flate_recovery.rs:186-187,
  301-304, 78-88`.
- `ExtractionResult`/`ExtractionMetadata.diagnostics: Vec<String>` —
  `src/extract.rs:237, 396, 416`; `extract_pdf` — `:547`.
