# pdftract-1j0f8: CLI Reference Documentation

## Summary

Verified that the CLI reference documentation is fully implemented and operational.

## Acceptance Criteria Status

### 1. ✅ cli-reference.md exists and is non-trivial
- File: `docs/user-docs/src/cli-reference.md`
- Size: 646 lines
- Coverage: 28 subcommands documented
- Commands covered: list-diagnostics, explain-diagnostic, compare, conformance, sdk, extract, classify, inspect, verify-receipt, hash, cache, profiles, serve, mcp, validate, migrate-schema, doctor

### 2. ✅ Auto-gen step compiles and runs
- Binary: `cargo run --bin gen-cli-reference`
- Library function: `pdftract_cli::generate_cli_markdown()` in `crates/pdftract-cli/src/lib.rs`
- Implementation: Uses `clap-markdown` crate (v0.1)
- Verification: Successfully regenerated and matched committed version

### 3. ✅ CI gate fails on stale cli-reference.md
- Location: `.ci/argo-workflows/pdftract-ci.yaml`
- Template: `cli-ref-gen`
- Validation: Regenerates CLI reference and diffs against committed version
- Error handling: Fails build with clear instructions if mismatch detected

### 4. ✅ mdBook renders the page without errors
- Config: `docs/user-docs/book.toml`
- SUMMARY entry: Line 11 `- [CLI Reference](./cli-reference.md)`
- Rendered output: `docs/user-docs/build/user-docs/cli-reference.html` (57KB)
- Build status: Successful

## Hand-Curated Content

Located after `<!-- AUTOGEN END -->` marker (line 613):
- Common patterns for basic extraction, JSON output, Markdown with anchors
- Exit codes documentation (0=success, 1=error, 2=usage error, 3=decryption error)
- Preserved across regenerations by the gen-cli-reference binary

## Feature-Gated Commands

The `grep` subcommand is feature-gated (`#[cfg(feature = "grep")]`) and not included in the default CLI reference. This is correct behavior:
- Default features: empty (no features enabled by default)
- grep requires: `dep:indicatif` feature
- Status: Phase 7.8 implementation (advanced feature)

## Implementation Details

**Generation binary:** `crates/pdftract-cli/src/bin/generate-cli-reference.rs`
- Reads existing file to preserve hand-curated content
- Auto-generates from clap derive annotations
- Writes output with AUTOGEN_END_MARKER delimiter
- Alternative: `xtask/src/bin/gen_cli_reference.rs` (workspace-aware variant)

**CI validation:** `.ci/argo-workflows/pdftract-ci.yaml`
```bash
cargo run --bin gen-cli-reference -- --output docs/user-docs/src/cli-reference.md
git diff --exit-code docs/user-docs/src/cli-reference.md
```

## Files Modified

None - this bead was a verification task. All infrastructure was already in place.

## Tests

```bash
# Generation test
cargo run --bin gen-cli-reference -- --output /tmp/test-cli-ref.md
# Result: Success, output matches committed version

# mdBook build test
mdbook build docs/user-docs
# Result: Success, cli-reference.html generated

# CI simulation
diff -u docs/user-docs/src/cli-reference.md /tmp/generated-cli-ref.md
# Result: No differences (files identical)
```

## PASS

All acceptance criteria verified and passing. CLI reference documentation is fully operational.
