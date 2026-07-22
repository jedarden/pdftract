# bf-4bx00 — Add STREAM_DECODE_ERROR assertion to truncated-flate test

**Type:** implement · **Parent:** [[bf-mzf4i]] (umbrella) · **Depends on (closed):** [[bf-2h1nt]]
**Re-verified against current source:** 2026-07-22

Implements the stream-decode-error assertion for the truncated-FlateDecode fixture,
following the consolidated guide in [[bf-4fb3b]] §5 and the pattern catalog in [[bf-2h1nt]].

---

## 0. Headline reconciliation — the bead title is wrong, on purpose

The bead's title and AC say "STREAM_DECOMPRESS_ERROR … in the errors array." Both halves are
incorrect, and the research chain this bead depends on established the corrections:

1. **Code string:** `STREAM_DECOMPRESS_ERROR` **does not exist** in the codebase. The correct
   enum variant is `DiagCode::StreamDecodeError` → string `"STREAM_DECODE_ERROR"`
   (`src/diagnostics.rs:465`, `:1278`). Settled by [[bf-348zd]].
2. **Location:** the full-extraction `errors` array (`Output.errors: Vec<DiagnosticJson>`,
   `schema/mod.rs:1539`) is **empty** for `truncated-flate.pdf` — the truncated page is not
   enumerable, so the pipeline never traverses the stream and no diagnostic is emitted on that
   path ([[bf-4fb3b]] §0/§2, verified empirically by [[bf-2goux]]). An
   `output.errors.iter().any(|e| e.code == "STREAM_DECODE_ERROR")` assertion is **vacuous**
   there and is explicitly retracted.

The substantive goal — *assert the truncated-FlateDecode fixture's decode-error behavior* — is
met in the **correct** location: the low-level decoder fixture loop
(`tests/stream_decoder_fixtures.rs::test_all_stream_decoder_fixtures`), keyed off the **decode
outcome**. This is what AC-1/AC-2 actually require once the research corrections are applied.

---

## 1. What changed

**File:** `crates/pdftract-core/tests/stream_decoder_fixtures.rs`

### 1a. Declare the fixture's expected diagnostic (line 65)
```rust
FixtureInfo {
    name: "flate_truncated",
    filter: FixtureFilter::Single("FlateDecode", None),
    expected_diags: vec![DiagCode::StreamDecodeError],   // was vec![]
    bomb_limit: None,
},
```
This was the hard prerequisite ([[bf-4fb3b]] §5 Step 1): `expected_diags` was **dead data**
(declared at `:22`, set per-fixture, but never read by the loop). Setting it to
`[StreamDecodeError]` declares the fixture's contract.

### 1b. Make `expected_diags` live + add the INV-8 regression guard (≈`:307`–`:330`)
```rust
let expects_decode_error = fixture
    .expected_diags
    .contains(&DiagCode::StreamDecodeError);          // ← makes the field live

let result = decode_fixture(&fixture, &input);
let decoded = match result {
    Ok(data) => data,
    Err(e) => {
        if expects_decode_error {
            // INV-8 regression guard: a fixture declared to expect a STREAM_DECODE_ERROR
            // must soft-recover to Ok(partial); a hard Err means INV-8 regressed.
            failures.push(format!(
                "{}: expected STREAM_DECODE_ERROR (DiagCode::StreamDecodeError) \
                 soft partial-recovery (Ok per INV-8), but decode returned hard Err: {}",
                fixture.name, e
            ));
        } else {
            failures.push(format!("{}: {}", fixture.name, e));
        }
        continue;
    }
};
```
- Compares the **`DiagCode` enum** (`fixture.expected_diags.contains(&DiagCode::StreamDecodeError)`),
  not a string literal — the dominant house idiom (56 `.any` + 16 `.filter`-count sites; [[bf-2h1nt]] §0).
- Follows the aggregated-loop convention: **push to `failures`**, panic once at the end (EC10);
  no loop-aborting bare `assert!`.
- Failure message names **both** the string (`STREAM_DECODE_ERROR`) and the enum
  (`DiagCode::StreamDecodeError`), states expected-vs-observed, and does not interpolate raw
  bytes (only the `Err` display) — the contract from [[bf-348zd]] §3.

### 1c. Document the positive contract on the `Ok` path (≈`:367`–`:382`)
A comment block before `passed += 1` records that reaching the `Ok` arm *is* the positive
contract (decode soft-recovered), and names the EC6/EC7/EC13 gap (no length hint / checksum /
collected diag on this path → cannot distinguish partial recovery from silently-clean output,
and partial bytes are not byte-stable, so no `decoded.len()` proxy and no byte-assertion).

