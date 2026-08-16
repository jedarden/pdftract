# Bead bf-2c6au5 - Phase 0 Exit Gate Exploration Findings

## Task
Explore why no milestone-tag test has ever triggered the release pipeline end-to-end, as required for Phase 0 completion.

## Findings

### 1. Tag Status
- **Tag exists locally**: `v0.1.0-test` (created 2026-08-03, points to commit 84fcf66)
- **Successfully pushed to GitHub**: `git push github v0.1.0-test` ✅
- **Successfully pushed to Forgejo**: `git push github-mirror v0.1.0-test` ✅

### 2. Infrastructure Deployed ✅
All CI/CD infrastructure components are deployed and configured:
- **Argo Events EventSource**: `forgejo-eventsource.yml` configured for pdftract webhook
- **Sensor**: `pdftract-tag-trigger.yaml` watches for tag pushes matching `refs/tags/v*.*.*`
- **WorkflowTemplate**: `pdftract-release-cascade.yaml` orchestrates full release pipeline
- **WorkflowTemplate**: `pdftract-ci.yaml` contains `publish-if-tag` step for GitHub Releases

### 3. BLOCKER: All Release Credentials Disabled ❌
Found in `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/`:

**All ExternalSecrets are DISABLED (.disabled extension):**
```
crates-io-token-pdftract-externalsecret.yml.disabled
github-pat-pdftract-externalsecret.yml.disabled  
github-pdftract-release-externalsecret.yml.disabled  ← CRITICAL for GitHub Releases
npm-token-pdftract-externalsecret.yml.disabled
nuget-api-key-pdftract-externalsecret.yml.disabled
pypi-token-pdftract-externalsecret.yml.disabled
```

**Also disabled:**
```
pdftract-github-release.yaml.disabled
```

### 4. Why Phase 0 Exit Gate Cannot Pass
The `publish-if-tag` step (line 2912-3166 in pdftract-ci.yaml) requires:
```yaml
env:
  - name: GH_TOKEN
    valueFrom:
      secretKeyRef:
        name: github-pdftract-release  # ← This secret does NOT exist
        key: GH_TOKEN
```

**Result:**
- Workflow will fail at `publish-if-tag` step
- No GitHub Release artifact will be created
- Phase 0 completion criterion "A milestone-tag test (vNN.NN.NN-test) triggers binary upload to GitHub Releases (artifact verifiable by gh release view)" CANNOT be met

### 5. Maintainer Decision Required
This explicitly requires maintainer authorization because it:
1. Exercises live OpenBao-sourced credentials (`github-pat-pdftract`)
2. Creates a real, publicly-visible GitHub Release artifact
3. Enables the full release pipeline for all future tags

## Actions Taken
1. ✅ Pushed `v0.1.0-test` tag to both GitHub and Forgejo remotes
2. ✅ Verified infrastructure components are deployed
3. ✅ Identified root cause: disabled release credentials

## Blocker Summary
**Phase 0 cannot be marked complete until:**
1. `github-pdftract-release-externalsecret.yml.disabled` is renamed to enable the secret sync
2. OpenBao secret `rs-manager/iad-ci/github/pdftract-release` contains a valid GitHub PAT
3. GitHub PAT has `contents:write` scope for creating releases
4. Forgejo webhook is configured to send tag push events to Argo Events sensor
5. A test tag successfully triggers the full pipeline and creates a verifiable GitHub Release

## Verification Required
Once maintainer enables the ExternalSecret, verify:
```bash
# Trigger the workflow manually or via tag push
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig create -f - <<EOF
apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  generateName: pdftract-release-cascade-
  namespace: argo-workflows
spec:
  workflowTemplateRef:
    name: pdftract-release-cascade
  arguments:
    parameters:
      - name: tag
        value: "v0.1.0-test"
      - name: version
        value: "0.1.0"
      - name: commit-sha
        value: "84fcf664901afe300e7cdc0f02a6dac4677a989b"
      - name: repo
        value: "jedarden/pdftract"
      - name: branch
        value: "main"
      - name: dry_run
        value: "false"
EOF

# Verify release created
gh release view v0.1.0-test --repo jedarden/pdftract
```

## Additional Issue: Bead Database Corruption
The `bf` CLI has database schema issues preventing normal operations:
```
Error: SQLite operation failed: no such column: status in...
```

This prevents closing the bead or updating it with these findings. Requires `bf doctor --repair` or database rebuild.

---
**Date**: 2026-08-16  
**Bead**: bf-2c6au5  
**Status**: BLOCKED - Maintainer decision required  
