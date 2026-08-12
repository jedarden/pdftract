# Bead Close Verification Integration

## Overview

This document describes the rust-verify validation integration into the bead close lifecycle for pdftract NEEDLE workers. The integration ensures that beads can only close when tests pass, preventing broken code from being marked as complete.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     NEEDLE Worker                           │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  bf-close-with-verify.sh                              │ │
│  │                                                        │ │
│  │  1. Parse arguments (--skip-verify, --test-args)     │ │
│  │  2. If --skip-verify:                                 │ │
│  │     → Call bf close directly                          │ │
│  │  3. Otherwise:                                         │ │
│  │     → Call needle-verify-wrapper.sh                    │ │
│  │     → Branch push → submit → poll workflow           │ │
│  │     → If exit_code == 0: call bf close               │ │
│  │     → Else: block close, show logs                    │ │
│  └──────────────────────────────────────────────────────┘ │
│                                                              │
└─────────────────────────────────────────────────────────────┘

                    ↓

┌─────────────────────────────────────────────────────────────┐
│                Argo Workflows (iad-ci)                       │
│                                                              │
│  rust-verify WorkflowTemplate:                              │
│    - Clone repository from wip/<worker>/<bead> branch       │
│    - Run cargo test with provided args                       │
│    - Return exit code and output                            │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. bf-close-with-verify.sh

Main wrapper script that NEEDLE workers call instead of `bf close` directly.

**Usage:**
```bash
.cli/bf-close-with-verify.sh <bead-id> [close-reason] [options]
```

**Options:**
- `--skip-verify`: Bypass validation and close immediately (for infra beads)
- `--test-args`: Arguments to pass to cargo test (e.g., `-p pdftract-core --lib`)
- `--timeout`: Maximum time to wait for workflow completion (default: 1800s)

**Exit codes:**
- `0`: Bead closed successfully (validation passed or skipped)
- `1`: Validation failed or close error (bead NOT closed)

**Environment variables:**
- `KUBECONFIG`: Path to kubectl config (defaults to `~/.kube/iad-ci.kubeconfig`)
- `NEEDLE_WORKER`: Worker name (defaults to `claude-code-glm-4.7-lab-test-fix`)
- `NEEDLE_REPO`: Repository path (defaults to current directory)

### 2. needle-verify-wrapper.sh

Existing script that handles the rust-verify workflow:
- Creates wip/<worker>/<bead> branch
- Commits and pushes changes
- Submits rust-verify workflow
- Polls for completion
- Returns exit code based on test result

### 3. needle_verify.py

Python module with programmatic interface:
- `NeedleVerifier`: Main verification class
- `WorkflowSubmitter`: Direct workflow submission
- `verify_and_gate()`: Convenience function for workers

## Worker Integration

### Standard Usage

```bash
# Worker completes bead work, commits changes
git add -A
git commit -m "Implement feature X"

# Worker runs validation before close
.cli/bf-close-with-verify.sh bf-abc123 "Feature X implemented"

# If validation passes:
# → Bead closes, worker moves to next bead

# If validation fails:
# → Bead remains open, worker fixes tests and retries
```

### Infra Beads (Skip Validation)

```bash
# For beads that don't require code tests (CI config, docs, etc.)
.cli/bf-close-with-verify.sh bf-xyz789 "Updated Argo workflow" --skip-verify
```

### Custom Test Arguments

```bash
# Run specific tests only
.cli/bf-close-with-verify.sh bf-def456 "Fix parser bug" \
  --test-args "-p pdftract-core --lib test_parse"

# Run with longer timeout for slow tests
.cli/bf-close-with-verify.sh bf-ghi789 "Add integration tests" \
  --test-args "--test-threads=1" \
  --timeout 3600
```

## Workflow Lifecycle

### 1. Worker Submits Verification

```
Worker → bf-close-with-verify.sh
         ↓
       needle-verify-wrapper.sh
         ↓
       Create wip/worker/bead branch
         ↓
       git push origin
         ↓
       kubectl create workflow (rust-verify)
         ↓
       Return workflow name
```

### 2. Workflow Runs Tests

```
rust-verify workflow on iad-ci:
  - Clone repo from wip branch
  - cargo test <test-args>
  - Capture exit code and output
  - Set output parameters: result, exit_code
```

### 3. Polling and Result

