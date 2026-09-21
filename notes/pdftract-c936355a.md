# Verification note — pdftract-c936355a

**Bead:** pdftract-c936355a — "Verify the forward_scan_xref chunked-path termination fix at HEAD"
(parent pdftract-c7c43f45; MCP dogfood pilot F1)
**Date:** 2026-09-21
**HEAD verified:** `ea0ed51c7d7b7a1254c4b9a81704466d1d642b71` (origin/main == HEAD at dispatch)
**Disposition:** no code change needed — both terminating guards and the regression test are
already at HEAD (landed in `3aa2d58a` under bead pdftract-c1fceb36, an ancestor of HEAD).
This dispatch's deliverable is the recorded passing run.

## 1. Guards confirmed by reading the chunked branch

`crates/pdftract-core/src/parser/xref.rs`, function `forward_scan_xref` (starts at line 1188).
The chunked path is only reachable when `source_len > SMALL_FILE_THRESHOLD` (1 MiB): lines
1225–1232 return `forward_scan_memory(...)` for everything at or below the threshold, so the
≤1 MiB path is untouched by the chunked loop.

The advance/slide-back block (lines 1293–1309), with the in-file comment citing this parent
bead and `docs/notes/mcp-dogfood-pilot.md`:

- **High-water mark** — `xref.rs:1309`:
  `pos = chunk_end.saturating_sub(CHUNK_OVERLAP).max(pos + 1);`
  the slide-back never lands at or before `pos`, so every iteration advances even on short reads.
- **EOF break** — `xref.rs:1305–1308`, executed *before* the slide-back:
  `let chunk_end = pos + chunk.len() as u64; if chunk_end >= source_len { break; }`
  once a chunk reaches end-of-source there is no following chunk to overlap with; without this
  the final partial chunk cycled `source_len-3 → source_len → source_len-3` forever.

Comment block `xref.rs:1293–1304` documents both guards and cites parent bead
pdftract-c7c43f45 / pilot F1. Regression test: `test_forward_scan_large_file_terminates` at
`xref.rs:3387–3446` (2 MiB source, 30 s `recv_timeout`, headers straddling both chunk
boundaries, a header fully inside the final partial chunk — exactly where the old code hung —
plus a trailer for the post-loop trailer-scan success path). Sibling test
`test_forward_scan_trailer_scan_terminates` (`xref.rs:3449`) covers the same slide-back defect
family in `forward_scan_trailer`.

## 2. Recorded passing run at HEAD

Method: clean extraction of HEAD via `git archive HEAD | tar -x` into
`/var/tmp/pdftract-verify-c936355a` (no `.git`, so the cargo wrapper runs the local
cgroup-limited path), private `CARGO_TARGET_DIR=/var/tmp/pv-c936355a-target`. The test was
**not** run from the shared working tree, which carries ~61 dirty files from other workers.

Exact command from the bead description, one bare test name:

    $ cd /var/tmp/pdftract-verify-c936355a
    $ export CARGO_TARGET_DIR=/var/tmp/pv-c936355a-target
    $ timeout --kill-after=30s 600s cargo test -p pdftract-core --lib test_forward_scan_large_file_terminates

Output tail:

    running 1 test
    test parser::xref::tests::test_forward_scan_large_file_terminates ... ok

    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3651 filtered out; finished in 0.00s

    EVIDENCE_RUN_EXIT=0

Wall clock: seconds (test itself `finished in 0.00s`), nowhere near the 600 s timeout — no
exit 124, no hang, no retries spawned.

## 3. Supplementary checks in the same clean extraction

Sibling defect-family test (same run protocol, separate invocation):

    $ timeout --kill-after=30s 600s cargo test -p pdftract-core --lib test_forward_scan_trailer_scan_terminates
    test parser::xref::tests::test_forward_scan_trailer_scan_terminates ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3651 filtered out; finished in 0.01s
    TRAILER_TEST_EXIT=0

Compile-level definition-of-done scoped to the package the fix lives in:

    $ cargo build -p pdftract-core --all-targets
    CORE_ALL_TARGETS_BUILD_EXIT=0

## 4. Explicitly not claimed

A full `cargo test` run is **not** claimed anywhere: the tree carries ~300 pre-existing test
failures at HEAD (documented in the plan TH notes and repo CLAUDE.md), and the bead
description forbids claiming one. Workspace-wide `--all-targets` was likewise not run (xtask
fixture-generator bins have pre-existing breakage outside this bead's scope); the compile DoD
above is scoped to `pdftract-core`.

## 5. Acceptance criteria

- PASS — recorded passing run of `test_forward_scan_large_file_terminates` at HEAD
  `ea0ed51c7d7b7a1254c4b9a81704466d1d642b71` (§2), in a clean extraction of that exact tree.
- PASS — chunked branch provably terminates for sources above `SMALL_FILE_THRESHOLD`: both
  guards read and confirmed (§1); the regression test drives a 2 MiB source whose final
  partial chunk contains a header and completes in 0.00 s; ≤1 MiB path returns via
  `forward_scan_memory` before the chunked loop (lines 1227–1232) and is untouched.
- PASS — this note written; bead notes field updated with the summary.

## Cleanup

The extraction and private target dir under `/var/tmp` are self-owned by this dispatch and
removed after verification (root mount had 42 G free; nothing written to `~/scratch`/`/data`).
