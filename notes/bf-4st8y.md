# NEEDLE Per-Bead Verify Wrapper - Implementation Notes

**Bead:** bf-4st8y
**Date:** 2026-08-06
**Status:** ✅ Complete

## Overview

Built the per-bead verification wrapper that integrates NEEDLE workers with the `rust-verify` WorkflowTemplate on iad-ci. This enables remote verification of bead work before closing, preventing broken commits from being accepted.

## Components Implemented

### 1. Core Verification Wrapper: `~/bin/needle-verify-wrapper.sh`

Main script that implements the full verification lifecycle:

**Features:**
- **Git branch management**: Creates/pushes `wip/<worker>/<bead>` branches
- **Workflow submission**: Submits rust-verify WorkflowTemplate with proper parameters
- **Polling logic**: Waits for workflow completion with 10-minute timeout
- **Result extraction**: Captures workflow output parameters and logs
- **Exit code handling**: Returns 0 on pass, 1 on fail

**Parameters:**
- `bead-id`: The bead to verify (required)
- `worker-name`: Worker identifier (default: claude-code-glm-4.7)
- `repo-path`: Git repository path (default: current directory)
- `test-args`: Optional cargo test arguments

**Environment variables:**
- `KUBECONFIG`: Path to kubectl config (default: `/home/coding/.kube/iad-ci.kubeconfig`)
- `GIT_REMOTE`: Git remote name (default: `origin`)
- `DRY_RUN`: Set to "true" for testing without submission

### 2. Integration Hook: `~/bin/needle-verify-integration.sh`

Lightweight wrapper for NEEDLE lifecycle integration:

- Simplified interface: single `bead-id` argument
- Environment-based configuration
- Clear error messaging for failed verifications

## Architecture

The verification flow:

```
NEEDLE Worker (completes bead)
    ↓
verify-before-close hook
    ↓
needle-verify-wrapper.sh
    ├→ Create wip/<worker>/<bead> branch
    ├→ Push to git remote
    ├→ Submit rust-verify Workflow
    ├→ Poll for completion (10 min timeout)
    └→ Return result + logs
    ↓
NEEDLE (gate close on result=pass)
```

## Integration with rust-verify WorkflowTemplate

The wrapper submits a Workflow that references the `rust-verify` WorkflowTemplate with these parameters:

- **repo**: Git repository URL (github.com or git.ardenone.com)
- **revision**: Branch name (`wip/<worker>/<bead>`)
- **test-args**: Optional cargo test filters
- **builder-image**: Container image for the build

The WorkflowTemplate then:
1. Clones the repository at the specified revision
2. Runs `cargo check --all-targets` (fast fail)
3. Runs `cargo clippy --all-targets -- -D warnings`
4. Runs `cargo test` with optional args
5. Outputs result (pass/fail) and full build log

## Usage Examples

### Basic usage
```bash
~/bin/needle-verify-wrapper.sh bf-4st8y
```

### With custom worker name
```bash
~/bin/needle-verify-wrapper.sh bf-4st8y claude-opus-5
```

### With specific repository
```bash
~/bin/needle-verify-wrapper.sh bf-4st8y claude-opus-5 ~/pdftract
```

### With test arguments
```bash
~/bin/needle-verify-wrapper.sh bf-4st8y claude-opus-5 ~/pdftract "-p pdftract-core --lib"
```

### Dry run (testing without submission)
```bash
DRY_RUN=true ~/bin/needle-verify-wrapper.sh bf-4st8y
```

### Integration hook usage
```bash
~/bin/needle-verify-integration.sh bf-4st8y
```

## Testing

Test the wrapper in dry-run mode:

```bash
cd ~/pdftract
DRY_RUN=true ~/bin/needle-verify-wrapper.sh bf-4st8y
# Output: DRY_RUN: Would submit workflow with:
#   repo: https://git.ardenone.com/jedarden/pdftract.git
#   revision: wip/claude-code-glm-4.7/bf-4st8y
#   test-args: <none>
# DRY_RUN_SUCCESS
```

## Next Steps for Full Integration

To complete the validate-before-close lifecycle integration:

1. **Add NEEDLE hook**: Configure NEEDLE to call the integration hook before closing beads
   ```yaml
   # In ~/.needle/config.yaml or workspace .needle/config.yaml
   hooks:
     before_close:
       - path: ~/bin/needle-verify-integration.sh
         timeout: 600s
         fail_action: block
   ```

2. **Configure sccache**: Set up the sccache-garage secret in iad-ci for faster builds
   ```bash
   kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig \
     create -f - <<EOF
   apiVersion: v1
   kind: Secret
   metadata:
     name: sccache-garage
     namespace: argo-workflows
   type: Opaque
   stringData:
     bucket: sccache
     endpoint: https://s3.${SECRET_DOMAIN}
     access-key-id: ${SCCACHE_ACCESS_KEY_ID}
     secret-access-key: ${SCCACHE_SECRET_ACCESS_KEY}
   EOF
   ```

3. **Test end-to-end**: Verify the full flow from bead work to verification to close

## Benefits

- **Quality gate**: Prevents broken commits from being accepted
- **Remote execution**: Heavy cargo test runs happen on iad-ci, not lab box
- **Isolation**: Each bead verification runs in isolated pods
- **Memory safety**: OOM-prone tests kill the pod, never the lab server
- **Audit trail**: Workflows are tracked in Argo UI with full logs
- **Reproducibility**: Each bead's work is preserved in a wip branch

## References

- rust-verify WorkflowTemplate: `~/declarative-config/k8s/iad-ci/argo-workflows/rust-verify-workflowtemplate.yml`
- Argo UI: https://argo-ci.ardenone.com
- NEEDLE documentation: https://github.com/user/needle
