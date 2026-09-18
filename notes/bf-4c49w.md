# bf-4c49w — Build profile_yaml fuzz harness successfully

**Date:** 2026-09-18 · **HEAD at close:** `cb0fe966` · **Verdict: PASS — auto-split declined, bead satisfied and closed with evidence.**

## What the dispatcher claimed vs what was true

The dispatch ordered a 3–5-way auto-split on `failure-count:6`. Forensic history
(`.beads/checkpoint/forensic.jsonl`, filtered on `issue_id == "bf-4c49w"`, 12 events)
contains **zero failed attempts**: only `updated` ×5, one `released` (never-worked
prior assignee), `dependency_removed` ×3, and the quarantine label set. The
failure counter was inflated by precondition releases and dependency removals,
not by builds. The bead had also never been closed, with a stranded uncommitted
artifact in the tree — the recorded "land it instead of splitting" shape.

## Verification (all PASS, at HEAD + stranded blobs in a clean extraction)

Environment note: builds in the **shared checkout** currently fail for every
target with 18 pre-existing `parser/pages.rs` union errors (stranded edits,
documented 2026-09-17) — unrelated to this harness, unfixable by any child of
this bead. Verified in `/var/tmp/bf-4c49w-head-verify`:
`git archive HEAD` + exactly the two stranded blobs
(`fuzz/fuzz_targets/profile_yaml.rs`, `fuzz/Cargo.lock`) = exactly the landed
commit `cb0fe966`.

| Acceptance criterion | Result | Evidence |
|---|---|---|
| `cargo fuzz build profile_yaml` completes without compilation errors | PASS | `cargo +nightly fuzz build profile_yaml` → exit 0, `Finished release profile ... in 1m 45s` |
| No warnings about unresolved dependencies | PASS | 0 dep errors; warnings were ordinary code warnings (85, incl. one pre-existing `unused_mut` in `match_eval.rs:278`) |
| Build produces the expected binary artifact | PASS | `/build/target-workers/x86_64-unknown-linux-gnu/release/profile_yaml` (63,685,384 B) — global `target-dir = /build/target-workers` |
| Reasonable build time | PASS | 1m 45s from scratch; no hang (timeout guard 580s not approached) |
| (bonus) artifact executes | PASS | `-runs=1` → `Done 2 runs`, `cov: 358`, exit 0, with `LD_LIBRARY_PATH` = nix `gcc-*-lib` |

Toolchain: `cargo fuzz 0.13.2`, nightly required (`-Zsanitizer` is nightly-only;
stable default 1.97.1 fails with "the option `Z` is only accepted on the nightly
compiler" — use `cargo +nightly fuzz build`).

## Artifacts

- Commit `cb0fe966` — lands the stranded harness hardening (forbidden-key gate +
  Profile validator coverage, regex-DoS out-of-scope rationale, Cargo.lock refresh).
- Parent `bf-4c49w` closed with evidence; **no split children created** (see below).

## Why the auto-split was declined

1. The failure premise is false: zero failed attempts exist; the only reproducible
   failure (shared-tree build) is caused by stranded `parser/pages.rs` edits outside
   this bead's scope and would fail identically for any child.
2. The deliverable is a single verified build command — not decomposable into 3–5
   independently meaningful children; a split would fabricate dispatch churn on a
   satisfied bead (the recorded auto-split treadmill).
3. The stranded artifact was landed instead, per the terminal-action guidance:
   re-derive at HEAD, land, evidence-close.
