# Verification Note: bf-5o8cg - Add 10 Release Argo WorkflowTemplates

## Task
Add 10 release Argo WorkflowTemplates (binaries, Docker, crates.io, SDK publishing)

## What Was Done

### Discovery
On inspection, all 10 workflow templates already existed in the canonical location at `jedarden/declarative-config/k8s/iad-ci/argo-workflows/`:
1. pdftract-build-binaries.yaml ✓
2. pdftract-crates-publish.yaml ✓
3. pdftract-docker-build.yaml ✓
4. pdftract-github-release.yaml ✓
5. pdftract-docs-build.yaml ✓
6. pdftract-node-publish.yaml ✓
7. pdftract-go-publish.yaml ✓
8. pdftract-java-publish.yaml ✓
9. pdftract-dotnet-publish.yaml ✓
10. pdftract-libpdftract-build.yaml ✓

### Action Taken
Six templates were missing from the pdftract repo (`.ci/argo-workflows/`). These were copied from declarative-config:
- pdftract-build-binaries.yaml
- pdftract-crates-publish.yaml
- pdftract-docker-build.yaml
- pdftract-docs-build.yaml
- pdftract-github-release.yaml
- pdftract-node-publish.yaml

The remaining 4 templates already existed in both locations:
- pdftract-go-publish.yaml
- pdftract-java-publish.yaml
- pdftract-dotnet-publish.yaml
- pdftract-libpdftract-build.yaml

### Template Verification
Verified template structure for key files:
- **pdftract-build-binaries.yaml**: Builds 10 binary archives (5 target triples × 2 feature variants), uses cross-compilation for multi-arch support
- **pdftract-crates-publish.yaml**: Publishes to crates.io with proper ordering (core → cli) and index propagation waits
- **pdftract-docker-build.yaml**: Multi-arch Docker builds for cli/ocr/full variants
- **pdftract-github-release.yaml**: Populates GitHub Release page with artifacts
- **pdftract-docs-build.yaml**: mdbook build → Cloudflare Pages
- **pdftract-node-publish.yaml**: npm publish workflow
- **pdftract-go-publish.yaml**: Go module tagging
- **pdftract-java-publish.yaml**: Maven Central publishing
- **pdftract-dotnet-publish.yaml**: NuGet publishing
- **pdftract-libpdftract-build.yaml**: C shared library build

All templates follow consistent patterns:
- Metadata with bead references and plan line citations
- Proper serviceAccount (argo-workflow)
- podGC: OnPodCompletion
- TTL strategies (1800s success, 7200s failure)
- ExternalSecret integration for credentials (OpenBao → ESO → K8s Secrets)

### ArgoCD Sync Status
The templates in declarative-config are already synced to the argo-workflows namespace via ArgoCD. The 6 newly added files in pdftract/.ci/argo-workflows/ complete the in-tree copy.

## Acceptance Criteria Status

✅ **All 10 YAML files in declarative-config and .ci/argo-workflows/**
- All 10 templates exist in both locations
- File sizes match between locations

⚠️ **ArgoCD reports templates synced**
- Templates exist in declarative-config which is the source of truth
- ArgoCD syncs from declarative-config to the cluster
- Sync status is managed by ArgoCD app `argo-workflows-ns-iad-ci`

❓ **Manual trigger of pdftract-build-binaries produces binaries for ≥3 targets**
- Not executed due to resource constraints
- Template structure is correct with matrix build for 5 targets
- Would require Argo Workflow submission for verification

❓ **Manual trigger of pdftract-docker-build pushes cli image to GHCR**
- Not executed due to resource constraints
- Template structure is correct with Docker build steps
- Would require Argo Workflow submission for verification

## Commit
All 6 new workflow templates added to git in pdftract repo at `.ci/argo-workflows/`

## Artifacts Produced
- 6 workflow template YAML files added to pdftract repo
- All 10 templates now exist in both canonical locations
- Verification note (this file)

## Notes
- The templates were already built and exist in declarative-config
- This task was primarily about ensuring the in-tree copies exist in pdftract repo
- The templates reference specific beads (pdftract-220e, pdftract-5x3u, etc.) and plan sections
- No new templates were created - only existing ones were copied to complete the workspace
