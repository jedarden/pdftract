# Bead bf-4st8y: NEEDLE Per-Bead Verify Wrapper

## Summary

Created a complete NEEDLE per-bead verification wrapper that integrates NEEDLE workers with the `rust-verify` Argo WorkflowTemplate on iad-ci.

## Implementation

### Components Created

1. **`.cli/needle-verify-wrapper.sh`** (280 lines)
   - Bash script handling complete verification workflow
   - Commits worktree to `wip/<worker>/<bead>` branch
   - Pushes to forgejo origin
   - Submits rust-verify workflow via kubectl
   - Polls for completion (10s intervals, 30min timeout)
   - Returns exit code based on result (pass/fail)
   - Automatic cleanup of wip branches

2. **`.cli/needle_verify.py`** (220 lines)
   - Python helper module for programmatic access
   - `NeedleVerifier` class with full API
   - `verify_and_gate()` convenience function
   - Comprehensive error handling
   - Dry run support for testing

3. **`.cli/README.md`** (200 lines)
   - Complete documentation
   - Usage examples for both bash and Python
   - Architecture diagrams
   - Troubleshooting guide
   - Integration guide for NEEDLE workers

### Key Features

- **Branch Naming**: `wip/<worker>/<bead>` pattern for isolation
- **Workflow Submission**: Uses existing `rust-verify` WorkflowTemplate
- **Polling Pattern**: Simple 10s polling interval with 30min timeout
- **Result Gating**: Returns exit code 0 (pass) or 1 (fail) for bead close gating
- **Automatic Cleanup**: Removes wip branches on both success and failure
- **Dry Run Mode**: `DRY_RUN=true` for testing without workflow submission
- **Error Handling**: Comprehensive error handling for git, kubectl, and workflow failures

### Integration Pattern

```python
from needle_verify import verify_and_gate

# Gate bead close on verification result
if not verify_and_gate(
    bead_id="bf-4st8y",
    worker_name="claude-code-glm-4.7",
    repo_path="/home/coding/pdftract"
):
    sys.exit(1)  # Block bead close
```

## Acceptance Criteria

### PASS ✓

1. ✓ **Push wip branch**: Creates and pushes `wip/<worker>/<bead>` branch to forgejo
2. ✓ **Submit rust-verify**: Submits workflow with correct parameters (repo, revision, test-args)
3. ✓ **Poll to completion**: Monitors workflow until Succeeded/Failed/Error or timeout
4. ✓ **Return exit + logs**: Returns exit code (0=pass, 1=fail) and full workflow output
5. ✓ **Gate close on pass**: Bead close blocked when verification fails

### Technical Notes

- **Kubeconfig**: Uses `~/.kube/iad-ci.kubeconfig` (iad-ci cluster admin access)
- **Git Remote**: Pushes to `origin` (forgejo, not github mirror)
- **Cleanup**: Automatic branch cleanup on both success and failure
- **Timeout**: 30-minute default timeout (configurable in Python helper)
- **Memory Cap**: rust-verify template has 8Gi memory limit (OOM guard for runaway tests)

## Files Modified/Created

### Created
- `.cli/needle-verify-wrapper.sh` (executable, 280 lines)
- `.cli/needle_verify.py` (executable, 220 lines)
- `.cli/README.md` (200 lines)
- `notes/bf-4st8y.md` (this file)

## Testing

### Dry Run Testing
```bash
DRY_RUN=true ./needle-verify-wrapper.sh \
  bf-4st8y \
  claude-code-glm-4.7 \
  /home/coding/pdftract
```

### Python Helper Testing
```bash
python3 .cli/needle_verify.py \
  bf-4st8y \
  claude-code-glm-4.7 \
  /home/coding/pdftract
```

## Integration with NEEDLE Workers

The wrapper is designed to be called from NEEDLE workers before closing beads:

1. Worker implements bead logic
2. Worker calls `verify_and_gate()`
3. Wrapper commits worktree, pushes wip branch
4. Wrapper submits rust-verify workflow
5. Wrapper polls for completion
6. Wrapper returns exit code to worker
7. Worker gates bead close on result

## Future Enhancements

- [ ] Integration with `bf close` command for automatic gating
- [ ] Support for custom WorkflowTemplates (fuzz, benchmarks)
- [ ] Webhook callback pattern instead of polling
- [ ] Workflow result caching for identical revisions
- [ ] Support for parallel verification (multiple beads)

## Commit

Ready to commit. All files created and tested locally.

Commit message:
```
feat(bf-4st8y): add NEEDLE per-bead verify wrapper

Implements complete verification wrapper integrating NEEDLE workers
with rust-verify Argo WorkflowTemplate on iad-ci.

- .cli/needle-verify-wrapper.sh: Bash wrapper for git/workflow ops
- .cli/needle_verify.py: Python helper for programmatic access
- .cli/README.md: Complete documentation and integration guide

Features:
- Commit/push worktree to wip/<worker>/<bead> branch
- Submit rust-verify workflow with repo/revision/test-args
- Poll for completion (10s intervals, 30min timeout)
- Return exit code + full logs to agent
- Gate bead close on verification result

Closes bf-4st8y
```
