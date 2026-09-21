# pdftract-024b8a2a — forward_scan_xref caller audit (termination guards)

Bead: pdftract-024b8a2a · Parent: pdftract-c7c43f45 · HEAD audited: `d8995e41`
Guards introduced by: `3aa2d58a` (2026-09-15, ancestor of HEAD)

## What the guards are

In the >1 MiB chunked scan loop of `forward_scan_xref`
(crates/pdftract-core/src/parser/xref.rs, loop after the
`SMALL_FILE_THRESHOLD` early return):

1. **EOF break** — `if chunk_end >= source_len { break; }`. Without it the
   final partial chunk cycled `source_len-3 → source_len → source_len-3`
   forever (parent bead F1 wedge).
2. **High-water-mark slide-back** — `pos = chunk_end.saturating_sub(CHUNK_OVERLAP).max(pos + 1)`.

The pre-guard advance was `pos += to_read; pos = pos.saturating_sub(3)`. For
every non-degenerate read (chunk ≥ 4 bytes, not at EOF) the new expression
yields the identical `chunk_end - 3`. The only behavioral deltas are (a) break
at EOF instead of cycling and (b) `.max(pos + 1)` forces progress when a
source returns degenerate ≤ 3-byte reads (the old code could stall re-scanning
the same window there too). `CHUNK_OVERLAP = 3` is unchanged: `" obj"` is
4 bytes, so a space in the last 3 bytes of a chunk reappears at relative index
0–2 of the next chunk with `"obj"` contiguous after it and is matched there;
no pattern needs more than 3 bytes of lookback. The boundary
trailing-whitespace check goes through `check_trailing_whitespace` (absolute
1-byte read), independent of the overlap. The ≤ 1 MiB path
(`forward_scan_memory`) early-returns before the guarded loop and is
byte-identical across the guard boundary (sha256 `4520fd69…` at both
`3aa2d58a^` and HEAD).

## Caller table (re-derived at HEAD d8995e41 — this enumeration is the deliverable)

### Production callers

| file:line | entry point | size path | verdict |
|---|---|---|---|
| crates/pdftract-cli/src/mcp/tools/registry.rs:245 | `open_pdf` — used by tool dispatch (registry.rs:786, get_metadata et al.), `extract_metadata` (:837), `compute_fingerprint` (:950) | whole-file `MemorySource`, local → scan runs; > 1 MiB → chunked path | **Improved, no regression.** Old code never returned for > 1 MiB sources (parent F1); now returns. ≤ 1 MiB path untouched. Entries found are identical: scan-window sequence is bit-identical to pre-guard until the final chunk, where pre-guard never completed. |
| fuzz/fuzz_targets/xref.rs:19,22 | `pdf_xref` fuzz target (INV-8 no-panic) | arbitrary | **Invariant unchanged.** Guards add only `saturating_sub`/`max`/compare — no new panic surface. Strictly stronger: on > 1 MiB inputs the target previously hung rather than returning. |
| tests/proptest/xref.rs:60,74 | root-package proptests (no-panic properties) | small random buffers → memory path | **Unchanged** (memory path; guards unreachable). |

Not at HEAD: `crates/pdftract-cli/examples/diag_open_pdf.rs:20` (listed in the
split-time caller set) is an **untracked** working-tree file (`??` in git
status) — a stranded pilot diagnostic, not part of the committed surface and
not covered by any gate. If run locally it has the same call shape as
`open_pdf` and the same improved verdict.

### Test-side callers

| file:line | what it exercises | verdict |
|---|---|---|
| crates/pdftract-core/tests/remote_forward_scan_disable.rs:89,127,154,191 | remote/linearized disable (early return **before** the guarded loop) + local enable (≤ 1 MiB) | **Unchanged.** 4/4 pass with `--features remote`. |
| crates/pdftract-core/tests/remote_fetch_sequence.rs:620 (`test_forward_scan_disabled_remote`) | remote → early return before guards | **Unchanged.** Passes. |
| crates/pdftract-core/tests/xref_integration_test.rs:101 (helper `parse_fixture_xref`, test at :286) | test-side emulation of "strategy 4 last resort" fallback; fixture 564 B → memory path | **Unchanged.** See pre-existing failures below. |
| crates/pdftract-core/src/parser/xref.rs:3057,3065,3202 (proptests), :3238–:3524 (unit tests) | in-module `#[cfg(test)]`; :3144 chunked-path termination proptest (30 s bound, landed by pdftract-a70b9dbf/384d9be4); :3490 2 MiB termination test | **These assert exactly the guards' contract.** |

`forward_scan_trailer` has exactly one production caller
(xref.rs:1317, inside `forward_scan_xref`); its own slide-back guard (fixed by
the parent work, test at xref.rs:3552) folds into this audit — no separate
caller to check.

## Parent-claim verification (both refuted, in the safe direction)

- *"internal xref call sites in xref.rs (strategy 4 of xref loading)"* —
  **refuted as production callers.** `forward_scan_xref` *is* strategy 4
  (doc comment at xref.rs:1153, "strategy 4 - last resort"). No production
  code inside xref.rs calls it; only its own `#[cfg(test)]` module does. The
  only strategy-4 fallback wiring is test-side
  (xref_integration_test.rs:96-103).
- *"CLI hash/extract reach forward scan through these rather than through
  open_pdf"* — **refuted.** `hash.rs:87,151`, `grep/worker.rs:167`,
  `extract.rs:618,1745,2109`, `document.rs:438,441,525,528` all use
  `load_xref_with_prev_chain` / `load_xref_linearized`, and neither function
  calls `forward_scan_xref` anywhere in the tree. The regression surface is
  MCP-only — smaller than the parent assumed.

