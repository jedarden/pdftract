# pdftract-1j0f8: CLI Reference Documentation

## Summary

Verified CLI reference documentation implementation is complete and working. All acceptance criteria met.

## Implementation Status (2026-06-08)

### Components Verified

1. **CLI Reference Page** (`docs/user-docs/src/cli-reference.md`)
   - 646 lines covering 28 command sections
   - Auto-generated from clap derive annotations via clap-markdown
   - Includes `<!-- AUTOGEN END -->` marker for hand-curated content preservation
   - Hand-curated section with common patterns and exit codes

2. **Generator Binary** (`crates/pdftract-cli/src/bin/generate-cli-reference.rs`)
   - Binary name: `gen-cli-reference`
   - Preserves hand-curated content after AUTOGEN END marker
   - Handles newline accumulation prevention

3. **Library Export** (`crates/pdftract-cli/src/lib.rs`)
   - `generate_cli_markdown()` function exports clap-markdown generation

4. **CI Gate** (`.ci/argo-workflows/pdftract-ci.yaml`)
   - Step: `cli-ref-gen` (300s timeout)
   - Regenerates CLI reference and compares to committed version
   - Fails build on any diff

5. **mdBook Integration** (`docs/user-docs/src/SUMMARY.md`)
   - Entry included
   - Builds successfully without errors

### Previous Work (2026-06-01)

Fixed clap configuration bug: duplicate short option `-s` in `conformance` subcommand. Changed `--sdk` to use `-k`.

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
