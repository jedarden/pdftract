# pdftract-8c4bed00 — split re-check (2026-09-16)

**Dispatch:** `claude-code-glm-5.3-glm53-adc` (auto-split, attempt 5)
**Verdict:** duplicate split declined — the requested decomposition already exists and is complete. One wiring repair applied.

## What the dispatch asked

Break the bead into 3–5 chained `split-child` beads, convert the parent to an
umbrella (`umbrella` + `auto-split-parent`), parent depends on the last child.

## What was already in place (from the 2026-09-15 split)

Verified at HEAD `81c7541a` via `bead show <id> --json`:

| # | Child | Scope | Status |
|---|-------|-------|--------|
| 1 | `pdftract-d1c76b2a` | Fix default_rust gate failures so verification gates can pass | closed |
| 2 | `pdftract-18954693` | Verify metadata diagnostics string↔structured mirror tests at HEAD | closed |
| 3 | `pdftract-d4abc102` | Verify empty-omission serialization tests at HEAD | closed |
| 4 | `pdftract-c8738574` | Verify full-JSON top-level errors-array propagation tests at HEAD | closed |
| 5 | `pdftract-f910c08c` | Verify NDJSON footer errors propagation + end-to-end mirror at HEAD | closed |

All five carry `split-child` + `parent-pdftract-8c4bed00`. The parent already
had `umbrella` + `auto-split-parent` and already depended on the last child
(`pdftract-f910c08c`, kind `blocks`).

## Repair applied this dispatch

`bead dep add pdftract-c8738574 pdftract-d4abc102 --kind blocks` — child 4 had
no dependency on child 3, breaking the sequential chain the split spec
requires (`→ exit 0, "Added dependency"`). Both endpoints are closed, so this
has no readiness effect; it restores the graph invariant. Checkpoint
auto-published (generation `gen-e034a72aa2e3aad89ed756d52a8cb655` at the time
of the edge add).

Also appended a split re-check record to the parent's `notes` field
(`bead update --fencing-token 5`, exit 0) so the decline decision lives where
the next dispatcher/worker looks.

## Why no new children

The bead's own coverage rule: an "investigate" and an "implement" bead for the
same defect is fine; two independent implement/verify sets for the same X is
not. A second split would have created exactly that, against children that are
already closed with evidence.

## Why the parent is not closed here

The split dispatch explicitly forbids closing. The terminal action for this
umbrella remains an evidence close by a work dispatch: attempt 4 was
`verified_success`, and the re-issue-7 close of `pdftract-c8738574` (commit
`5d4d9b02`) recorded a fresh 15/15 surface-test run at `338d4709`. The
enforcement work itself landed as `23cf6bb4`
(`crates/pdftract-core/tests/diagnostics_surface_mirror.rs`, 8 tests, plus the
`output::ndjson::footer_errors` refactor) with the two sibling suites
(`diagnostics_serialization_format.rs`, `diagnostics_catalog_drift.rs`)
re-verified green at that HEAD.

## Command evidence

```
$ bead dep add pdftract-c8738574 pdftract-d4abc102 --kind blocks
Added dependency: pdftract-c8738574 blocked by pdftract-d4abc102   (exit 0)
$ bead update pdftract-8c4bed00 --notes "$(cat /tmp/parent_notes_old.md)" --fencing-token 5
pdftract-8c4bed00                                                   (exit 0)
```