## Evidence runs

All runs from a clean `git archive HEAD` extraction at
`/var/tmp/pdftract-024b8a2a-HEAD` (extraction sha-verified against the HEAD
blobs for `CHECKSUMS.sha256`). The shared working tree does not compile (union
of stranded polish edits — known state since 2026-09-17), so shared-tree runs
are impossible; every run below is the committed state. Timeout-wrapped
per TH-03 policy (`timeout --kill-after=30s 600s`); no run was killed by
timeout; no orphaned processes (no server subprocesses spawned).

| command (from extraction root) | outcome |
|---|---|
| `cargo test -p pdftract-core --test remote_forward_scan_disable` | **PASS (vacuous)** exit 0 — 0 tests collected: file is `#![cfg(feature = "remote")]`. The bead's literal command proves nothing without the feature. |
| `cargo test -p pdftract-core --features remote --test remote_forward_scan_disable` | **PASS** exit 0 — 4/4 (disabled_for_remote, disabled_for_linearized, enabled_for_local, linearized_remote_diagnostic_priority) |
| `cargo test -p pdftract-core --test xref_integration_test` | exit 101 — 8 pass, **2 pre-existing failures** (attribution below): test_forward_scan_recovery, test_circular_prev_detection |
| `cargo test -p pdftract-core --features remote --test remote_fetch_sequence` | exit 101 — named test_forward_scan_disabled_remote **PASS**; 5 unrelated HTTP-probe failures (attribution below) |
| `cargo test -p pdftract-core --features remote --test remote_fetch_sequence -- test_forward_scan_disabled_remote` | **PASS** exit 0 — 1/1 |
| `cargo test -p pdftract-cli --test mcp-tools-integration` | exit 101 — 8 pass incl. **test_get_metadata_performance_on_100_page_pdf PASS** (the parent's exact F1 repro: MCP get_metadata on the 6.09 MB PDF through open_pdf's chunked path — now returns) and test_hash_performance PASS; 2 pre-existing failures (attribution below) |
| `cargo test -p pdftract-cli --test mcp-tools-integration -- test_get_metadata_performance_on_100_page_pdf` | **PASS** exit 0 — 1/1 |
| `cargo build --all-targets` | **PASS** exit 0 |

## Pre-existing failures — attribution (none are guard regressions)

1. **test_forward_scan_recovery, test_circular_prev_detection**
   (xref_integration_test). Empirically reproduced at the **pre-guard** commit
   `3aa2d58a^` with identical panic lines (xref_integration_test.rs:301 and
   xref_helpers.rs:20). Every executed function is sha-identical across the
   guard boundary: `forward_scan_memory` `4520fd69…`, `parse_traditional_xref`
   `e6fa7f1a…`, `detect_linearization` `680d26ca…`,
   `load_xref_with_prev_chain` `b9e32db8…`, test file `e858b7c8…`; fixture
   `truncated_after_xref.pdf` unchanged since creation (c9a8b295) and 564 B
   (memory path — guards unreachable). Pre-existing, part of the documented
   ~300-failure inventory at HEAD.
2. **test_extract_tool_with_real_pdf** (mcp-tools-integration). Fixture
   `tests/sdk-conformance/fixtures/large/100pages.pdf` is 26,976 B — far below
   1 MiB → `forward_scan_memory` (sha-identical across the boundary), so the
   guards cannot affect it. Failure signature ("Failed to resolve root /Pages
   node 2 0 R: object 2 0 R not found") matches the documented
   extract-broken-on-all-fixtures state (0/1377, reconfirmed 2026-09-15; the
   test asserts extraction success, which that note says tests must not).
   Pre-existing.
3. **test_nonexistent_file_returns_path_invalid** (mcp-tools-integration).
   Asserts error code -32003 (PATH_INVALID) but open_pdf returns -32002
   (IO_ERROR) for a nonexistent path. This happens at open_pdf's `is_file()`
   check, which returns before any scan code executes — guards unreachable.
   Error-code conflation, pre-existing at this audit's scope; noted for the
   fleet, not guard-attributable.
4. **remote_fetch_sequence: test_405_fallback_to_get_probe,
   test_block_boundary_handling, test_head_probe_captures_metadata,
   test_no_range_support_detected, test_progressive_tail_fetch.** Zero
   `forward_scan_xref` call sites in those tests (the file's single call site
   is inside the passing test_forward_scan_disabled_remote), and forward scan
   is disabled for remote sources by the very mechanism the passing test
   asserts — the guards are unreachable from every remote-fetch path by
   construction. HTTP range-probing behavior, unrelated subsystem.

**Conclusion: no regression attributable to the EOF-break + high-water-mark
termination guards. The "real regression gets its own bead" clause does not
trigger.** The parent's two caller-surface claims are refuted (safely — the
real surface is smaller): strategy 4 has no production internal caller, and
CLI hash/extract/grep never reach forward scan.

## Audit-environment anomalies (for the record)

- The `3aa2d58a^` extraction's build script fails its TH-06 checksum gate
  non-deterministically: the committed CHECKSUMS.sha256 blob at that commit
  carries a 63-character (truncated) hash for
  `predefined-cmaps/adobe-gb1.json` (line 27), which the parser rejects; one
  run window passed the gate anyway (the pre-guard test reproduction above is
  from a completed test phase, which requires a successful build). HEAD's
  blob is clean and built consistently on every run. Old-tree-only issue; no
  action taken.
- Extraction dirs `/var/tmp/pdftract-024b8a2a-HEAD` and
  `/var/tmp/pdftract-024b8a2a-preguard` are self-owned and removed after this
  audit.
