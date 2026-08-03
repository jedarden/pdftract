# Phase 0 Exit Gate Status: Milestone Tag Test Verification

## Bead
bf-2c6au5 - Phase 0 exit gate unmet: no milestone-tag test has ever triggered the release pipeline end-to-end

## Date
2026-08-03 (updated)

## Current State Analysis

### Local Tags
```bash
$ git tag -l
needle-cleanup-backup-20260801
v0.1.0-test
```

**Status:** Test tag exists locally, created on 2026-08-03 19:27:33
- **Tag:** v0.1.0-test
- **Commit:** 84fcf66 (docs(bf-19an5y): document SDK repo hosting policy contradiction)
- **Creator:** jedarden <github@jedarden.com>

### Remote Tags (GitHub)
```bash
$ git ls-remote --tags github
# No output - zero tags on GitHub
```

**Status:** No tags exist on GitHub remote

### GitHub Releases
```bash
$ curl -sk "https://github.com/jedarden/pdftract/releases" | grep -c "release"
# 0 releases found
```

**Status:** No releases exist on GitHub

### Previous Attempt (2026-08-03)
A previous attempt was documented in `notes/bf-2c6au5.md`:
1. ✅ Created local tag `v0.1.0-test`
2. ❌ Attempted push to GitHub - outcome unknown
3. ❌ Manual pipeline trigger blocked by credential expiry

**Current finding:** The tag was NOT successfully pushed to GitHub, contrary to what was documented. The tag exists locally but not remotely.

## Phase 0 Completion Criterion

From `docs/plan/plan.md` line 3276:

> - [ ] A milestone-tag test (`vNN.NN.NN-test`) triggers binary upload to GitHub Releases (artifact verifiable by `gh release view`)

### Breaking Down the Requirement

The requirement has THREE distinct parts that must all be satisfied:

1. **Tag Creation & Push:** A tag matching pattern `vNN.NN.NN-test` must be pushed to GitHub
   - Current state: ❌ Tag exists locally but NOT on GitHub
   - Evidence: `git ls-remote --tags github` returns empty

2. **Pipeline Trigger:** Pushing the tag must trigger the Argo Workflows CI/CD pipeline
   - Trigger mechanism: GitHub webhook → WorkflowEventBinding → pdftract-ci WorkflowTemplate
   - Current state: ❌ Cannot verify (no tag on GitHub to trigger webhook)

3. **Binary Upload & Verification:** Pipeline must upload binaries to GitHub Releases
   - Verification method: `gh release view vNN.NN.NN-test`
   - Current state: ❌ No release exists
   - Evidence: GitHub releases page shows 0 releases

## What Has Been Built (Infrastructure Ready)

The following pipeline stages were built by previous beads and are ready:

### Build Stage (pdftract-build-binaries.yaml)
- **Bead:** pdftract-2x7y (GitHub release workflow)
- **Capabilities:**
  - Cross-compile for 5 targets (x86_64/aarch64 Linux musl, macOS x64/ARM64, Windows x64)
  - Generate SHA256SUMS
  - Sign with cosign (keyless OIDC)
  - Generate SLSA provenance
  - Generate CycloneDX SBOM

### Release Stage (pdftract-github-release.yaml)
- **Bead:** pdftract-2x7y
- **Capabilities:**
  - Collect all build artifacts
  - Create GitHub Release with all assets
  - Mark as pre-release for `-test` suffixed tags

### Integration Stage (pdftract-ci.yaml)
- **Location:** `.ci/argo-workflows/pdftract-ci.yaml`
- **Workflow:** `setup → build-matrix → test-matrix → quality-matrix → publish-if-tag`
- **Trigger condition:** `when: "{{workflow.parameters.is-tag}} == true"`

## Why This Is OPS-GATED

This bead is marked OPS-GATED because it requires:

1. **Live Credential Exercise:** Uses OpenBao-sourced secret `github-pat-pdftract` (GitHub Personal Access Token)
   - This token has write access to GitHub Releases
   - Cannot be tested without touching production infrastructure

2. **Public Artifact Creation:** Creates a real, publicly-visible GitHub Release
   - Even with `-test` suffix, the release is visible to anyone
   - Cannot be rolled back without manual cleanup

3. **Cross-System Coordination:** Requires coordination between:
   - GitHub (tag push → webhook)
   - Argo Workflows (pipeline execution)
   - OpenBao (credential injection)
   - GitHub Releases (artifact upload)

## What Needs To Happen

### Option 1: Manual Pipeline Test (Recommended for Verification)

This approach tests the pipeline WITHOUT pushing a tag to GitHub:

