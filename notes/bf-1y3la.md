# bf-1y3la — Add unmapped_glyph_names field to generator config

**Outcome:** Already satisfied at HEAD; re-verified and evidence-closed 2026-10-17.
No code change was needed — the implementation landed on `origin/main` in an earlier
attempt, and this dispatch (an auto-split order triggered by `failure-count:7`) was
declined because there is no remaining work to decompose.

## Why the split was declined

The auto-split template fired on `failure-count:7`, but the bead's own event history
shows one real attempt (2026-09-18), which died on an infra `api_error 499` — not a
work failure. `failure-count` on this workspace accumulates from reason-less reopens
and quarantine cycling. The implementation has existed on `origin/main` since before
the 2026-10-06 verification recorded in the bead notes, an operator session on
2026-10-17T09:38Z released the ghost claim and directed: *"verify them at HEAD and
close with evidence rather than redoing the work."* Splitting satisfied work into
3–5 children would manufacture already-satisfied beads (treadmill fuel), which the
dispatch's own outcome rules forbid. No `parent-bf-1y3la` children exist
(`bead list --json --limit 999999 | grep -c parent-bf-1y3la` → 0).

## Artifacts at HEAD 4b5752f3

| Acceptance criterion | Evidence |
|---|---|
| Config struct has the `unmapped_glyph_names` field | `crates/pdftract-core/build.rs:22-31` — `struct UnmappedGlyphNamesConfig` with `unmapped_glyph_names: Vec<String>` |
| Field can be parsed from config files | `crates/pdftract-core/build.rs:1047-1048` parses `build/unmapped-glyph-names.json` via `serde_json::from_str` into the struct; the JSON (v1.1, 14 glyph names) carries the field |
| Default value handling (empty list) | `#[serde(default)]` on the field (`build.rs:30`); covered by `test_unmapped_glyph_names_defaults_to_empty` and `test_unmapped_glyph_names_minimal_config` |
| Code compiles without errors | `cargo check -p pdftract-core` exit 0 at HEAD |

## Verification (clean extraction of HEAD, 2026-10-17)

Run in `/var/tmp/pdftract-bf1y3la.M3xi2U`, a `git archive HEAD` extraction of
`4b5752f3` (not the shared dirty working tree), removed after success:

```
$ cargo test -p pdftract-core --test unmapped_glyph_names_config
  test result: ok. 4 passed; 0 failed; 0 ignored   (exit 0)
  — test_unmapped_glyph_names_defaults_to_empty
  — test_unmapped_glyph_names_specified
  — test_unmapped_glyph_names_empty_array
  — test_unmapped_glyph_names_minimal_config
$ cargo check -p pdftract-core
  Finished `dev` profile (exit 0)
```

Both commands exited 0. All four acceptance criteria: **PASS**. No WARN/FAIL items.

The three task-owned paths are clean in the shared working tree (no uncommitted
drift against HEAD); the shared tree's ~25 unrelated modified files from other
workers were left untouched, and nothing was staged beyond this note.
