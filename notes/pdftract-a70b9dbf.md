# Verification note — pdftract-a70b9dbf

**Bead:** pdftract-a70b9dbf — "Extend forward_scan_xref property coverage past the 1 MiB
chunked threshold" (parent pdftract-c7c43f45; split child)
**Date:** 2026-09-21
**Commit:** `384d9be408abcae145c1f06b8901646f0c47e644` (test-only, +103 lines in
`crates/pdftract-core/src/parser/xref.rs`)
**Disposition:** implemented as a sibling property module — see §1.

## 1. What was added

New module `proptest_large_file_tests` nested inside `mod tests::proptest_tests` in
`crates/pdftract-core/src/parser/xref.rs` (immediately after the existing small-input
`proptest!` block), containing one property, `proptest_forward_scan_chunked_path_terminates`,
configured with `ProptestConfig::with_cases(8)` so the large-input cases stay few and the
existing small-input properties keep their default fast configuration.

Strategy (module consts mirroring the ones inside `forward_scan_xref`):

- `total_len in (SMALL_FILE_THRESHOLD + 1)..=MAX_LEN` where
  `SMALL_FILE_THRESHOLD = 1 MiB` and `MAX_LEN = SMALL_FILE_THRESHOLD + 6 * CHUNK = 2.5 MiB`
  (`CHUNK = 256 KiB`) — threshold plus a couple of CHUNK multiples plus slack, per the bead's
  sizing guidance;
- `obj_num in 1..=99_999`, `gen_num in 0..=65_535` — random header contents;
- `space_slack in 1..=3` — which of the last 3 bytes of the straddled chunk holds the space
  of `" obj"`; consecutive boundaries cycle `slack = (slack % 3) + 1`, so all three positions
  of the `CHUNK_OVERLAP` window get exercised across the boundaries of a single case;
- `with_trailer in any::<bool>()` — optionally injects `"\ntrailer<<"` near EOF so the
  trailer scan's success path also runs on large sources.

Input construction mirrors `test_forward_scan_large_file_terminates`: `\n`-filled buffer with
a `%PDF-1.4` magic; one `"N G obj\n"` header per chunk boundary placed so its `" obj"` space
sits at `boundary - slack` and `"obj"` completes inside the next chunk (a 32-byte loop guard
keeps these clear of the tail region); plus one header ending exactly at EOF, fully inside
the final partial chunk — the placement where the pre-fix loop cycled forever.

Assertions:

- **In-body size proof** — `prop_assert!(total_len > SMALL_FILE_THRESHOLD)`: every generated
  case demonstrably exceeds 1 MiB, so the chunked branch (not the read-it-all memory branch,
  which returns at `source_len <= SMALL_FILE_THRESHOLD`, xref.rs:1227) is the code under
  test. This satisfies acceptance criterion 2 both by strategy construction and by
  in-test assertion.
- **Termination bound** — the scan runs on a spawned thread with a 30 s
  `mpsc::recv_timeout`, panicking with the offending `total_len` on timeout, so a regression
  fails the property instead of hanging the suite (same pattern as the deterministic test).
- **Completion** — `XrefRepaired` diagnostic present, which is emitted only after the entry
  loop and the trailer scan have both run to completion (xref.rs:1322).

## 2. Recorded passing run (clean extraction of the commit)

Method: `git archive HEAD | tar -x -C /var/tmp/a70b9dbf-verify-p3QRZc` (no `.git`, so the
cargo wrapper runs the local cgroup-limited path), private
`CARGO_TARGET_DIR=/build/a70b9dbf-target`. Not run from the shared working tree — it carries
other workers' stranded edits (19 pre-existing E0061/E0308 errors, all in
`crates/pdftract-core/src/parser/pages.rs`; none in `xref.rs`, which was clean at HEAD before
this bead's edit).

Acceptance-criterion command (8 default cases):

    $ cd /var/tmp/a70b9dbf-verify-p3QRZc
    $ export CARGO_TARGET_DIR=/build/a70b9dbf-target
    $ timeout --kill-after=30s 600s cargo test -p pdftract-core --lib proptest_forward_scan_chunked_path_terminates

    running 1 test
    test parser::xref::tests::proptest_tests::proptest_large_file_tests::proptest_forward_scan_chunked_path_terminates ... ok

    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3652 filtered out; finished in 0.03s
    EXIT=0

Scaling evidence that real multi-MB work happens per case (100 cases via env override —
linear in case count, ~3 ms/case, allocation + memchr pass over 1–2.5 MiB each):

    $ timeout --kill-after=30s 600s env PROPTEST_CASES=100 cargo test -p pdftract-core --lib proptest_forward_scan_chunked_path_terminates
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3652 filtered out; finished in 0.30s
    EXIT=0

No exit 124, no hang, no overlapping retries spawned.

## 3. Supplementary checks in the same clean extraction

Sibling tests in the same defect family (multi-filter form; small-input properties included
to show they stay fast alongside the new one):

    $ timeout --kill-after=30s 600s cargo test -p pdftract-core --lib -- test_forward_scan_large_file_terminates test_forward_scan_trailer_scan_terminates proptest_forward_scan_no_panic proptest_forward_scan_linearized_no_panic

    running 4 tests
    test parser::xref::tests::proptest_tests::proptest_forward_scan_linearized_no_panic ... ok
    test parser::xref::tests::test_forward_scan_large_file_terminates ... ok
    test parser::xref::tests::proptest_tests::proptest_forward_scan_no_panic ... ok
    test parser::xref::tests::test_forward_scan_trailer_scan_terminates ... ok

    test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 3649 filtered out; finished in 0.02s
    EXIT=0

Compile-level definition of done scoped to the package the change lives in (there is no
`scripts/definition-of-done.sh` in this repo):

    $ cargo check -p pdftract-core --all-targets
    CHECK_EXIT=0

## 4. Explicitly not claimed

A full `cargo test` run is **not** claimed: the tree carries ~300 pre-existing test failures
at HEAD (plan TH notes / repo CLAUDE.md), and the bead's criterion asks for precisely
filtered runs naming only the tests actually run (§2–§3). Workspace-wide checks were not run
(xtask fixture-generator bins have pre-existing breakage outside this bead's scope). Entry
recovery counts are not asserted — consistent with the deterministic test's note that entry
recovery is tracked separately (F2 / pdftract-257d92c3); this property asserts termination,
no-panic, and full-path completion only.

## 5. Acceptance criteria

- **PASS** — the new property passes under
  `timeout --kill-after=30s 600s cargo test -p pdftract-core --lib proptest_forward_scan_chunked_path_terminates`
  in a clean extraction of commit `384d9be4` (§2), exit 0.
- **PASS** — generated cases demonstrably exceed 1 MiB: strategy range is
  `(SMALL_FILE_THRESHOLD + 1)..=2.5 MiB` by construction, asserted in the test body via
  `prop_assert!(total_len > SMALL_FILE_THRESHOLD)`; per-case wall time (~3 ms, §2 scaling
  run) is consistent with scanning multi-MB sources.
- **PASS** — commit `384d9be4` touches only test code in `crates/pdftract-core` (+103 lines,
  `xref.rs` only) with the bead ID in the message; this note records the run.
