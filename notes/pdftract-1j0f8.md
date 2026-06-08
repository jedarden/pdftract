# pdftract-1j0f8: CLI Reference Documentation

## Summary

Verified CLI reference documentation infrastructure. Fixed a clap configuration bug that prevented the generator from running (duplicate short option `-s` in `conformance` subcommand).

## Work Completed

### 1. Bug Fix: Clap Short Flag Conflict
**File:** `crates/pdftract-cli/src/cli.rs`

**Problem:** The `conformance` subcommand had duplicate short options:
- `--suite` used `-s`
- `--sdk` used `-s` (conflict!)

**Solution:** Changed `--sdk` short option to `-k` (as used in CI workflow).

**Before:**
```rust
#[arg(short, long, default_value = "pdftract")]
sdk: String,
```

**After:**
```rust
#[arg(short = 'k', long, default_value = "pdftract")]
sdk: String,
```

### 2. Verification Tests

1. **CLI Reference Generation:**
   ```bash
   cargo run --bin gen-cli-reference -- --output /tmp/cli-reference-test.md
   ```
   Result: PASS - Generated successfully with preserved hand-curated content.

2. **mdBook Build:**
   ```bash
   cd docs/user-docs && mdbook build
   ```
   Result: PASS - HTML book built successfully to `build/user-docs/`.

3. **CI Gate Check:**
   The `cli-ref-gen` template in `.ci/argo-workflows/pdftract-ci.yaml` (lines 1952-2042) correctly:
   - Regenerates CLI reference via `cargo run --bin gen-cli-reference`
   - Compares output to committed file
   - Fails build on any diff

## Acceptance Criteria Status

**PASS:**
- cli-reference.md exists at `docs/user-docs/src/cli-reference.md`
- Auto-gen compiles and runs: `cargo run --bin gen-cli-reference`
- CI gate `cli-ref-gen` fails on stale content
- mdBook builds and renders without errors
- cli-reference.md is included in SUMMARY.md

**WARN:**
- None

**FAIL:**
- None

## Commit

- **Files Changed:**
  - `crates/pdftract-cli/src/cli.rs`: Fixed short flag conflict

## Retrospective

**What worked:** The CLI reference infrastructure was already complete with clap-markdown, CI gate, and mdBook integration.

**What didn't:** The clap configuration bug prevented the generator from running - needed to debug panic output to find the duplicate short option.

**Surprise:** The `-s` conflict existed but was masked - CI gate would catch it once docs needed regeneration.

**Reusable pattern:** When adding clap short options, always check for conflicts within the same subcommand context.
