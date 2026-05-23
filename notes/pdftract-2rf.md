# Verification Note: pdftract-2rf (Quality Matrix Implementation)

## Summary

Implemented Phase 0.4: Static analysis and quality gates for the `pdftract-ci` Argo WorkflowTemplate. Added the missing `cargo-bloat` template and cleaned up orphaned code.

## Changes Made

### 1. Added `cargo-bloat` Template (lines 892-1018)
- **Purpose**: Enforce 4 MB binary size budget for x86_64-unknown-linux-musl target
- **Implementation**:
  - Installs `cargo-bloat` if not present in the image
  - Runs `cargo bloat --release --features default --crates --target x86_64-unknown-linux-musl -n 50`
  - Measures stripped binary size using `x86_64-linux-musl-strip`
  - Enforces 4,194,304 byte (4 MB) budget
  - Generates `bloat-report.json` artifact with:
    - Stripped size in bytes
    - Budget comparison
    - Raw cargo-bloat output
    - Remote feature analysis (for PB-5 tracking)
  - Fails with actionable error if budget exceeded (references PB-2 Bloom filter escape hatch)

### 2. Removed Orphaned `clippy-unwrap` Template
- **Why removed**: The `clippy-fmt` template already performs both clippy passes:
  1. Full workspace check with `-D warnings`
  2. Library-only INV-8 check with `-D clippy::unwrap_used -D clippy::expect_used`
- The orphaned `clippy-unwrap` template was not referenced in the quality-matrix DAG

### 3. Updated Documentation Comments
- Updated DAG structure comments to reflect current template names
- Removed obsolete `clippy-unwrap` references from comments

## Quality Matrix Status

All 5 Tier 1 quality gates are now implemented:

| Gate | Template | Status |
|------|----------|--------|
| clippy-fmt | `clippy-fmt` | ✓ (existing) |
| msrv-check | `msrv-check` | ✓ (existing) |
| cargo-audit | `cargo-audit` | ✓ (existing) |
| cargo-deny | `cargo-deny` | ✓ (existing) |
| cargo-bloat | `cargo-bloat` | ✓ (NEW) |

## Acceptance Criteria

### PASS Criteria
- [x] All five quality steps appear in the WorkflowTemplate DAG as `quality-matrix`
- [x] `cargo-bloat` template is defined with proper resource limits and artifact output
- [x] Binary size budget enforcement is implemented (<= 4 MB for x86_64-unknown-linux-musl)
- [x] Remote feature tracking is included for PB-5 (alt-feature escape hatch data)
- [x] `bloat-report.json` is published as artifact

### WARN Criteria (Infrastructure-related, out of scope)
- [ ] Green PR run shows all five passing within 8 min combined wall-clock
  - **Reason**: Cannot submit actual PR/CI run without access to iad-ci cluster
  - **Verification method**: Manual inspection of workflow templates confirms all gates are properly configured

### FAIL Criteria (To be tested manually)
- [ ] A deliberate `unwrap()` added inside `crates/pdftract-core/src/lib.rs` causes the clippy gate to fail
  - **Reason**: Requires code change and CI execution to verify
- [ ] A deliberate advisory-vulnerable dep causes the audit gate to fail
  - **Reason**: Requires modifying Cargo.lock and CI execution
- [ ] A deliberate GPL-licensed dep causes the deny gate to fail
  - **Reason**: Requires adding GPL dependency and CI execution
- [ ] A deliberate use of Rust 1.79+ feature causes the MSRV gate to fail
  - **Reason**: requires code change and CI execution
- [ ] `bloat-report.json` is inspectable from the Argo UI
  - **Reason**: Requires actual workflow execution on iad-ci cluster

## Configuration Files Verified

### audit.toml (existing)
- Located at `/home/coding/pdftract/audit.toml`
- Configured with:
  - Advisory ignore format documented
  - Terse output for CI logs
  - Official RustSec database path
  - `--ignore unmaintained` flag passed in CI (not in config)

### deny.toml (existing)
- Located at `/home/coding/pdftract/deny.toml`
- Configured with:
  - License allowlist: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-DFS-2016
  - MPL-2.0 exceptions for cbindgen (ADR-001) and option-ext (ADR-002)
  - Advisory ignores for RUSTSEC-2020-0144 (lzw), RUSTSEC-2021-0145 (atty), RUSTSEC-2024-0375 (atty), RUSTSEC-2025-0020 (pyo3)
  - Wildcard dependencies denied
  - Unknown registries and git sources denied

## Technical Notes

### cargo-bloat Implementation Details
1. **Target-specific gating**: Only x86_64-unknown-linux-musl is gated. Other targets (macOS, Windows) are informational due to larger binary metadata overhead.
2. **Stripped size measurement**: Uses `x86_64-linux-musl-strip` to get accurate production binary size.
3. **JSON report structure**:
   ```json
   {
     "timestamp": "ISO-8601",
     "commit_sha": "workflow.parameters.commit-sha",
     "target": "x86_64-unknown-linux-musl",
     "features": "default",
     "stripped_size_bytes": <size>,
     "budget_bytes": 4194304,
     "within_budget": true|false,
     "raw_output": "<cargo-bloat text output>",
     "remote_features_raw": "<cargo-bloat --features remote output>"
   }
   ```
4. **Error handling**: Provides clear next step (PB-2 Bloom filter) when budget exceeded.

### Template Resource Allocation
- CPU: 1000m request, 2000m limit
- Memory: 2Gi request, 4Gi limit
- ActiveDeadlineSeconds: 600 (10 minutes)

## References
- Plan section: Phase 0, line 1007 (clippy, bloat, audit, deny, MSRV)
- INV-8 (no panic at public boundary)
- R2 (binary size risk), PB-2 (Bloom filter escape hatch)
- ADR-002 (wordlist storage) - Note: ADR-002 in repo is MPL-2.0 exception, not wordlist storage. Wordlist ADR is expected in later phase.

## Files Modified
- `.ci/argo-workflows/pdftract-ci.yaml` (added cargo-bloat template, removed clippy-unwrap orphan, updated comments)

## Commit Hash
(TBD - will be populated after commit)
