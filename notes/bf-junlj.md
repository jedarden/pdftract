# bf-junlj — Identify assertion location for the STREAM_DECODE_ERROR check

**Parent:** bf-4g6dj · **Depends on:** bf-2goux (extraction result) · phase: explore

## TL;DR

The assertion belongs in **`crates/pdftract-core/tests/stream_decoder_fixtures.rs`**,
inside **`test_all_stream_decoder_fixtures()`**, in the per-fixture loop **after the
byte-comparison block (after line 326) and before `passed += 1;` (line 345)** — i.e.
after the fixture is decoded/"extracted" but before the iteration completes.

It is a **presence check** styled after
`error_recovery_integration.rs::test_truncated_mid_stream` (lines 164–188):
assert the expected diagnostic is present, not absent.

## Naming correction

The bead title says `STREAM_DECOMPRESS_ERROR`. **No such code exists.** The real
diagnostic is:

- Enum: `DiagCode::StreamDecodeError` (`src/diagnostics.rs:465`)
- String: `"STREAM_DECODE_ERROR"` (`src/diagnostics.rs:1278`)

Use `DiagCode::StreamDecodeError` / `"STREAM_DECODE_ERROR"` everywhere.

## Why NOT the full-extraction test

bf-2goux established that running full extraction on
`tests/fixtures/malformed/truncated-flate.pdf` surfaces **no** error signal:
`pages == []`, `metadata.error_count == 0`, `metadata.diagnostics == []`. The
truncated page is simply not enumerable. Therefore an assertion placed in
`test_truncated_flate_recovery.rs::test_truncated_flate_extraction_result_structure`
(lines 119–178) — the natural "after extraction, before completion" slot — would
**fail**, because that path emits nothing to assert on. That test is the wrong home.

## The correct home: stream_decoder_fixtures.rs

The `flate_truncated` fixture is declared here:

```rust
// crates/pdftract-core/tests/stream_decoder_fixtures.rs:59-64
FixtureInfo {
    name: "flate_truncated",
    filter: FixtureFilter::Single("FlateDecode", None),
    expected_diags: vec![],          // <-- should be vec![DiagCode::StreamDecodeError]
    bomb_limit: None,
},
```

### Exact assertion location (in the test flow)

`test_all_stream_decoder_fixtures()` (lines 254–360) loops each fixture:

1. reads `.bin` + `.expected` (264–298)
2. `decode_fixture(&fixture, &input)` — the "extraction" (301–308)
3. compares decoded bytes vs `.expected` (310–326)
4. bomb-specific checks (328–343)
5. `passed += 1;` (345)

**Insert the diagnostic assertion between step 3/4 and step 5 — after line 326,
before line 345.** That satisfies the acceptance criterion "after extraction but
before test completion."

### Assertion TYPE: presence check

- Type: **presence** (does the expected diag appear?), not a count/exact-match.
- For `flate_truncated`, the truncated FlateDecode stream makes `decode_fixture()`
  return `Err(String)` (via the `.map_err(|e| format!("Decode error: {}", e))` at
  line 230). Note the loop currently treats that `Err` as a **failure** (lines
  302–308, `continue`) — so the assertion logic must special-case fixtures whose
  `expected_diags` contains `StreamDecodeError`: for those, an `Err`/partial
  outcome is the *expected, passing* result, and its absence is the failure.

### Pattern to follow (existing codebase style)

`error_recovery_integration.rs::test_truncated_mid_stream` (lines 164–188):

```rust
let stream_diags: Vec<_> = expected
    .expected_diagnostics
    .iter()
    .filter(|d| d.code.contains("STREAM_DECODE"))
    .collect();
assert!(
    !stream_diags.is_empty(),
    "Should expect STREAM_DECODE_ERROR diagnostic"
);
```

Mirror this: filter for the expected code and `assert!(!…is_empty(), "…")` with a
descriptive message.

## Blockers the implementer (bf-4bx00) must handle first

The assertion cannot just be dropped in as-is. Two gaps:

1. **`expected_diags` is currently dead data.** It is declared on `FixtureInfo`
   (line 22) and populated for several fixtures, but the loop (254–360) **never
   reads `fixture.expected_diags`**. The loop only diffs bytes. So the loop must be
   extended to consult `fixture.expected_diags` before the assertion means anything.

2. **`StreamDecoder::decode()` returns `Result<Vec<u8>, String>`, not diagnostics.**
   There is no `DiagCode` collector on this low-level path. So "presence of
   `StreamDecodeError`" is observable only as the `Err(_)` arm of `decode_fixture`
   (or as a short/partial byte count), not as a collected `DiagCode`. The assertion
   must key off the `Result`, not a diagnostics vector — unless the decode path is
   first refactored to thread a collector (out of scope for this chain).

Plus the one-line fixture-data change at line 62:
`expected_diags: vec![]` → `expected_diags: vec![DiagCode::StreamDecodeError]`.

## Acceptance criteria (this bead)

- [x] Assertion location identified: `stream_decoder_fixtures.rs`,
      `test_all_stream_decoder_fixtures()`, after line 326 / before line 345.
- [x] Assertion type determined: presence check (keyed off `Err`/partial decode for
      `expected_diags` containing `StreamDecodeError`).
- [x] Follows existing pattern: `error_recovery_integration.rs:174-184`
      (`filter(...).is_empty()` presence assert).
- [x] Location is after extraction (decode) but before test completion (`passed += 1`).
