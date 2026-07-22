# bf-60qj2 — Edge cases & robust assertion strategy for the STREAM_DECODE_ERROR check

**Parent:** bf-4g6dj · **Depends on:** bf-348zd (requirements) → bf-junlj (location) ·
**Feeds:** bf-4bx00 (implementer) · phase: explore

> **Naming.** The bead title says `STREAM_DECOMPRESS_ERROR`. That string **does not
> exist** in the codebase (settled by [[bf-348zd]] §0). The real code is
> `DiagCode::StreamDecodeError` / `"STREAM_DECODE_ERROR"`
> (`src/diagnostics.rs:465` / `:1278`). This note uses the real name throughout.

This note extends [[bf-348zd]]'s edge-case list (E1–E5) into a full taxonomy, and —
because the source reading resolves the one runtime fact bf-348zd left open — revises
the assertion strategy accordingly. Every claim below was verified against the current
source, not assumed from prior notes.

---

## 0. TL;DR — the headline finding changes the strategy

[[bf-348zd]] flagged **E3 (Err vs Ok(partial))** as "the one open runtime fact to nail
down before coding." Reading `FlateDecoder` settles it:

> **For a truncated zlib stream, `FlateDecoder::decode()` returns `Ok(partial_bytes)`.
> It never returns `Err`.**

Source: `stream.rs:520–554` (`decode_impl`) breaks out of the read loop on
`UnexpectedEof` (line 542) *or any decoder error* (line 546) and returns the bytes
accumulated so far, wrapped in `Ok` by every caller (`decode_with_fallback` →
`decode_with_predictor` → `StreamDecoder::decode`). `Err(FilterError)` is reserved for
"couldn't even start decoding" (`stream.rs:42–55`, 84–88) — `UnknownFilter`,
`InvalidParams`, `EncryptionUnsupported` — none of which a plain
`FixtureFilter::Single("FlateDecode", None)` fixture can produce.

Consequences that reshape the strategy:

1. **The loop's `Err` arm is dead for `flate_truncated`.** `decode_fixture`
   (`stream_decoder_fixtures.rs:220–251`) wraps the decoder result with `.map_err(...)`,
   but the decoder already returned `Ok`. So execution always takes the `Ok(data)` arm
   (line 302–303), never the `Err` arm (304–308). **[[bf-348zd]] §2c's "Err→pass
   polarity-flip" pseudocode, taken literally, implements nothing** — that branch is
   unreachable here. The assertion logic must live on the **`Ok` path**, not the `Err`
   path.
2. **There is no runtime-observable error signal at all on this path.** The
   `StreamDecoder::decode` trait (`stream.rs:74–99`) returns `Result<Vec<u8>,
   FilterError>` with **no diagnostics collector**. Soft errors become `Ok(partial)`; no
   `DiagCode` is collected. So `STREAM_DECODE_ERROR` is **not observable** as a produced
   diagnostic in `decode_fixture` — the diagnostic is emitted (via `emit!()`) only on the
   real extraction path, which this unit test deliberately bypasses.
3. **`flate_truncated` currently passes for the wrong reasons.** With `expected_diags:
   vec![]`, no diag check, and a **0-byte `.expected`** (verified: `ls -l` → 0 bytes), the
   byte-compare (line 318) is vacuous (`&decoded[..0] != &[]` → `false`), the decode
   returns `Ok(partial)`, and the loop falls through to `passed += 1`. It is a
   **false-pass** today. This is *why* an assertion is needed.

The robust strategy (§4) follows from these three facts: an honest assertion here is a
**selector / contract assertion**, not a runtime-signal assertion.

---

## 1. Edge-case taxonomy

[[bf-348zd]]'s E1–E5 are incorporated (marked ⟦E1⟧…⟦E5⟧) and extended. Grouped by theme.

### Category A — the observable-signal problem (the hard part)

**EC1 — No DiagCode is collected on the low-level path.** ⟦extends the "no collector"
point in [[bf-348zd]] §2a⟧ `StreamDecoder::decode` has no diagnostics channel. Soft
errors return `Ok(partial)`. An assertion that filters a *produced* diagnostics vector
for `"STREAM_DECODE_ERROR"` finds nothing — not because the decoder is broken, but
because the diag is emitted only where an `emit!()` collector is threaded (the full
extraction path), which this test skips.
*Test behavior:* the assertion MUST NOT key off a produced-`DiagCode` vector. There is
none to read.

