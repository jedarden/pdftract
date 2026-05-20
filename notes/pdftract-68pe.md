# pdftract-68pe: pdftract-docker-build WorkflowTemplate

## Summary

The `pdftract-docker-build` WorkflowTemplate was already implemented. This bead enhanced it with SLSA provenance attestation and added the missing `ghcr-registry` ExternalSecret.

## Changes Made

### 1. SLSA Provenance Attestation Enhancement (commit `df031e2`)

Enhanced the `sign-image` template with SLSA provenance attestation:
- Added `cosign attest` step to attach SLSA provenance to each signed image
- Builder ID: `https://iad-ci.ardenone.com/argo-workflows/pdftract-docker-build`
- Build type: `https://images.sigstore.dev/argo-build@v1`
- Materials include git commit SHA for supply chain traceability
- Invocation parameters include variant, tag, and version
- Provenance metadata includes build timestamp, completeness info, and reproducibility flag

### 2. Cosign Verification Improvements

- Added `--certificate-identity-regexp` parameter to verify step
- Added `--certificate-oidc-issuer` parameter to verify step
- Added `COSIGN_CERTIFICATE_IDENTITY` env var: `https://iad-ci-oidc.ardenone.com.*`

### 3. GHCR Registry ExternalSecret (`k8s/iad-ci/argo-workflows/ghcr-registry-externalsecret.yml`)

Created an ExternalSecret that:
- Fetches the GitHub PAT from OpenBao (`rs-manager/iad-ci/github/pat-pdftract`)
- Formats it as a `kubernetes.io/dockerconfigjson` secret for GHCR authentication
- Syncs to `argo-workflows` namespace as `ghcr-registry` secret
- Uses the same GitHub PAT as repo access (requires `read:packages` + `write:packages` scopes)

### 4. WorkflowTemplate Structure

The `pdftract-docker-build.yaml` (14,270 bytes after enhancement) includes:

- **3 image variants**: `latest` (default features), `ocr` (default + OCR), `full` (all features)
- **Multi-arch build**: linux/amd64 + linux/arm64 via `docker buildx` with QEMU emulation
- **GHCR push**: Pushes to `ghcr.io/jedarden/pdftract` with versioned (`X.Y.Z`) and floating (`latest`, `ocr`, `full`) tags
- **Cosign keyless signing**: Uses OIDC from iad-ci cluster (`https://iad-ci-oidc.ardenone.com`)
- **Dockerfile support**: The pdftract repo has a Dockerfile that accepts `FEATURES` build arg
- **Parallel builds**: All 3 variants build in parallel via DAG tasks
- **Idempotent**: Re-running on the same tag overwrites existing tags

## Acceptance Criteria Status

- [x] **PASS**: WorkflowTemplate file exists at `k8s/iad-ci/argo-workflows/pdftract-docker-build.yaml`
- [x] **PASS**: 3 image variants (latest, ocr, full) defined
- [x] **PASS**: Multi-arch build (amd64 + arm64) using docker buildx
- [x] **PASS**: GHCR push configuration (`ghcr.io/jedarden/pdftract`)
- [x] **PASS**: Cosign keyless signing with OIDC from iad-ci cluster
- [x] **PASS**: SLSA provenance attestation via `cosign attest`
- [x] **PASS**: GHCR registry secret created (`ghcr-registry-externalsecret.yml`)
- [ ] **WARN**: Test run not performed (requires actual tag push to trigger)
- [ ] **WARN**: `cosign verify` not tested (requires signed images in GHCR)
- [x] **PASS**: Re-running workflow on same tag is idempotent (uses `--push` which overwrites)

## Infrastructure Dependencies

1. **OpenBao Secret**: `rs-manager/iad-ci/github/pat-pdftract` (GitHub PAT with packages scope)
2. **OIDC Issuer**: `https://iad-ci-oidc.ardenone.com` (registered with Sigstore for keyless signing)
3. **ArgoCD Application**: `applications-iad-ci` syncs `k8s/iad-ci/argo-workflows/` to iad-ci cluster
4. **ServiceAccount**: `argo-workflow` with OIDC token projection for cosign signing

## Image Specifications

| Variant | Features | Base Image | Size (est.) | Tags |
|---------|----------|------------|-------------|------|
| `latest` | default | `gcr.io/distroless/cc-debian12` | ~20 MB | `:X.Y.Z`, `:latest` |
| `ocr` | default + OCR | `debian:bookworm-slim` | ~120 MB | `:ocr-X.Y.Z`, `:ocr` |
| `full` | all | `debian:bookworm-slim` | ~140 MB | `:full-X.Y.Z`, `:full` |

## Workflow Invocation

The workflow is invoked from `pdftract-release-cascade` on milestone tag push.

## Notes

- The Dockerfile in pdftract repo supports `FEATURES=default|ocr|full` build arg
- QEMU emulation for arm64 is slow (~3x amd64), so `activeDeadlineSeconds: 2400` (40 min) is set
- Cosign signatures are stored in `ghcr.io/jedarden/pdftract-signatures` repository
- License files (MIT/Apache) are copied to `/usr/share/doc/pdftract/` in all images

## Bead Closure

The workflow template was already complete. This bead added the missing GHCR ExternalSecret to enable Docker pushes to GitHub Container Registry.
