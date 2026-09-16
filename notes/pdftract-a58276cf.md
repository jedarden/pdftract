# pdftract-a58276cf — Verify legacy Vec<String> diagnostics byte-for-byte compatibility

Auto-split child 3/4 of pdftract-a126b61f. Date: 2026-09-16 (UTC).

## Outcome: PASS (with one regression found and fixed)

The compat adapter's own contract held at HEAD, but the **producers did not
match it**: a caller-visible byte regression was found and fixed. Per the bead's
FAIL criterion ("FAIL only if any caller-visible legacy string differs — fix
the regression, not the test"), the regression was repaired, not papered over.

## The regression

`fix(pdftract-c1fceb36)` commit `166b401f` (2026-09-15 19:43, "gate serde call
sites behind the serde capability feature") replaced the producer conversion

```rust
diagnostics: to_legacy_strings(&all_diagnostics)      // before: bare message
```

with

```rust
diagnostics: all_diagnostics.iter().map(ToString::to_string).collect(),
```

at all three extraction entry points, and dropped the `to_legacy_strings`
import. `ToString::to_string` resolves through `Diagnostic`'s `Display` impl
(`crates/pdftract-core/src/diagnostics.rs:2854`), which renders
`"{CODE}: {message} (byte offset N)? [{obj} {gen} R]?"` — so every caller of
`metadata.diagnostics` started receiving code-prefixed strings ~14 hours after
commit `56a4f6c7` (07:19) had wired the byte-compatible helper in, and ~2 years
of accumulated history (inline `d.message.as_ref().to_string()` pushes since
`51b7c81d` et al.) said the legacy surface is the bare message.

`166b401f`'s own commit message claims "default-feature builds are
behaviorally unchanged" — false for this surface. The switch was collateral of
mechanical serde gating, not a deliberate contract change. The adapter's
contract (and this bead's) is explicit: `to_legacy_string` returns `d.message`
verbatim, deliberately NOT the Display form; `diagnostics_compat` module docs
rule 1 pins byte-for-byte stability.

## Fix applied

`crates/pdftract-core/src/extract.rs` — restored the exact pre-regression
conversion (verified byte-identical to `166b401f^`) at all three producer
sites, plus the import:

| Entry point | Line (post-fix) | Change |
|---|---|---|
| `extract_pdf` | 1086 | `to_legacy_strings(&all_diagnostics_with_js)` |
| `extract_pdf_ndjson` | 2015 | `to_legacy_strings(&all_diagnostics)` |
| `extract_pdf_streaming` | 2332 | `to_legacy_strings(&all_diagnostics)` |

`crates/pdftract-core/src/diagnostics_compat.rs` — landed the golden test
`golden_legacy_bytes_match_inline_producer_expression` (found stranded
uncommitted in the working tree from a prior quarantined dispatch of this same
bead; it implements exactly the gap candidate this bead names). It pins
`to_legacy_strings` element-for-element against the inline
`d.message.as_ref().to_string()` shape across all 4 severities × 8
optional-field combinations (byte_offset/object_ref/page_index set and unset),
duplicate + reversed mixed order, and asserts no `byte offset`/`[` leakage
into the legacy bytes.

`diagnostics_detailed` (structured surface) is untouched — it never used
`ToString`, so JSON/NDJSON `errors` output is unaffected by this fix.

## Producer-wiring audit

- Pre-fix producers: exactly three sites (above), all `ToString::to_string`
  (Display). `grep "ToString::to_string" crates/pdftract-core/src/extract.rs`
  now returns zero hits.
- `crates/pdftract-core/src/output/json.rs:369` uses `diag.to_string()` but is
  inside `#[cfg(test)]` (module starts at line 265) — a hand-populated fixture,
  not a producer. Left alone.
- Downstream consumers of the legacy array pass it through unchanged:
  `extract.rs:1574` (`metadata_obj["diagnostics"] = json!(...)`),
  `pdftract-cli/src/serve.rs:589,663` (clone for SSE/HTTP framing). The only
  caller-visible byte change in this bead is the intended one: Display form →
  bare message, i.e. **restoration** of the long-standing legacy bytes.
- The hand-populated Display-pinning tests (`diagnostics_surface_mirror.rs`,
  `diagnostics_serialization_format.rs`) construct their fixtures with
  `typed.iter().map(ToString::to_string)` themselves and never read producer
  output, so restoring the producers does not affect them (15/15 green).

## Commands and results

Wrapped per test-hygiene policy (`timeout --kill-after=30s`). Scoped to the
diagnostics suites as the bead directs (the tree carries pre-existing
unrelated failures and extraction is broken tree-wide — 0/1377 fixtures — so a
full `--all-targets` PASS is not obtainable evidence here; a full run was
deliberately not used).

```
$ timeout --kill-after=30s 600s cargo test -p pdftract-core --lib diagnostics_compat
running 9 tests
... all 9 passed, 0 failed (includes the golden test)
$ timeout --kill-after=30s 600s cargo test -p pdftract-core \
    --test diagnostics_surface_mirror --test diagnostics_serialization_format
7 passed; 0 failed (serialization) / 8 passed; 0 failed (mirror)
$ timeout --kill-after=30s 300s cargo check -p pdftract-core --no-default-features
Finished (the wasm32/serde-gating edge 166b401f targeted still compiles)
```

Notes on individual results:
- `extraction_driven_end_to_end_mirror` reports "ok" but **soft-skips**: it
  early-returns with "SKIPPED (not passed)" when `extract_pdf` fails
  tree-wide. No test at HEAD exercises real producer output; the producer
  restoration is proven by construction (the helper is element-wise
  `message.as_ref().to_string()`, pinned by the golden test) plus the audit
  above.
- `rustfmt --check` on `extract.rs` reports pre-existing style-edition import
  ordering deviations across the whole import block (verified identical
  without this bead's edit via stash round-trip); not introduced here, left
  alone. `diagnostics_compat.rs` is fmt-clean.

## WARN (handoff, out of this bead's scope)

Two artifacts still encode the Display grammar as the legacy surface's
contract. They are only consistent with each other, not with the adapter or
the restored producer bytes:

1. `docs/errors-array-format.md` "String Format" section documents
   `{CODE}: {message} (byte offset N)? [obj gen R]?` and "The `CODE:` prefix
   is always present (diagnostics are never emitted as bare messages)". That
   statement was factually false for the producer surface from its original
   commit (`608fbcd8`, 2026-07-05) through `166b401f` (2026-09-15 19:43), and
   is false again after this restoration.
2. `diagnostics_surface_mirror.rs::assert_mirror` requires string entries to
   start with `"{code}: "`. It passes today only because its fixtures are
   hand-populated and the extraction-driven variant soft-skips. **When
   extraction recovers, this test will fail against the restored bare-message
   producers** and must be reconciled to the adapter contract
   (`diagnostics[i] == diagnostics_detailed[i].message`) — or the contract
   must be consciously flipped (which would break the adapter's tests and the
   long-standing caller bytes).

Flagged for child 4 of this split and/or the doc-reconciliation beads
(pdftract-456c2b02 / pdftract-3ebde655 lineage) rather than patched here, to
avoid churning another bead's just-landed artifacts from a compatibility bead.

## Re-verification at HEAD 98cf4de1 (attempt 3, 2026-09-16)

Attempt 2 landed `19318bf7` (producer restoration + golden test) but was killed
by the dispatch hard timeout before its evidence could be recorded durably; the
sections above are its account. This section re-derives the evidence at current
HEAD `98cf4de1` (attempt 3, this dispatch).

Setup: the run came from a private `git archive HEAD` extraction
(`/home/coding/scratch/a58276cf-verify`, removed after the run) with its own
`CARGO_TARGET_DIR`, so the shared checkout's in-flight edits from other workers
could not contaminate the verdict. Every stage wrapped with
`timeout --kill-after=30s`; **no 124 exits**. `19318bf7..HEAD` is an empty
code-diff for both touched files (`git diff 19318bf7..HEAD --stat` on
`extract.rs` + `diagnostics_compat.rs` prints nothing), so these results cover
the fix commit exactly as landed.

```
$ timeout --kill-after=30s 2700s cargo test -p pdftract-core --lib --no-run
    Finished `test` profile ... in 1m 15s          # 199 pre-existing warnings, unrelated
$ timeout --kill-after=30s 600s cargo test -p pdftract-core --lib diagnostics_compat
running 9 tests ... test result: ok. 9 passed; 0 failed; 0 ignored; 3600 filtered out
  (includes golden_legacy_bytes_match_inline_producer_expression)
$ timeout --kill-after=30s 600s cargo test -p pdftract-core \
    --test diagnostics_surface_mirror --test diagnostics_serialization_format
diagnostics_serialization_format: 7 passed; 0 failed
diagnostics_surface_mirror:       8 passed; 0 failed      # = 15/15, matches attempt 2's claim
```

Audit re-derivations at HEAD:

- Regression story cross-checked from history: `git show 166b401f^:…/extract.rs`
  has `diagnostics: to_legacy_strings(...)` (line 1073), `166b401f` replaced it
  with `.map(ToString::to_string)` (line 1085), HEAD restores
  `to_legacy_strings` (line 1090). `grep ToString::to_string extract.rs` → zero
  hits at HEAD.
- Producer-wiring audit extended: `crates/pdftract-cli/src/mcp/http.rs:277`
  builds a `Vec<String>` via `e.code.to_string()` — that is the MCP
  error-response *code* string for request logging, not the
  `metadata.diagnostics` legacy surface; no caller-visible legacy bytes flow
  through it. Remaining touchpoints are pass-throughs as recorded above
  (`extract.rs:1576` re-serializes the already-converted `Vec<String>`;
  `serve.rs:589,663` clone it).
- The WARN handoff stands unchanged at HEAD:
  `extraction_driven_end_to_end_mirror` still reports `ok` while soft-skipping
  (`finished in 0.00s` — early-return on tree-wide extract breakage), so the
  Display-grammar conflict in `docs/errors-array-format.md` +
  `assert_mirror` remains latent, owned by child 4 / the doc-reconciliation
  lineage.

## Artifacts

- Commits: see git log — `fix(pdftract-a58276cf): ...` (extract.rs producer
  restoration) and the golden-test landing in the same commit (`19318bf7`);
  pushed to `origin/main`. This note was left stranded untracked by the timed-out
  attempt 2 and is landed (with the re-verification section above) by attempt 3's
  `docs(pdftract-a58276cf)` commit.
- Files: `crates/pdftract-core/src/extract.rs`,
  `crates/pdftract-core/src/diagnostics_compat.rs`, this note.
