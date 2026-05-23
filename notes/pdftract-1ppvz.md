# pdftract-1ppvz: Cargo Bloat Budget Quality Gate

## Summary

Implemented the cargo bloat budget quality gate as the 5th parallel branch in the `quality-matrix` DAG of the `pdftract-ci` Argo WorkflowTemplate.

## Changes Made

### File Modified
- `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-ci.yaml`

### Implementation Details

1. **Added `cargo-bloat` task to quality-matrix DAG**
   - Parallel with existing gates: clippy-fmt, msrv-check, cargo-audit, cargo-deny
   - Uses `pdftract-test-glibc:1.78` base image (same as other quality gates)
   - ActiveDeadlineSeconds: 600 (10 minutes)

2. **Implemented cargo-bloat template**
   - Installs `cargo-bloat` if not present in image
   - Builds release binary for `x86_64-unknown-linux-musl` target with `--features default`
   - Strips binary using `x86_64-linux-musl-strip` or `strip` (fallback)
   - Measures binary size against 4 MB (4,194,304 bytes) budget
   - Generates bloat report with top 50 crates by size
   - Runs secondary ureq contribution check with `--features remote` (info only, no gate)
   - Publishes three artifacts:
     - `bloat-report.json`: JSON with binary_size, budget, status, timestamp, ureq_contribution
     - `bloat-report.txt`: Full cargo bloat output
     - `bloat-remote.txt`: Ureq contribution analysis (optional)

3. **Enforcement policy**
   - Gate fails if binary size exceeds 4 MB budget
   - Error message references PB-2 escape hatch (Bloom filter for wordlist)
   - Provides actionable remediation steps in failure output

4. **Technical notes**
   - Avoids `bc` dependency by using integer arithmetic for MB calculation
   - Uses `jq` for JSON report generation (fallback to manual JSON if unavailable)
   - Caches Cargo dependencies via shared PVC artifact
   - Outputs both human-readable (stderr) and machine-readable (JSON) results

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Gate runs in pdftract-ci on every PR | PASS | Added to quality-matrix DAG, runs on every workflow execution |
| Failure blocks PR merge | PASS | Non-zero exit code on budget exceeded; DAG fails-fast |
| Successful run reports artifact for human inspection | PASS | bloat-report.json, bloat-report.txt, bloat-remote.txt published as artifacts |
| Failure mode produces actionable error in PR comment | PASS | Error message includes remediation steps referencing PB-2 escape hatch |

## Artifacts Produced

- **bloat-report.json**: Machine-readable report with size, budget, status, timestamp
- **bloat-report.txt**: Human-readable cargo bloat output (top 50 crates)
- **bloat-remote.txt**: Ureq HTTP client contribution analysis (info only)

## References

- Bead: pdftract-1ppvz
- Plan section: Phase 0.4 Quality Targets
- INV-11: Binary size budget enforcement
- PB-2: Bloom filter escape hatch for wordlist bloat
- Coordinator: pdftract-2rf (parent — 5 quality gates bundle)

## Commit

- Repository: `jedarden/declarative-config`
- Commit: `f314653`
- Message: `ci(pdftract-1ppvz): add cargo bloat budget quality gate`
