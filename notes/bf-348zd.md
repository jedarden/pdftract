# bf-348zd — Assertion requirements for the STREAM_DECODE_ERROR check

**Parent:** bf-4g6dj · **Depends on:** bf-junlj (assertion location) · phase: explore

This note specifies *exactly what the assertion must validate and how* for the
`flate_truncated` fixture in
`crates/pdftract-core/tests/stream_decoder_fixtures.rs`. It is the contract the
implementer (bf-4bx00) codes against. Location was already pinned by
[[bf-junlj]]; this note pins the semantics.

---

## 0. Naming — settled

The bead/title string **`STREAM_DECOMPRESS_ERROR` does not exist** in the
codebase. Use the real code everywhere:

| Concern            | Value                                              |
| ------------------ | -------------------------------------------------- |
| Enum variant       | `DiagCode::StreamDecodeError`                       |
| String form        | `"STREAM_DECODE_ERROR"`                             |
| Source of truth    | `src/diagnostics.rs:465` (enum), `:1278` (string)  |

Assertions that compare against a code string MUST use `"STREAM_DECODE_ERROR"`
(exact spelling, all caps, single underscore-separated). `.meta` for the fixture
already states this: `"FlateDecode: mid-stream EOF; expects partial bytes +
STREAM_DECODE_ERROR"`.

---

## 1. Expected error code string (AC-1)

- **Expected code string:** `"STREAM_DECODE_ERROR"`
- **Expected enum:** `DiagCode::StreamDecodeError`
- Exactly **one** such diagnostic is expected for `flate_truncated`. No other
  diag codes are expected from this fixture.

Fixture-data change this depends on (one line, `stream_decoder_fixtures.rs:62`):

```rust
// before
expected_diags: vec![],
// after
expected_diags: vec![DiagCode::StreamDecodeError],
```

---

## 2. Assertion logic (AC-2)

### 2a. What "observable" means here — the core constraint

`StreamDecoder::decode()` returns `Result<Vec<u8>, String>` — **it does not emit
`DiagCode`s.** There is no diagnostics collector on this low-level path. So
"presence of `StreamDecodeError`" is **not** observable as a collected `DiagCode`
in this test. It is observable only through the `decode_fixture()` outcome. The
assertion logic must therefore key off the **`Result` / byte outcome**, driven by
whether `fixture.expected_diags` contains `DiagCode::StreamDecodeError`.

