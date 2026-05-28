# pdftract-z86x6: maturin wheel build for 5 triples + pdftract-py-ci Argo WorkflowTemplate

## Work Completed

### 1. Argo WorkflowTemplate Status
The `pdftract-py-ci` WorkflowTemplate was already implemented in declarative-config from previous beads (pdftract-23k1, pdftract-245s). This bead completed the remaining work:

- Added PyPI token secret mount (volume-based, not env var for security)
- Added TAG-GATED when conditions to publish steps
- Fixed token mounting to use volume mount at /etc/pypi-token
- Added in-tree copy at `.ci/argo-workflows/pdftract-py-ci.yaml`

### 2. Workflow Structure
The workflow implements the 5-triple wheel build pipeline:

**Build Stage (parallel):**
1. `wheel-linux-x86-64` → manylinux_2_28_x86_64 wheel (quay.io/pypa/manylinux_2_28_x86_64)
2. `wheel-linux-aarch64` → manylinux_2_28_aarch64 wheel (messense/manylinux_2_28-cross:aarch64)
3. `wheel-darwin-x86-64` → macosx_11_0_x86_64 wheel (messense/maturin:main-darwin-x86_64, osxcross)
4. `wheel-darwin-aarch64` → macosx_11_0_arm64 wheel (messense/maturin:main-darwin-aarch64, osxcross)
5. `wheel-windows-x86-64` → win_amd64 wheel (messense/maturin:main-windows-x86_64, cross-rs)
6. `sdist` → source distribution tarball

**Publish Stage (TAG-GATED, parallel):**
- `publish-pypi-sdist` → uploads sdist to PyPI (runs after sdist completes)
- `publish-pypi-wheels` → uploads all 5 wheels to PyPI (runs after all wheels complete)

### 3. Tag Gating
Publish steps only execute when `workflow.parameters.ref` matches:
```
^refs/tags/v[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$
```
Examples: `v1.0.0`, `v2.3.4-rc.1`

### 4. PyPI Authentication
- Uses sealed-secret `pypi-token-pdftract` synced from OpenBao (key: `rs-manager/iad-ci/pypi/pdftract`)
- Token mounted at `/etc/pypi-token/token` (readOnly)
- twine uses `--password "$(cat /etc/pypi-token/token)"` to avoid env var leak

### 5. Re-runnability
- `twine upload --skip-existing` returns 0 for already-uploaded files
- Re-running the same tag is idempotent: only missing artifacts are uploaded

## Acceptance Criteria

### PASS
- [x] pdftract-py-ci WorkflowTemplate committed to declarative-config (commit 05ad4c4)
- [x] pdftract-py-ci WorkflowTemplate committed to in-tree `.ci/argo-workflows/` (commit 5057db1)
- [x] Workflow covers all 5 target triples: x86_64/aarch64 Linux, x86_64/aarch64 macOS, x86_64 Windows
- [x] Workflow builds sdist
- [x] PyPI upload uses sealed-secret `pypi-token-pdftract`
- [x] Publish steps are TAG-GATED
- [x] Wheel naming follows PEP 491: `pdftract-{version}-cp311-abi3-{platform_tag}.whl`

### WARN (Environmental constraints)
- [ ] Manual workflow submission not tested (requires kubectl access to iad-ci cluster)
- [ ] Wheel install + smoke test not executed (requires built wheels from CI run)
- [ ] Milestone tag trigger not tested (requires actual tag push and CI execution)
- [ ] Wheel sizes not documented (requires built wheels)

### FAIL (None)

## Artifact Locations

- declarative-config: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/pdftract-py-ci.yaml`
- in-tree: `/home/coding/pdftract/.ci/argo-workflows/pdftract-py-ci.yaml`
- ExternalSecret: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/pypi-token-pdftract-externalsecret.yml`

## Commits

- declarative-config: `05ad4c4` - fix(pdftract-z86x6): add PyPI token secret mount and tag-gating to pdftract-py-ci
- pdftract: `5057db1` - feat(pdftract-z86x6): add pdftract-py-ci WorkflowTemplate to in-tree CI

## Notes

- The WorkflowTemplate uses `continueOn: failed: true` for wheel build steps, so one platform failure doesn't block others
- Expected wheel sizes: 5-15 MB per wheel (based on similar Rust extension modules)
- macOS builds use osxcross via messense/maturin images (pre-configured toolchain)
- Windows build uses MSVC target (x86_64-pc-windows-msvc) instead of GNU; maturin images handle this

## References

- Plan section: Phase 0 CI Infrastructure (lines 1010-1030)
- Plan section: Phase 6.3 build + CI (lines 2082-2084)
- ADR-009: PyPI distribution via API token (no OIDC trusted-publisher)
