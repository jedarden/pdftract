# pdftract-8efa24b7 — CLI `--version` support (clap version attribute)

**Type:** bug (release-blocking for the Homebrew channel)
**Date:** 2026-09-14

## Change

Added the clap `version` attribute to the top-level `Cli` command in
`crates/pdftract-cli/src/main.rs` (line 44):

```rust
#[derive(Parser)]
#[command(version, name = "pdftract")]
#[command(about = "pdftract CLI - PDF extraction and conformance testing", long_about = None)]
struct Cli {
```

clap fills the version from `CARGO_PKG_VERSION` (workspace version `0.1.0`).
No subcommand or other behavior was touched.

## Verification

Built `pdftract-cli` in a private `CARGO_TARGET_DIR`
(`/data/build/target-8efa24b7/debug/debug`, removed after verification) and ran
the produced binary. Binary under test:
`deps/pdftract-3131e332ce40df10` (mtime 19:44, after the forced `touch`
rebuild — see "Build note" below).

| Check | Result |
|---|---|
| `pdftract --version` | **PASS** — prints `pdftract 0.1.0`, exit 0 |
| `pdftract -V` | **PASS** — prints `pdftract 0.1.0`, exit 0 |
| `pdftract --help` | **PASS** — unchanged (same about text and command list) |
| `pdftract list-diagnostics` | **PASS** — exit 0, normal output |
| `pdftract extract --help` | **PASS** — unchanged subcommand usage |

Pre-change comparison: a binary built at 08:49 (before the edit) rejects
`--version` with `error: unexpected argument '--version' found` — the exact
defect reported in the bead. The post-change binary prints the version.

### Tests

`timeout --kill-after=30s 600s cargo test -p pdftract-cli --test test_hash_exit_codes`
→ **3 passed, 1 failed** (`test_hash_nonexistent_file`).

The failure is **pre-existing and unrelated**: `pdftract hash /nonexistent`
exits 2 (expected 4) identically on the pre-change 08:49 binary and the
post-change binary. A top-level clap `version` attribute cannot alter
subcommand exit codes. This matches the known ~300 pre-existing test failures
at HEAD recorded in project memory.

### Build note

Two pitfalls hit during verification, both documented in project memory:

1. Binaries land in `deps/` under the target dir (`target/debug/pdftract`
   stays absent), and a stale shared-tree binary from a concurrent worker's
   08:49 build initially masked the result. Forcing a rebuild with
   `touch crates/pdftract-cli/src/main.rs` and running the binary whose mtime
   postdates the touch resolved it.
2. `CARGO_TARGET_DIR` is the target *root* — passing `.../debug` nests builds
   under `debug/debug/`. The first functional test therefore hit a stale
   binary; re-verified against the correct artifact.

## Acceptance criteria

- **PASS** — `pdftract --version` and `pdftract -V` exit 0 and print a string
  containing the workspace version (`pdftract 0.1.0`).
- **PASS** — existing subcommand behavior unchanged (`--help` and
  `list-diagnostics` / `extract` spot-checks identical).
- **PASS** — no other behavioral changes; the one targeted test run shows only
  a pre-existing failure (proven identical on the pre-change binary).
