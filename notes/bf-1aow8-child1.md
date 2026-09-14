# bf-1aow8-child1 — CLI extraction errors surface their root cause

Bead: pdftract-0c4c760b (split-child of the bf-3bnao chain, re-issued as
pdftract-0c4c760b). Verified 2026-09-14 against a private-target-dir build of
the working tree at main `4a91bf8e` plus the change below.

## Root cause

`main()` matched each command's `Err` with `eprintln!("Error: {}", e)`. Display
on an `anyhow::Error` renders **only the outermost context** — the
`.context("Failed to extract PDF")` wrapper at the `extract_with_cache` call
site — so every cause beneath it (stage context, io error, parse error) was
silently dropped. The exit-code logic was fine; the reporter was lossy.

## Fix (crates/pdftract-cli/src/main.rs only)

1. `format_error(&anyhow::Error) -> String` — renders the error with `Debug`,
   which appends the `Caused by:` chain, then applies the panic hook's
   `redact_secret_patterns` so no credential material can reach the terminal.
2. `report_error(&anyhow::Error)` — prints `Error: {format_error(err)}` to
   stderr. Wired into the `Extract` and `Classify` command arms (both are
   extraction-failure surfaces); exit-code checks are byte-identical to before
   (`e.to_string()` matching), so exit semantics are unchanged.
3. The local extraction context now names the input:
   `.with_context(|| format!("Failed to extract PDF: {}", input.display()))`.
   `std::io` errors carry only errno text ("No such file or directory (os
   error 2)"), never the path, so the path must come from the CLI layer — this
   also covers the `--cache-dir` fingerprint path (`parse_pdf_file`) without
   touching core.
4. `error_reporting_tests` module (3 tests): full chain rendering,
   io-vs-parse distinguishability, secret redaction.

## Captured acceptance outputs (private build, dev profile)

### `pdftract extract /path/that/does/not/exist.pdf --json -`

```
Error: Failed to extract PDF: /path/that/does/not/exist.pdf

Caused by:
    0: Failed to open PDF file
    1: No such file or directory (os error 2)
```
exit code 1

### `pdftract extract tests/fixtures/encoding/fingerprint-match.pdf --json -`

```
Error: Failed to extract PDF: tests/fixtures/encoding/fingerprint-match.pdf

Caused by:
    No /Root reference in trailer
```
exit code 1

The two failures are now distinguishable from each other and from an io
failure; before the fix both printed the identical line
`Error: Failed to extract PDF`.

## Additional evidence

- Unreadable file (chmod 000): `Caused by: 0: Failed to open PDF file
  1: Permission denied (os error 13)` — exit 1. The three formerly identical
  failures (missing / unreadable / parse) are now three distinct messages.
- `--cache-dir` variant of the missing-path case surfaces the full chain:
  `0: Failed to parse PDF file for fingerprint  1: Failed to open PDF file
  2: No such file or directory (os error 2)` — exit 1.
- Successful extraction: exit 0, JSON output code paths untouched (the only
  success-path change is a context closure that never executes on `Ok`).
- Secret redaction verified by unit test: a chain containing "SecretString"
  renders as `[REDACTED:SecretString]`.
- `RUST_BACKTRACE=1` still adds nothing by default (no RUST_BACKTRACE in the
  reporter), unchanged from before.

## Tests

`timeout --kill-after=30s 600s cargo test -p pdftract-cli` (wrapper falls back
to a local cgroup run because the shared tree has uncommitted changes):

- Final stable run: **lib 352 passed / 9 failed**, failing fast at the lib
  stage (documented bare-`cargo test` behavior — bin tests need
  `--no-fail-fast` or an explicit `--bin` filter).
- Full `--no-fail-fast` run at a stable moment: bin **421 passed / 28 failed**
  with **all 3 new `error_reporting_tests` passing**; TH-02 (10), TH-05 (38),
  TH-08 (6) integration suites all pass.
- Every failure is in modules this bead did not touch (`url`, `serve`,
  `inspect`, `pages`, TH-09 inspector-xss). My diff is confined to
  `crates/pdftract-cli/src/main.rs`, which is part of the *bin* target only —
  the lib test target does not compile main.rs, so the lib failures are
  independent of this change by construction. Those files carry other workers'
  in-flight uncommitted edits (at one point during verification the tree did
  not compile at all — an unclosed delimiter in
  `crates/pdftract-core/src/parser/xref.rs` from a concurrent worker; the same
  worker landed 163 lines shortly after and the tree compiled again).
- The acceptance command completes without hanging. Orphan check
  (`pgrep -af 'pdftract[ ]mcp|TH_0|TH-0'`) found three leaked
  `pdftract mcp` servers (systemd-reparented, ~21 min old, not spawned by this
  iteration — leftover from some worker's earlier test run); killed them per
  the repo test-hygiene procedure; zero remain.

## Environment note

Shared target dir `/build/target-workers` is a race zone — concurrent worker
builds replaced the binary mid-verification, producing one misleading run.
All evidence above comes from a private `CARGO_TARGET_DIR` build.