**EC2 — The `Err` arm is unreachable for this fixture (E3, resolved).** ⟦resolves E3⟧
`decode_impl` (542–549) converts EOF/decoder errors to `Ok(partial)`. The loop's `Err`
arm (304–308) is therefore dead code for `flate_truncated`.
*Test behavior:* any assertion placed in the `Err` arm never executes. The selector logic
must be evaluated on the `Ok` path. (This corrects the orientation of [[bf-348zd]] §2c,
which foregrounded the `Err` arm.)

**EC3 — Vacuous byte-compare.** ⟦E2⟧ `flate_truncated.expected` is 0 bytes.
`expected_bytes.len().min(decoded.len()) == 0`, so `&decoded[..0] != &[]` is `false` →
byte-compare trivially passes.
*Test behavior:* byte-diff provides **zero** regression protection for this fixture and
cannot be the pass/fail signal.

### Category B — selector / metadata

**EC4 — `expected_diags` is dead data until the loop reads it.** ⟦E5 / [[bf-junlj]]
blocker #1⟧ `FixtureInfo.expected_diags` (line 22) is populated but the loop (262–346)
**never reads it** — verified by reading the full loop body.
*Test behavior:* the one-line change `vec![]` → `vec![DiagCode::StreamDecodeError]`
(line 62) is **necessary but not sufficient**; the loop must also be extended to consult
`fixture.expected_diags`, else the selector is a no-op.

**EC5 — Per-fixture selector; no cross-fixture contamination.** ⟦E1⟧ Other fixtures
carry different codes (`StreamBomb`, `StreamInvalidJpeg`, `OcrJbig2Unsupported`,
`StreamUnknownFilter`). The selector must be evaluated **per fixture**:
`fixture.expected_diags.contains(&DiagCode::StreamDecodeError)`.
*Test behavior:* never assert "any fixture produced this code globally"; scope to the
current fixture only.

### Category C — decode-outcome shape (given `Ok(partial)` is the path)

**EC6 — Partial output may be empty.** `decode_impl` returns whatever `output` was
accumulated before EOF. If truncation lands before any flushed deflate output (e.g. cut
inside the zlib header or first block), `output` is `Vec::new()`. `flate_truncated`'s
26 bytes DO yield ~13 bytes (the `78 9c f3 48…` prefix inflates to ASCII text), so this
fixture is non-empty today — but the assertion must not *assume* `decoded.len() > 0`.
Combined with EC3, a 0-length decoded + 0-length expected is doubly vacuous.
*Test behavior:* never use `decoded.len()` as an error proxy.

**EC7 — Partial output content is not byte-stable.** The bytes returned before EOF
depend on `flate2`'s internal buffering / `BOMB_CHECK_CHUNK` boundaries. A `flate2`
version bump can shift the exact partial byte sequence for identical input.
*Test behavior:* never byte-assert the *content* of partial output for an error fixture.
(If content stability is ever wanted, populate `.expected` and reclassify the fixture as
a content fixture — at which point it is no longer an error fixture.)

**EC8 — Deflate-fallback interaction.** `decode_with_fallback` (497–515) retries with
`DeflateDecoder` only when ZlibDecoder output is empty **and** `input[0] != 0x78`.
`flate_truncated` starts with `0x78`, so no fallback fires. But a *future* headerless
truncated fixture would take the fallback path, whose partial semantics differ slightly.
*Test behavior:* keep the assertion filter-agnostic and treat `Ok(partial)` uniformly;
do not assume the zlib-header path.

### Category D — loop / aggregation

**EC9 — Missing-fixture guards must precede the assertion.** ⟦E4⟧ The `.bin`/`.expected`
existence checks (268–277) and read guards (282–298) must run first.
*Test behavior:* place the assertion after line 298; it must never execute on a
missing/unreadable fixture.

**EC10 — Honor the aggregated-loop failure convention.** The test collects
`failures: Vec<String>` and panics once at the end (349–356). A bare mid-loop `assert!`
aborts and hides every later fixture's result.
*Test behavior:* push a formatted string to `failures`; do not use a loop-aborting
`assert!` for the expected outcome.

**EC11 — `total`/`passed` accounting.** `total` increments at the top (263); `passed` at
the bottom (345). An error-expecting fixture that "got its expected error" must still
reach `passed += 1`, else the summary `{passed}/{total}` silently undercounts (a green
run would report e.g. `15/16`).
*Test behavior:* the error-expecting branch must fall through to (or explicitly do)
`passed += 1; continue;` — not bare `continue;`.

### Category E — what the assertion must actually catch (failure modes)

Given EC1–EC3, the only regressions the assertion can meaningfully guard are
**inversions** of the expected `Ok(partial)` outcome:

