# pdftract-26pc: Argo WorkflowTemplate pdftract-docs-build

## Summary

Created the Argo WorkflowTemplate `pdftract-docs-build` that builds mdBook documentation and deploys to Cloudflare Pages.

## Implementation

### File Created
- `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-docs-build.yaml`

### Template Features

1. **Build Process**
   - Clones pdftract repo at specified tag
   - Installs mdBook and mdbook-linkcheck via cargo (cached in emptyDir for speed)
   - Runs mdbook-linkcheck with fail-on-internal-error policy
   - Builds mdBook to `docs/user-docs/build/user-docs/`
   - Validates build output (index.html, schema directory)

2. **Cloudflare Pages Deploy**
   - Uses wrangler to deploy `build/user-docs/` directory
   - Project name: `pdftract-com`
   - Branch: set to `$TAG` for metadata
   - Uses `--commit-dirty=true` for non-git deployments

3. **Secret Management**
   - Cloudflare API token from ExternalSecret `cloudflare-pages-secret`
   - Key: `CF_API_TOKEN`
   - Sourced from OpenBao via ESO (`rs-manager/iad-ci/cloudflare/pages`)

4. **Idempotency**
   - Re-running against same tag overwrites deployment cleanly
   - Uses wrangler's default deploy behavior

5. **Resource Limits**
   - Requests: 500m CPU, 1Gi memory
   - Limits: 2000m CPU, 4Gi memory
   - Active deadline: 1200 seconds (20 minutes)

### ArgoCD Integration
- Template lives in `k8s/iad-ci/argo-workflows/` directory
- Synced via ArgoCD Application `argo-workflows-ns-iad-ci`
- Auto-sync enabled with prune and self-heal

## Acceptance Criteria

### PASS
- [x] `k8s/iad-ci/argo-workflows/pdftract-docs-build.yaml` exists in declarative-config
- [x] Template synced via ArgoCD (included in `argo-workflows-ns-iad-ci` app)
- [x] Cloudflare token sourced from OpenBao via ESO (`cloudflare-pages-secret`)
- [x] Template is re-runnable (wrangler overwrites on deploy)
- [x] Uses proper secret reference (`CF_API_TOKEN` from `cloudflare-pages-secret`)

### WARN (Infrastructure constraints)
- [ ] Manual workflow submission testing - requires kubectl access to iad-ci cluster
- [ ] Milestone tag deployment verification - requires actual milestone tag and Cloudflare Pages setup

### FAIL
- None

## Git Commits

- declarative-config: `c51ff5f fix(pdftract-26pc): correct docs build paths and Cloudflare Pages config`

## Notes

The template follows the same pattern as `website-build-workflowtemplate.yml`:
- Uses wrangler for Cloudflare Pages deploy
- References the same shared `cloudflare-pages-secret` ExternalSecret
- Uses Rust base image (rust:1.83-slim) for cargo tooling

Key difference from generic website-build:
- Specialized for pdftract docs (hardcoded repo, build command, output dir)
- Includes mdbook-linkcheck validation step
- Builds mdBook instead of npm-based sites

## Cloudflare Pages Configuration

- **Project**: `pdftract-com`
- **Domain**: pdftract.com (DNS configured separately)
- **Production branch**: Configured in Cloudflare Pages dashboard
- **Account ID**: e26f015c7ba47a6ad6219385e77072b7

## Trigger

The template is designed to be triggered on milestone tags (e.g., v0.3.0) via the `pdftract-release-cascade` workflow, which chains `pdftract-crates-publish` before this template to ensure docs.rs links resolve.