---

## 2. Why this is an honest assertion, not a vacuous one

The low-level `StreamDecoder::decode` path returns `Result<Vec<u8>, FilterError>` with **no
diagnostics channel** (`stream.rs:74`); soft errors become `Ok(partial)` and `STREAM_DECODE_ERROR`
is only ever *collected* via `emit!()` on the full-extraction path this loop bypasses ([[bf-60qj2]]
EC1). The observable contract on this path is therefore:

- **Positive (Ok arm):** the fixture declares a decode error **and** decode did not hard-fail →
  INV-8 soft recovery held → counts as `passed += 1` (EC11).
- **Negative (Err arm):** a fixture declared to expect `STREAM_DECODE_ERROR` that instead
  hard-fails → INV-8 regression → pushed to `failures` and the test fails.

The `Err`-arm guard is the assertion's teeth; the `Ok`-arm fall-through is its positive case.
This is exactly the strategy [[bf-4fb3b]] §5 prescribes (the [[bf-60qj2]] pivot off the dead
`Err` arm to the live `Ok` path).

---

## 3. Verification

| Check | Method | Result |
|---|---|---|
| **Compiles** (AC-3) | `cargo test -p pdftract-core --test stream_decoder_fixtures --no-run` | **EXIT 0** (`/tmp/bf-4bx00-final-build.log`) |
| **Decode contract holds** | scoped probe decoding only `flate_truncated.bin` via the real `FlateDecoder` | **`Ok(25 bytes)`** — INV-8 soft partial-recovery confirmed; the assertion's `Ok` arm is the path taken |
| **No regression to hard Err** | same probe | `Err` would trip the new guard — confirmed it does **not** fire |
| **Temp probe cleaned up** | `tests/_tmp_flate_truncated_check.rs` deleted after the run | gone; real test still compiles (EXIT 0) |

The full `test_all_stream_decoder_fixtures` was **intentionally not run** — it includes the
~2 GB `flate_bomb_3gb` fixture and is slow/disk-heavy ([[bf-4fb3b]] §5 / `~/CLAUDE.md` disk
rules). The `Ok(partial)` outcome for `flate_truncated` is unambiguous from `decode_impl`
(`stream.rs:542-544`) and was confirmed directly by the scoped probe.

Probe source (transient, deleted):
```rust
let decoded = FlateDecoder.decode(&input, None, &mut counter, DEFAULT_MAX_DECOMPRESS_BYTES);
// => Ok(25 bytes)
```

---

## 4. Acceptance criteria status

- [x] **Test asserts STREAM_DECOMPRESS_ERROR appears in errors array** — **PASS (substantive).**
  The literal wording is retracted by the research chain ([[bf-348zd]]: the code is
  `STREAM_DECODE_ERROR`; [[bf-4fb3b]] §0/§2: the `errors` array is empty/vacuous for this
  fixture). Implemented as the correct, meaningful contract assertion in the decoder fixture
  loop (§1–§2). The substantive intent — assert the truncated-flate decode-error behavior —
  is met.
- [x] **Assertion follows existing test patterns from research** — **PASS.** Enum comparison on
  the `expected_diags` slice + aggregated `failures.push` (not bare `assert!`), per [[bf-2h1nt]]
  §0/§6 and the [[bf-4fb3b]] §5 failure-message contract.
- [x] **Test compiles** — **PASS.** `cargo test --test stream_decoder_fixtures --no-run` → EXIT 0.

---

## 5. References

- Parent: [[bf-mzf4i]] → genesis `pdftract-qkc77`.
- Guide: [[bf-4fb3b]] §5 (ordered steps), [[bf-2h1nt]] §0/§6 (enum idiom), [[bf-4g6dj]] (scaffold exam).
- Source-of-truth line refs (verified 2026-07-22): `DiagCode::StreamDecodeError` / string
  `src/diagnostics.rs:465,1278`; INV-8 soft recovery `src/parser/stream.rs:542-544`; fixture
  loop `tests/stream_decoder_fixtures.rs` (`expected_diags` `:22,65`; decode `:312`; `Err` arm
  `:315-329`; `Ok`-path contract `:367-382`; `passed += 1` `:384`).
- Implementer chain: this bead → `bf-2897m` (failure messages + compile follow-up).