**EC12 — Regression: decoder returns `Err` for truncated input (INV-8 violation).** If a
future change makes `FlateDecoder` return `Err(FilterError)` for truncation, the existing
`Err` arm reports it as a generic `"Decode error: …"` failure — but the message would not
say "lost partial-recovery."
*Test behavior:* when `expects_decode_error` is set and decode returns `Err`, push a
failure message that explicitly calls out the INV-8 regression ("fixture expects soft
partial-recovery, but decode returned hard Err"), distinguishing it from a content
mismatch.

**EC13 — Unfixable gap: "decoder silently completed" is indistinguishable from "correct
partial recovery."** There is no length hint or checksum on this path (EC3) and no
collected diag (EC1). The assertion **cannot** tell "decoder recovered the right partial
bytes" from "decoder swallowed a real corruption and returned clean output."
*Test behavior:* document the gap explicitly in the test/commit; note that a true
end-to-end `STREAM_DECODE_ERROR` assertion belongs in the **integration layer** (where
`DiagCode`s are collected) — but [[bf-junlj]] already established the full-extraction path
emits nothing for `truncated-flate.pdf` (`pages == []`, `diagnostics == []`), so that
integration assertion is blocked on the truncated page becoming enumerable and is out of
scope for this unit test.

**EC14 — The assertion must be falsifiable (avoid a tautology).** Because of EC1, a
"did we observe the error?" runtime assertion is unfalsifiable on the happy path — it
could only ever fire as a failure and would "pass" vacuously otherwise. The selector's
polarity must be made explicit in a comment, and ideally a negative case (a fixture
declared `expected_diags: vec![StreamDecodeError]` whose input the decoder fully decodes)
would trip the failure. A full negative fixture is out of scope for the one-line change,
but the implementer should at minimum leave a `// contract: this fixture is *declared* to
expect a decode error; the diag itself is not observable on this path (see bf-60qj2 EC1)`
comment so future readers do not mistake the assertion for a runtime check.

---

## 2. Test-behavior matrix (acceptance: "test behavior for each edge case specified")

| Case | Fixture / input state | Decode outcome | Verdict | Why |
|------|-----------------------|----------------|---------|-----|
| **Happy (today, post-fix)** | `expected_diags=[StreamDecodeError]`, 26-byte truncated `.bin`, 0-byte `.expected` | `Ok(~13 partial bytes)` | **PASS** (reaches `passed += 1`) | Soft-recovery honored INV-8; selector declares the error; no hard `Err`. (EC2, EC11) |
| Pre-fix (current `vec![]`) | `expected_diags=[]`, same input | `Ok(partial)` | **vacuous PASS** (false-pass) | Selector false ⇒ assertion no-op; byte-compare vacuous (EC3, EC4). This is the bug being fixed. |
| INV-8 regression | selector set; decoder changed to return `Err` | `Err(FilterError)` | **FAIL** with INV-8-specific message | Distinguish from content mismatch (EC12) |
| Missing `.bin` | file absent | n/a | **FAIL** "fixture file not found" (pre-assertion guard) | Never reaches assertion (EC9) |
| Missing `.expected` | file absent | n/a | **FAIL** "expected file not found" (pre-assertion guard) | Never reaches assertion (EC9) |
| Empty partial output | truncation before first flush | `Ok(Vec::new())` | **PASS** | No `decoded.len()` assumption (EC6) |
| Bomb fixture (`flate_bomb_3gb`) | `expected_diags=[StreamBomb]` | `Ok(partial, bomb-truncated)` | unaffected | Selector is per-fixture; `StreamDecodeError` not in its diags (EC5) |
| Headerless truncated (future) | `input[0] != 0x78` | `Ok(partial)` via Deflate fallback | **PASS** | Treat `Ok(partial)` uniformly (EC8) |

---

## 3. What a robust assertion can and cannot do

**Can:**
- Verify the fixture is **declared** to expect a decode error (the one-line `expected_diags`
  change, EC4) — i.e. the contract is in place.
- Verify the decode did **not** hard-fail: it returned `Ok(_)` (partial recovery, INV-8),
  not `Err(FilterError)`.
- Produce a clear, attributed failure if a future change flips truncation to `Err` (EC12).
- Reach `passed += 1` so the summary stays honest (EC11).

**Cannot (on this path — EC1, EC13):**
- Observe that a `STREAM_DECODE_ERROR` `DiagCode` was *emitted*. None is collected here.
- Distinguish "correct partial recovery" from "decoder silently completed."
- Byte-verify the partial output (EC7) or rely on byte-diff (EC3).

The honest framing for the implementer and for the commit message: **this is a
contract/selector assertion over fixture metadata plus a "did not hard-fail" guard — not
a runtime observation that the diagnostic fired.** The real diagnostic-emission test
lives upstream in the integration layer once the truncated page is enumerable.

---

## 4. Robust assertion strategy (recommendation to bf-4bx00)

Given §0–§3, place on the **`Ok` path** (not the `Err` arm), after the byte-compare
block and before `passed += 1`:

```rust
// After line 308 (decode Ok) — evaluate the selector HERE, on the Ok path.
let expects_decode_error =
    fixture.expected_diags.contains(&DiagCode::StreamDecodeError);

if expects_decode_error {
    // Contract assertion (see notes/bf-60qj2.md EC1/EC13): the STREAM_DECODE_ERROR
    // diagnostic is NOT collected on this low-level StreamDecoder path — soft errors
    // return Ok(partial) per INV-8 (stream.rs:542-549). So we assert the *contract*:
    // the fixture is declared to expect a decode error and decode did not hard-fail
    // (we are in the Ok arm). A future change that flips truncation to Err is caught
    // in the Err arm below with an INV-8-regression message.
    eprintln!(
        "{}: decode-error contract honored (Ok partial, {} bytes) — \
         STREAM_DECODE_ERROR declared but not collected on this path",
        fixture.name,
        decoded.len()
    );
    // (fall through to passed += 1 — EC11)
}
```

…and in the (today-unreachable, EC2) `Err` arm, add the INV-8-regression failure:

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

Plus the hard prerequisite (EC4): the one-line `expected_diags` change at line 62, in the
same commit, plus extending the loop to actually read `expected_diags` (it currently
doesn't — [[bf-junlj]] blocker #1).

This satisfies all four acceptance criteria of this bead:
- **Robust against EC2** (lives on the `Ok` path, not the dead `Err` arm).
- **Robust against EC12** (a future Err regression gets a distinct, attributable message).
- **Honest about EC1/EC13** (comments name the gap; does not fake a runtime observation).
- **Loop-hygiene-correct** (EC10 push-to-`failures`, EC11 reaches `passed += 1`, EC9
  after the missing-file guards).

---

## 5. Acceptance criteria (this bead)

- [x] **Edge case list documented** — taxonomy EC1–EC14 across five categories (§1),
      incorporating and extending [[bf-348zd]] E1–E5.
- [x] **Robust assertion strategy defined** — §4: selector/contract assertion on the `Ok`
      path + INV-8-regression guard on the `Err` path, framed honestly against EC1/EC13.
- [x] **Test behavior for each edge case specified** — §2 matrix: case → decode outcome →
      verdict → rationale.
- [x] **Assertion handles edge cases appropriately** — §3 enumerates can/cannot; §4 code
      references each EC it defends against.

## 6. Handoff deltas vs [[bf-348zd]] (what changed by reading the source)

1. **E3 RESOLVED:** outcome is `Ok(partial)`, **not** `Err` (§0). The implementer does
   *not* need to "run the fixture first to resolve Err vs Ok" as [[bf-348zd]] §6.1
   suggested — the source is unambiguous (`decode_impl:542–549`). Optional scoped
   confirmation only; do **not** run the full `test_all_stream_decoder_fixtures` (it
   includes the 2 GB `flate_bomb_3gb` fixture — slow and disk-heavy; cf. `~/CLAUDE.md`
   disk rules).
2. **Strategy pivots to the `Ok` path.** [[bf-348zd]] §2c foregrounded the `Err`-arm
   polarity flip; that arm is dead here (EC2). The selector logic moves to the `Ok` arm.
3. **The "no observable signal" gap (EC1/EC13) is now the central design constraint** and
   is documented as an explicit limitation, with the integration-layer test flagged as
   the only place a true emission assertion can live.

## 7. References

- Parent: [[bf-4g6dj]] (scaffold/extraction-result examination)
- Requirements (dependency): [[bf-348zd]] (assertion requirements; E1–E5)
- Location: [[bf-junlj]] (assertion location; naming correction; dead-`expected_diags` blocker)
- Implementer: bf-4bx00, bf-2897m (failure messages + compile)
- Code: `crates/pdftract-core/src/parser/stream.rs:42–99, 458–571`;
  `crates/pdftract-core/tests/stream_decoder_fixtures.rs:18–25, 220–251, 262–360`;
  `crates/pdftract-core/src/diagnostics.rs:465, 1278`
- Fixture: `tests/stream_decoder/fixtures/flate_truncated.{bin(26B),expected(0B),meta}`