This is why `expected_diags` is treated as a **selector** ("does this fixture
expect a decode error?"), not as a literal list to diff against a produced
diagnostics vector (no such vector exists on this path).

### 2b. Logic type: **presence check** (drive on the selector)

- Type: **presence**, not count-exact and not ordering-sensitive.
- Predicate: `expects_decode_error =
  fixture.expected_diags.contains(&DiagCode::StreamDecodeError)`.
- Style it after the existing presence pattern in
  `error_recovery_integration.rs:174-184`
  (`filter(...).is_empty()` → `assert!(!is_empty, "…")`).

### 2c. The polarity flip — do not reuse the generic Err→failure path

The loop today (lines 301-308) treats **any** `Err` from `decode_fixture()` as a
test *failure* (`failures.push(...); continue;`). For a fixture that
`expects_decode_error`, that polarity is **inverted**: a decode error is the
**expected, passing** outcome, and its **absence** is the failure.

So the implementer MUST special-case the decode-result handling. Pseudocode
(placed at the decode-result match, ~line 301, replacing the blanket-Err arm for
error-expecting fixtures):

```rust
let expects_decode_error =
    fixture.expected_diags.contains(&DiagCode::StreamDecodeError);

let result = decode_fixture(&fixture, &input);

let decoded = match result {
    Ok(data) => {
        // A clean full decode when an error was expected is itself a failure.
        // (Partial-bytes-Ok is allowed — see edge case E3.)
        data
    }
    Err(e) => {
        if expects_decode_error {
            // EXPECTED: truncated stream surfaced a decode error. Pass this
            // fixture and move on — the byte-compare block below does not apply.
            passed += 1;
            continue;
        }
        failures.push(format!("{}: {}", fixture.name, e));
        continue;
    }
};
```

If `decode_fixture` instead returns `Ok(partial_bytes)` for the truncated stream
(FlateDecoder may recover partial output — see E3), the presence check must fall
back to a **byte-shortfall** signal: assert the decode did **not** produce a
clean/complete result. Because `flate_truncated.expected` is **empty (0 bytes)**
(verified: `ls -l` → 0-byte `.expected`), a produced full output cannot be
distinguished from expected by byte-diff alone; the error signal is the only
discriminator. See E2.

### 2d. Where the assertion sits in the flow

Per [[bf-junlj]]: inside `test_all_stream_decoder_fixtures()`, in the per-fixture
loop, **after the byte-comparison block (after line 326) and before
`passed += 1;` (line 345)** — i.e. after decode/extraction, before iteration
completes. The `Err`-arm special-case in 2c short-circuits earlier (at the decode
match) for the truncated fixture; the after-326 slot is the home for the
`Ok(partial)` fallback assertion when the decoder recovers bytes.

---

## 3. Failure message template (AC-3)

Presence-check failure (expected the decode error, did not get it):

```rust
assert!(
    saw_decode_error,
    "{}: expected STREAM_DECODE_ERROR (DiagCode::StreamDecodeError) from \
     truncated FlateDecode stream, but decode completed cleanly",
    fixture.name
);
```

Requirements for any message the implementer writes:

- Lead with `fixture.name` so a failure in the aggregated loop is attributable.
- Name **both** the string (`STREAM_DECODE_ERROR`) and the enum
  (`DiagCode::StreamDecodeError`) so grep finds it either way.
- State the *expected* condition and the *observed* condition ("expected X, but
  Y") — never a bare `assert!(cond)` with no message.
- Do not interpolate raw decoded bytes into the message (fixtures can be large /
  binary); a byte **count** is fine.

Aggregated-loop note: this test collects `failures: Vec<String>` and panics once
at the end (lines 349-356). Prefer pushing a formatted string to `failures`
(consistent with the surrounding code) over a bare `assert!` that aborts the
whole loop early. If using `assert!`, be aware it stops all later fixtures.

---

## 4. Edge cases (AC-4)

- **E1 — Multiple / ordering:** Only **one** diag (`StreamDecodeError`) is
  expected for this fixture; ordering is irrelevant. Do **not** write an
  order-sensitive or exact-count-across-codes assertion. Presence is sufficient.
  (Other fixtures like `flate_bomb_3gb` expect `StreamBomb`; keep the selector
  per-fixture so codes don't cross-contaminate.)

- **E2 — Empty `.expected` file:** `flate_truncated.expected` is **0 bytes**.
  The existing byte-comparison (lines 318-326) is therefore vacuous for this
  fixture (`expected_bytes.len() == 0` ⇒ the slice compare trivially passes).
  This means the byte-diff **cannot** be the pass/fail signal — the decode-error
  presence is the *only* meaningful assertion. Do not rely on byte-diff to catch
  regressions here.

- **E3 — Err vs partial-Ok ambiguity:** The truncated stream (26-byte `.bin`,
  mid-stream EOF) may surface either as `Err(String)` **or** as
  `Ok(partial_bytes)` depending on FlateDecoder's recovery behavior. The `.meta`
  says "partial bytes + STREAM_DECODE_ERROR", implying partial recovery is
  plausible. The assertion MUST accept **both** as the expected outcome and treat
  only a *clean, complete, error-free* decode as the failure. The implementer
  (bf-4bx00) should confirm the actual runtime outcome first (run the fixture)
  and shape the discriminator accordingly. **This is the one open runtime fact
  to nail down before coding.**

- **E4 — Fixture missing / not generated:** The loop already handles absent
  `.bin`/`.expected` (lines 267-277) by pushing a failure and `continue`. The new
  assertion must live *after* those guards so it never runs on a missing fixture.

- **E5 — Selector must not silently no-op:** If `expected_diags` for the fixture
  is left as `vec![]` (the pre-change state), `expects_decode_error` is `false`
  and the new logic is dead — the truncated decode error would be (mis)reported
  as a generic failure again. The one-line fixture-data change (§1) is a hard
  prerequisite; flag it in the same commit as the assertion.

---

## 5. Acceptance criteria (this bead)

- [x] **Expected error code string documented** — `"STREAM_DECODE_ERROR"` /
      `DiagCode::StreamDecodeError`; the bead's `STREAM_DECOMPRESS_ERROR` does not
      exist (§0, §1).
- [x] **Assertion logic clearly specified** — presence check driven by an
      `expected_diags.contains(StreamDecodeError)` selector, keyed off the
      `decode_fixture` `Result`/byte outcome (no DiagCode collector on this path),
      with the Err→pass polarity flip made explicit (§2).
- [x] **Failure-message template defined** — `"{name}: expected
      STREAM_DECODE_ERROR … but decode completed cleanly"`, with rules for
      attribution and expected-vs-observed phrasing (§3).
- [x] **Edge cases documented** — single-diag/no-ordering, empty `.expected`,
      Err-vs-partial-Ok ambiguity, missing-fixture guard ordering, and the
      selector-no-op prerequisite (§4).

## 6. Handoff to implementer (bf-4bx00)

1. First: run the truncated fixture, record whether decode yields `Err` or
   `Ok(partial)` (resolves E3).
2. Apply the one-line fixture-data change at `stream_decoder_fixtures.rs:62`
   (E5).
3. Add the selector + polarity-flipped decode handling (§2c) and the
   presence/shortfall assertion with the §3 message.
4. Keep it a `failures.push(...)`-style entry to match the aggregated-loop
   convention, not a loop-aborting bare `assert!`.
