# pdftract-b5cb36ca — default_rust gate verification at HEAD

**Result: PASS — gate already green, no fixes needed (empty code-diff).**

## What was run

Faithful replica of the `default_rust` gate method, on a fresh `git archive HEAD`
extraction (not the working tree — this checkout carries ~110 in-flight edits from
other workers):

```bash
EX=$(mktemp -d /tmp/gate-replica-b5cb36ca.XXXXXX)
git -C /home/coding/pdftract archive HEAD | tar -x -C "$EX"
cd "$EX"
timeout --kill-after=30s 600s cargo check --all-targets --quiet
```

## Evidence

| Item | Value |
|---|---|
| HEAD sha verified | `e20ecf8cd5c4ecd849de4653172dc904219f0092` |
| Command exit code | **0** |
| Started / ended (UTC) | 2026-09-15T20:11:13Z → 2026-09-15T20:11:40Z (27s, warm shared `/build/target-workers`) |
| Timeout fired | No (exit 124 would indicate a hang; none) |
| Target-dir lock contention | None (`Blocking waiting for file lock`: 0 occurrences — single run, no retry needed) |
| Errors (`^error` / `error[...]`) | **0** |
| Warnings | 495 (unused imports, dead code, unused variables — see below) |

Gate method note: the build reused the global cargo `target-dir`
(`/build/target-workers` per `~/.cargo/config.toml`), which is exactly the warm-cache
behavior the gate prescribes. 4024 lines of checker output confirm the workspace was
actually checked, not skipped.

## Warnings are non-blocking

No `deny(warnings)` in this workspace; a warning cannot fail the gate — only a hard
error can, and there were zero. The known live noise cited in the bead was observed
among the 495 (e.g. unused `intern`/`ObjRef` imports in `crates/pdftract-core/src/document.rs`
tests, dead-code warnings in `pdftract-cli` `grep/worker.rs`, `hash.rs`, `mcp/framing/mod.rs`,
`mcp/root.rs`) plus additional pre-existing unused-import/dead-code warnings in examples
and test helpers (`crates/pdftract-core/examples/ocr.rs`, `tests/xref_helpers.rs`,
`tests/test_page_helper_error_handling.rs`). Per the bead's implementation guidance these
were left alone: "If the HEAD extraction exits 0, this child closes on that evidence —
do not invent work."

## Fixes made

**None.** The gate was already green at HEAD. The parent's attempt-3 failure
(pdftract-a126b61f, 2026-09-15T09:34Z) predates `63da9e7a` ("land stranded diagnostics
catalog/round-trip work…"), which evidently resolved whatever was red then. No commit
was manufactured for this child; the only artifact is this note.

## References

- Parent: pdftract-a126b61f (gate foundation; chain: this bead → child 2 serialization → child 3 legacy compat → child 4 contract docs)
- docs/plan/plan.md line 145 (error model: recoverable parse errors, no `panic!` in library code)
- docs/plan/plan.md line 933 (INV-8: no `panic!` reaches the `pdftract-core` public boundary)
- Extraction dir cleaned up after the run (owned-and-removed, per workspace file-organization rules)
