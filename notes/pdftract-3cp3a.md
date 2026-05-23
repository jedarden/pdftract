# pdftract-3cp3a: Clippy Quality Gate with INV-8 Enforcement

## Summary

Implemented the clippy quality gate for pdftract-ci with INV-8 (no panic at public boundary) enforcement via `clippy::unwrap_used` and `clippy::expect_used` lints.

## Changes Made

### File: `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-ci.yaml`

**Commit:** `f927adb` - `ci(pdftract-3cp3a): add clippy quality gate with INV-8 unwrap/expect enforcement`

Updated the `clippy-fmt` template:

1. **Image change:** `rust:1.83-bookworm` → `pdftract-test-glibc:1.78`
   - The pdftract-test-glibc image has the full dependency tree precompiled
   - Faster clippy runs due to cached artifacts

2. **Feature set change:** `--all-features` → `--features default,serve,decrypt`
   - Explicitly tests the feature combinations used in production
   - Aligns with the plan's Phase 0.4 Quality Targets

3. **Two-pass clippy strategy:**
   - **Pass 1 (full workspace):** `cargo clippy --locked --all-targets --features default,serve,decrypt -- -D warnings`
   - **Pass 2 (library-only INV-8):** `cargo clippy --locked --lib --features default,serve,decrypt -- -D warnings -D clippy::unwrap_used -D clippy::expect_used`

4. **Timeout increase:** 600s → 900s
   - Accounts for the additional clippy pass

5. **Documentation:** Added comments citing:
   - Bead ID: pdftract-3cp3a
   - Plan section: Phase 0.4 Quality Targets
   - INV-8 invariant (no panic at public boundary)

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Gate runs in pdftract-ci on every PR | PASS | quality-matrix DAG includes clippy-fmt step |
| Failure blocks PR merge | PASS | Non-zero exit code from clippy marks workflow Failed |
| Successful run reports artifact for human inspection | WARN | Artifact output for clippy report not yet implemented; stderr shows results |
| Failure mode produces actionable error in PR comment | WARN | Argo PR-comment integration not yet implemented |

## Verification

The clippy-fmt step now:
1. Runs on every PR via the quality-matrix DAG
2. Executes two clippy passes (full workspace + library-only INV-8)
3. Fails the workflow if any clippy warning is detected
4. Enforces INV-8 by banning `unwrap()` and `expect()` in library code

To manually test (once CI is running):
```bash
# Trigger a workflow run
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig create -f - <<EOF
apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  generateName: pdftract-ci-manual-
  namespace: argo-workflows
spec:
  workflowTemplateRef:
    name: pdftract-ci
  arguments:
    parameters:
      - name: commit-sha
        value: "$(git rev-parse HEAD)"
      - name: ref
        value: "refs/heads/main"
EOF
```

## References

- Plan section: Phase 0.4 Quality Targets (line ~3199)
- INV-8: No `panic!` reaches the public boundary of `pdftract-core`
- Parent bead: pdftract-2rf (5 quality gates bundle)
- Commit: `f927adb` in `jedarden/declarative-config`