```bash
# Submit a manual workflow with tag parameters set
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig create -f - <<EOF
apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  generateName: pdftract-ci-test-manual-
  namespace: argo-workflows
spec:
  workflowTemplateRef:
    name: pdftract-ci
  arguments:
    parameters:
      - name: commit-sha
        value: "c8fd8cc466728251127ee18a4716e6fa309a5479"  # Current HEAD
      - name: ref
        value: "refs/tags/v0.1.0-test"
      - name: repo-url
        value: "https://github.com/jedarden/pdftract.git"
      - name: is-tag
        value: "true"
EOF
```

**Advantages:**
- Tests the entire pipeline end-to-end
- Uses real OpenBao credentials
- Creates real GitHub Release
- No git tag required (workflow parameter overrides)

**Verification after run:**
```bash
# Check if release was created
gh release view v0.1.0-test --repo jedarden/pdftract

# Verify artifacts
gh release view v0.1.0-test --repo jedarden/pdftract --json assets -q '.[].name'
```

### Option 2: Full End-to-End Test (Requires Maintainer Authorization)

This approach tests the COMPLETE flow including GitHub webhook trigger:

```bash
# 1. Push the existing local tag to GitHub
git push github v0.1.0-test

# 2. Monitor Argo Workflows for triggered pipeline
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig \
  get workflows -n argo-workflows -l workflows.argoproj.io/workflow-template=pdftract-ci

# 3. Verify release was created
gh release view v0.1.0-test --repo jedarden/pdftract
```

**Authorization required:**
- This pushes to the public GitHub repository
- Creates a public, visible release artifact
- Uses live OpenBao credentials
- Should ONLY be done by the maintainer

## Current Blockers

### 1. Credential Expiry (iad-ci kubeconfig)
The previous attempt noted credential expiry:
```bash
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig get workflowtemplate -n argo-workflows
# Error: the server has asked the client to provide credentials
```

**Status:** Unknown if resolved - needs verification

### 2. No GitHub Release
Even if the pipeline ran, there's no evidence of a release:
```bash
$ curl -sk "https://github.com/jedarden/pdftract/releases" 
# Shows 0 releases
```

**Status:** Confirmed - no release exists

### 3. Maintainer Authorization Not Obtained
The bead description explicitly states:
> "Needs a maintainer decision to actually push a test tag against the real jedarden/pdftract GitHub repo"

**Status:** Authorization NOT obtained - requires maintainer action

## Recommended Path Forward

### For Verification (Without Public Artifact)
Use Option 1 (Manual Pipeline Test) to verify the pipeline works without requiring git tag push:

1. Submit manual workflow with `is-tag: "true"` parameter
2. Verify all pipeline stages execute successfully
3. Verify GitHub Release is created with all artifacts
4. Document the results

### For Full Phase 0 Completion (Requires Maintainer)
After Option 1 succeeds, use Option 2 to complete the verification:

1. Maintainer authorizes tag push to GitHub
2. Push `v0.1.0-test` tag to GitHub
3. Verify webhook triggers pipeline automatically
4. Verify GitHub Release creation
5. Update plan.md line 3276 to check the box

## Acceptance Criteria Verification

When the test is complete, verify the following:

- [ ] Tag `v0.1.0-test` exists on GitHub (`git ls-remote --tags github` shows it)
- [ ] Pipeline executed successfully (check Argo Workflows UI)
- [ ] GitHub Release exists (`gh release view v0.1.0-test` returns data)
- [ ] All 5 binary archives present in release
- [ ] SHA256SUMS file present and valid
- [ ] Cosign signature present (SHA256SUMS.sig)
- [ ] SLSA provenance present (multiple.intoto.jsonl)
- [ ] CycloneDX SBOM present (pdftract-v0.1.0-test.cdx.json)
- [ ] Release marked as pre-release (due to `-test` suffix)

## Summary

**Phase 0 Exit Gate Status:** ❌ UNMET

**Root Cause:** No milestone-tag test has ever successfully:
1. Been pushed to GitHub AND
2. Triggered the pipeline AND
3. Uploaded binaries to GitHub Releases

**What Exists:**
- ✅ Local tag v0.1.0-test
- ✅ Complete CI/CD pipeline infrastructure
- ✅ All build/release stages implemented

**What's Missing:**
- ❌ Tag on GitHub
- ❌ Pipeline execution on tag
- ❌ GitHub Release artifact

**Next Action:**
1. Obtain maintainer authorization for test
2. Execute Option 1 (Manual Pipeline Test) or Option 2 (Full E2E Test)
3. Verify all acceptance criteria
4. Update plan.md Phase 0 completion checklist

**Note:** This is OPS-GATED and cannot be completed without:
- Valid iad-ci credentials OR
- Maintainer authorization to push to public GitHub repo
