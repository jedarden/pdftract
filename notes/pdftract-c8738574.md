# Verification note — pdftract-c8738574

**Task:** Verify full-JSON top-level errors-array propagation tests pass at HEAD.
**Result: VERIFIED — both named tests PASS at HEAD `9e0a1a58`, no fix needed.**

## What was run

Working tree was dirty (83 files, other workers' in-flight edits), so the run
used the bead's prescribed recipe: a **git-archive-HEAD extraction** into
`~/scratch/pdftract-c8738574-verify` at HEAD `9e0a1a582fb31a9dac82337baf617e161cc4dbe1`,
with a warm **private `CARGO_TARGET_DIR`** hardlinked (`cp -al`) from the shared
`/build/target-workers`. No `.git` in the extraction → `cargo test` took the
local cgroup-limited fallback (no remote CI submission).

```
CARGO_TARGET_DIR=~/scratch/c8738574-target \
  timeout --kill-after=30s 900s cargo test -p pdftract-core \
  --test diagnostics_surface_mirror --test diagnostics_serialization_format \
  -- --test-threads=2
```

`pdftract-core` (lib) **was recompiled from the HEAD extraction**
(`Compiling pdftract-core v0.1.0 (/data/build/scratch/pdftract-c8738574-verify/crates/pdftract-core)`),
so the result genuinely reflects HEAD source, not a stale shared artifact.

## Cargo test summary (pasted per acceptance criterion 1)

```
     Running tests/diagnostics_serialization_format.rs
running 7 tests
test compact_json_omits_metadata_diagnostics_when_empty ... ok
test catalog_entries_populate_the_documented_envelope ... ok
test footer_frame_errors_are_always_present ... ok
test full_output_errors_array_is_always_present ... ok          <-- named test
test optional_fields_are_omitted_not_null ... ok
test page_frame_omits_errors_when_page_succeeded ... ok
test emitted_severities_match_documented_enum ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     Running tests/diagnostics_surface_mirror.rs
running 8 tests
test display_grammar_matches_documented_format ... ok
test compact_json_metadata_arrays_mirror_one_to_one ... ok
test display_prefix_carries_code_for_every_catalog_code ... ok
test empty_diagnostics_fields_are_omitted_from_serialized_output ... ok
test footer_errors_places_page_failures_before_structured_diagnostics ... ok
test full_json_errors_array_is_populated_from_diagnostics_detailed ... ok  <-- named test
test string_and_structured_arrays_mirror_one_to_one ... ok
test extraction_driven_end_to_end_mirror ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Exit code 0. Not a timeout kill (no 124).

## Acceptance criteria

### 1. Both named tests pass at a fresh commit — PASS

See summary above, run against HEAD `9e0a1a58` (the fresh commit; no new code
commit was required — see criterion 4 below).

### 2. Multi-diagnostic identity (length, order, per-index code) — PASS

The committed helper `sample_typed_diagnostics()`
(`crates/pdftract-core/tests/diagnostics_surface_mirror.rs:76`) builds **6
typed diagnostics** chosen to defeat order-accident passes: every severity,
document- and page-level entries, optional fields both present and absent, a
message containing `": "`, and two entries (indices 3/4) with **identical
messages but different codes** — so only true index pairing passes.
`full_json_errors_array_is_populated_from_diagnostics_detailed` pins:

- struct level: `assert_eq!(output.errors, *detailed)` — whole-array Vec
  equality (length + order + content),
- wire level: serialized `doc["errors"][i]["code"] == detailed[i].code` and
  `errors[i]` serializing identically to `diagnostics_detailed[i]`, per index.

Single-entry results cannot pass these assertions by accident of order.

### 3. Zero new compiler warnings — PASS

No source changes were made (verification-only bead), so no new warnings are
attributable to this bead. Measured baseline at HEAD, from the run log:
`pdftract-core (lib) generated 74 warnings` plus 1 build-script notice
(`glyph-shapes.json not found ... generating empty shape database`) — all
**pre-existing at HEAD**. **Zero warnings from the two test targets
themselves** (no `generated N warnings` summary lines after the lib block;
log verified clean after the lib compilation section).

## Emission-site confirmation

`crates/pdftract-core/src/output/json.rs` — `result_to_output` (~:85):

```rust
let errors: Vec<DiagnosticJson> = result.metadata.diagnostics_detailed.clone();
```

and `Output.errors` serializes **unconditionally** (no
`skip_serializing_if`), so it is always present, `[]` when clean — exactly
what `full_output_errors_array_is_always_present` pins. Matches
`docs/errors-array-format.md`.

## Notes / known-neighbor behavior

- `extraction_driven_end_to_end_mirror` reports `ok` via its designed
  self-skip; run with `--nocapture` it prints:
  `SKIPPED (not passed): ... extract_pdf currently fails tree-wide with:
  Document contains no pages` — the known tree-wide extraction breakage
  (memory: pdftract-extract-broken-everywhere). Its skip is by design and
  independent of the two named tests, which exercise `result_to_output`
  directly on hand-assembled results.
- Per the bead's "on failure fix production code; do not weaken the tests"
  rule: **nothing failed, nothing was changed.** The committed tests and the
  production emission site are in agreement at HEAD.

## Re-verification 3 (final) — HEAD `49951074` (claim epoch 3)

Attempts 1 (`9e0a1a58`) and 2 (`dc9f1c40b518`) both passed but the bead was
never closed; it was re-dispatched (quarantine expired 2026-09-16T02:50Z).
After attempt 2, `19318bf7` (pdftract-a58276cf) touched
`crates/pdftract-core/src/extract.rs` (+18) and `diagnostics_compat.rs` (+96) —
a non-empty code diff from the executed-verdict tip, so the PASS verdict was
stale and the tests were re-run fresh at HEAD `499510748c140a102c9a9cd70598776aa65276ee`.

Same recipe: git-archive-HEAD extraction in `~/scratch` (no `.git` → local
cgroup fallback), private `CARGO_TARGET_DIR` warmed via `cp -al` from
`/build/target-workers` (lib recompiled from HEAD source), `timeout
--kill-after=30s 900s` (no kill; exit 0), `--test-threads=2`.

**Result — identical to attempts 1 and 2:**
`diagnostics_serialization_format` 7/7 ok (`full_output_errors_array_is_always_present` ok),
`diagnostics_surface_mirror` 8/8 ok (`full_json_errors_array_is_populated_from_diagnostics_detailed` ok).
Warnings: `pdftract-core (lib) generated 74 warnings` — the same pre-existing
baseline count measured in attempts 1 and 2 — and zero warnings from the two
test targets. `19318bf7` deliberately changed only the legacy
`metadata.diagnostics` string conversion (`to_legacy_strings`); the structured
`diagnostics_detailed` surface and the `json.rs:86` errors clone are
untouched, and the fresh run confirms the contract holds at this tip. Scratch
extraction + private target dir removed after success; no orphaned processes
(`pgrep` clean).
