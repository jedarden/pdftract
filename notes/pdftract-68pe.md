# Verification Note: pdftract-68pe

## Summary
Created `pdftract-docker-build` WorkflowTemplate for building 3 multi-arch Docker images (latest, ocr, full) for amd64 + arm64, pushed to GHCR with cosign keyless signatures.

## Artifacts Created

### 1. Dockerfile (pdftract repo)
- **File**: `/home/coding/pdftract/Dockerfile`
- **Commit**: `79f13c9` (pdftract repo)
- **Features**:
  - Multi-stage build with builder stage using Debian slim
  - Runtime stage conditional on FEATURES build-arg
  - `default` variant uses `gcr.io/distroless/cc-debian12` (~20 MB target)
  - `ocr` and `full` variants use `debian:bookworm-slim` with Tesseract (~120-140 MB target)
  - LICENSE files copied to `/usr/share/doc/pdftract/`

### 2. WorkflowTemplate (declarative-config repo)
- **File**: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/pdftract-docker-build.yaml`
- **Commit**: `b6d0ccf` (declarative-config repo)
- **Templates**:
  - `setup`: Clone repo at tag
  - `build-multi-arch`: Build and push multi-arch images using docker buildx
  - `sign-image`: Sign multi-arch manifest lists with cosign keyless OIDC
- **DAG**: Build all 3 variants in parallel, then sign each

## Acceptance Criteria Status

### PASS
- [x] WorkflowTemplate file lands at `k8s/iad-ci/argo-workflows/pdftract-docker-build.yaml` in `jedarden/declarative-config`
- [x] Template builds 3 image variants (latest, ocr, full)
- [x] Each variant is multi-arch (linux/amd64, linux/arm64)
- [x] Uses docker buildx with QEMU emulation for cross-platform builds
- [x] Pushes to `ghcr.io/jedarden/pdftract` with version and floating tags
- [x] Includes cosign signing template with keyless OIDC
- [x] Uses `ghcr-registry` secret for GHCR authentication
- [x] Uses `github-pat-pdftract` secret for repo access
- [x] Dockerfile supports FEATURES build-arg for variant selection

### WARN (Infrastructure / Test-time limitations)
- [!] **Manual testing required**: Workflow has not been executed on iad-ci cluster yet
  - Reason: No test run performed (requires cluster access and GHCR secret setup)
  - Mitigation: Template structure follows existing patterns (miroir-release, botburrow-agents-build)
  - Next step: Submit test workflow via `kubectl create -f` on milestone tag

- [!] **GHCR secret verification pending**: `ghcr-registry` secret existence not verified
  - Reason: kubectl not available in this environment
  - Mitigation: Secret referenced by existing templates (botburrow-agents-build)
  - Next step: Verify secret exists in argo-workflows namespace before first run

- [!] **OIDC issuer URL not explicitly configured**: Uses cluster default
  - Reason: cosign keyless uses cluster's service account OIDC identity
  - Mitigation: Pattern matches pdftract-github-release.yaml cosign usage
  - Next step: Verify OIDC issuer is registered with Sigstore

### FAIL
- (none)

## References
- Plan section: Release Engineering / Argo WorkflowTemplates, line 3392
- Plan section: Artifact Taxonomy, line 3358
- Plan section: Signing and Provenance, line 3403
- ADR-009 (Argo only)
- Bead: pdftract-68pe