```
needle-verify-wrapper.sh:
  - Poll workflow status every 10s
  - If Succeeded + exit_code == 0: return 0
  - If Failed or exit_code != 0: return 1
  - Timeout after 1800s (30 min)
```

### 4. Close Decision

```
bf-close-with-verify.sh:
  - If validation exit_code == 0: call bf close
  - Else: print logs, exit with error code
```

## Error Handling

### Validation Failure

When validation fails:

1. **Exit code**: Non-zero (blocks close)
2. **Output**: Full workflow logs printed to stderr
3. **Bead state**: Remains open (not closed)
4. **Worker action**: Fix tests, commit, retry

Example output:
```
✗ Rust-verify validation failed for bf-abc123 (exit code: 1)
Bead close blocked - fix tests and retry

To see full workflow logs, run:
  kubectl --kubeconfig=~/.kube/iad-ci.kubeconfig logs -n argo-workflows <pod-name> -c main
```

### Workflow Timeout

If workflow doesn't complete within timeout:

1. **Exit code**: 1 (blocks close)
2. **Output**: Timeout message with partial workflow output
3. **Bead state**: Remains open
4. **Worker action**: Check iad-ci cluster status, retry with longer timeout

### Git Errors

If git operations fail (push, branch creation):

1. **Exit code**: 1 (blocks close)
2. **Output**: Git error message
3. **Bead state**: Remains open
4. **Worker action**: Fix git issues (permissions, conflicts), retry

## Testing

### Run Test Suite

```bash
# Run all tests
.cli/test_bf_close_with_verify.py

# Run specific test
python3 -c "from test_bf_close_with_verify import test_skip_verify_mode; test_skip_verify_mode()"
```

### Test Coverage

The test suite (`test_bf_close_with_verify.py`) covers:

1. **Skip-verify mode**: Validation bypassed, bead closes immediately
2. **Validation failure**: Tests fail, bead remains open
3. **Validation success**: Tests pass, bead closes
4. **Invalid bead ID**: Format validation works
5. **Missing arguments**: Error handling works
6. **Custom close reason**: Reason properly passed through

## Deployment

### Lab Server

The wrapper script is deployed to `.cli/bf-close-with-verify.sh` in the pdftract repository.

NEEDLE workers call it via:
```bash
.repo/.cli/bf-close-with-verify.sh $BEAD_ID "$CLOSE_REASON"
```

### CI/CD Integration

The rust-verify WorkflowTemplate is deployed to iad-ci:
```yaml
apiVersion: argoproj.io/v1alpha1
kind: WorkflowTemplate
metadata:
  name: rust-verify
  namespace: argo-workflows
spec:
  # Template definition (see declarative-config)
```

## Troubleshooting

### Workflow Stuck in Running State

```bash
# Check workflow status
kubectl --kubeconfig=~/.kube/iad-ci.kubeconfig \
  get workflows -n argo-workflows \
  -l bead-id=$BEAD_ID,worker-name=$WORKER_NAME

# Get workflow logs
kubectl --kubeconfig=~/.kube/iad-ci.kubeconfig \
  logs -n argo-workflows <pod-name> -c main

# Delete stuck workflow
kubectl --kubeconfig=~/.kube/iad-ci.kubeconfig \
  delete workflow <workflow-name> -n argo-workflows
```

### Bead Stuck in In-Progress After Failed Close

```bash
# Check bead status
bf show <bead-id>

# Reopen if stuck
bf reopen <bead-id>

# Or force close if validation is known to pass
bf close <bead-id> --reason "Manual close after verification"
```

### Permission Errors

```bash
# Verify kubeconfig access
kubectl --kubeconfig=~/.kube/iad-ci.kubeconfig get workflows -n argo-workflows

# Verify git push permissions
git push origin wip/$WORKER_NAME/$BEAD_ID
```

## References

- Bead: `bf-zgd8q8` - Integrate rust-verify result into bead close gating
- Parent beads: See dependency tree for validation workflow implementation
- Argo Workflows: `rust-verify` WorkflowTemplate in `declarative-config`
- NEEDLE: Worker orchestration system

## Future Enhancements

1. **Parallel verification**: Run multiple beads' verification in parallel
2. **Result caching**: Skip verification if same commit already verified
3. **Partial validation**: Only run tests affected by changed files
4. **UI integration**: Show workflow status in NEEDLE dashboard
5. **Retry logic**: Automatic retry with exponential backoff on transient failures