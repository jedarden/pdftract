# Verification Note: pdftract-245s (pdftract-py-ci WorkflowTemplate)

## Summary

Implemented the `pdftract-py-ci` WorkflowTemplate at `k8s/iad-ci/argo-workflows/pdftract-py-ci.yaml` in `jedarden/declarative-config`. The template builds 5 platform wheels + 1 source distribution using maturin and publishes to PyPI via twine.

## File Location

- **WorkflowTemplate**: `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-py-ci.yaml`
- **ExternalSecret**: `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pypi-token-pdftract-externalsecret.yml`
- **Commit**: `9d40a65` (feat(pdftract-245s): implement pdftract-py-ci WorkflowTemplate with maturin builds)

## Acceptance Criteria Status

### PASS

1. **WorkflowTemplate file lands at correct location**
   - File exists at `k8s/iad-ci/argo-workflows/pdftract-py-ci.yaml`
   - Commit `9d40a65` added the file to `jedarden/declarative-config`

2. **Failed platform publish does NOT cancel other matrix items**
   - Verified: All 5 wheel build tasks have `continueOn.failed: true`
   - The sdist and publish steps run independently
   - Lines 163-188 in the YAML confirm continueOn behavior

3. **Two consecutive runs are idempotent**
   - Verified: `twine upload --skip-existing` is used (line 702)
   - Returns 0 for already-uploaded files
   - Only missing artifacts are uploaded on re-run

4. **PyPI token from ESO Secret**
   - ExternalSecret `pypi-token-pdftract` exists
   - Syncs from OpenBao key `rs-manager/iad-ci/pypi/pdftract`
   - Referenced in publish-pypi template (lines 710-714)

5. **Wheel naming convention**
   - Uses abi3 tagging: `pdftract-X.Y.Z-cp311-abi3-<platform_tag>.whl`
   - One wheel per platform serves Python 3.11+

6. **Parallel builds**
   - All 5 wheel builds run in parallel under DAG dependencies
   - sdist builds in parallel with wheels

### WARN (Environmental - Not Testable in This Session)

1. **A test workflow against a sample tag produces artifacts**
   - Requires Argo Workflows submission to `iad-ci` cluster
   - Would verify wheel and sdist artifact generation
   - Command to test: `kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig create -f <workflow-manifest>`

2. **The `twine upload` step succeeds with ESO-provided token**
   - Requires actual PyPI token to be present in OpenBao
   - Requires a real tag to be published
   - Would verify `twine upload --skip-existing` succeeds

3. **`pip install pdftract` on clean machine installs appropriate wheel**
   - Requires PyPI publish to complete
   - Would verify pip selects correct platform wheel
   - Test command: `pip install pdftract==X.Y.Z`

## Implementation Details

### Platform Wheels Built

| Platform | Container Image | Target Triple |
|----------|-----------------|---------------|
| manylinux_2_28_x86_64 | quay.io/pypa/manylinux_2_28_x86_64 | x86_64-unknown-linux-gnu |
| manylinux_2_28_aarch64 | messense/manylinux_2_28-cross:aarch64 | aarch64-unknown-linux-gnu |
| macosx_11_0_x86_64 | messense/maturin:main-darwin-x86_64 | x86_64-apple-darwin |
| macosx_11_0_arm64 | messense/maturin:main-darwin-aarch64 | aarch64-apple-darwin |
| win_amd64 | messense/maturin:main-windows-x86_64 | x86_64-pc-windows-msvc |

### DAG Structure

```
setup -> [parallel: wheel-linux-x86_64, wheel-linux-aarch64,
           wheel-darwin-x86_64, wheel-darwin-aarch64,
           wheel-windows-x86_64, sdist] ->
    [parallel: publish-pypi-sdist (after sdist),
                publish-pypi-wheels (after all wheels)]
```

### Key Features

- **Maturin version**: Installed via cargo in setup, pip in manylinux containers
- **abi3 tagging**: `--interpreter python3.11 --abi3` for Python 3.11+ compatibility
- **Strip symbols**: `--strip` for smaller wheel sizes
- **Reproducible builds**: `SOURCE_DATE_EPOCH` set from git commit timestamp
- **Shared cargo cache**: 50Gi PVC for faster rebuilds
- **Wheel artifacts PVC**: 5Gi for collecting wheels before upload

## ADR-009 Compliance

Per ADR-009: NO OIDC trusted-publisher (GitHub Actions exclusive feature). The workflow uses PyPI API token from ExternalSecret, not OIDC.

## References

- Plan section: Release Engineering / Argo WorkflowTemplates, line 3390
- Plan section: Artifact Taxonomy, lines 3355-3356
- ADR-009 (PyPI token auth, not GitHub OIDC)
- Phase 6.3 (PyO3 binding provides the cdylib)
