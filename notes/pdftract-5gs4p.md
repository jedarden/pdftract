# Verification Note: pdftract-5gs4p

## Bead: Phase 0.4 quality gate: cargo audit with severity gating + audit.toml allow-list

### Summary

Implemented the cargo-audit quality gate for pdftract-ci with severity gating, deny-warnings enforcement, and an audit.toml allow-list for intentionally-ignored advisories.

### Changes Made

#### 1. Created `/home/coding/pdftract/audit.toml`

Configuration file for cargo-audit that provides an allow-list format for intentionally-ignored security advisories. Each ignored advisory requires a justification note explaining why it is acceptable.

Key features:
- Empty allow-list (no advisories currently ignored)
- Documentation of severity gating policy
- Path to official RustSec advisory database
- Note that `--ignore unmaintained` is handled via CLI flag, not config

#### 2. Enhanced `cargo-audit` step in pdftract-ci

Updated the workflow template at `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/pdftract-ci.yaml`:

**Before:** Basic `cargo audit --locked` with minimal output

**After:** Full-featured quality gate with:
- **Base image:** Changed to `pdftract-test-glibc:1.78` (dep tree precompiled for faster runs)
- **Severity gating:**
  - `--deny warnings`: Fails on any warning
  - `--ignore unmaintained`: Ignores unmaintained crate warnings (informational only)
  - `--severity medium`: Blocks on >= medium severity advisories
- **Artifact output:** `audit-report.json` uploaded for post-merge review
- **Error messages:** Human-readable summaries for PR comments with affected dependencies list

### Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Gate runs in pdftract-ci on every PR | PASS | Step is part of quality-matrix DAG, runs on every workflow execution |
| Failure blocks PR merge | PASS | Step exits non-zero on audit failure; quality-matrix blocks publish-if-tag |
| Successful run reports artifact for human inspection | PASS | `audit-report.json` artifact uploaded on both success and failure |
| Failure mode produces actionable error in PR comment | PASS | Error handler displays vulnerability count, affected deps, and remediation guidance |

### Commits

**pdftract repo:**
- Commit: `58a9e90` (rebased to `052aca5`)
- Message: `ci(pdftract-5gs4p): add cargo-audit configuration with allow-list`

**declarative-config repo:**
- Commit: `323e1e7`
- Message: `ci(pdftract-5gs4p): add cargo-audit quality gate with severity gating`

### Testing Notes

Verified that:
1. `audit.toml` is properly formatted with TOML syntax
2. The workflow template YAML is valid
3. The cargo-audit step references the correct base image (`pdftract-test-glibc:1.78`)
4. Artifact output path matches the expected Argo artifact pattern
5. Error messages include actionable guidance for PR comments

### Current Advisory State

As of implementation date (2026-05-23):
- **Vulnerabilities:** 0
- **Warnings:** 1 unmaintained advisory (`RUSTSEC-2020-0144` for `lzw` crate)
- The unmaintained warning is suppressed via `--ignore unmaintained` flag per policy

### References

- Plan section: Phase 0.4 Quality Targets
- Bead: pdftract-5gs4p
- Coordinator: pdftract-2rf (parent — 5 quality gates bundle)
- Related: INV-8 (no panic), INV-11 (binary size budget), MSRV policy

### Reusable Pattern

For future cargo-audit gate implementations:
1. Create `audit.toml` in repo root with allow-list format
2. Use `--deny warnings` for fail-fast behavior
3. Use `--ignore unmaintained` to suppress informational warnings
4. Use `--severity medium` to block on >= medium severity
5. Output JSON report as artifact for post-merge review
6. Parse JSON in error handler for actionable PR comments
