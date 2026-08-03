# bf-1toqu8: Fix Stuck pdftract-nightly-fuzz Workflows

## Summary
Fixed stuck pdftract-nightly-fuzz Argo workflows that were hanging at 0/1 progress across multiple days. Root cause was missing workflow-level timeout guards and outdated declarative-config template.

## Changes Made

### 1. Fixed Timeout Guards in pdftract/.ci/argo-workflows/pdftract-nightly-fuzz.yaml
- **Added workflow-level activeDeadlineSeconds**: 86400 seconds (24 hours absolute max)
  - Prevents workflows from running indefinitely
  - Enforces timeout at the workflowSpec level, not just template level
- **Increased fuzz-matrix timeout**: 21600 → 28800 seconds (6 → 8 hours)
  - Allows more overhead for parallel target execution
- **Added workflowMetadata labels**: Better observability in Argo UI

### 2. Synced Improved Template to declarative-config
- Replaced outdated sequential version with improved parallel version
- **Key improvements in synced version**:
  - Parallel fuzz target execution (5 targets run concurrently, not sequentially)
  - Cgroup v2/v1 memory enforcement with proper cleanup
  - Layered memory limits: cgroup MemoryMax (1536 MB) + libfuzzer RSS/malloc limits (1024 MB)
  - More sophisticated crash artifact handling
  - Better exit handlers and reporting

### 3. Timeout Hierarchy (from coarsest to finest)
```
workflowSpec.activeDeadlineSeconds: 86400s (24h)  ← NEW: Workflow-level abort
  ├─ fuzz-matrix.activeDeadlineSeconds: 28800s (8h) ← INCREASED: DAG timeout
  │   └─ fuzz-target.activeDeadlineSeconds: 21600s (6h) ← Per-target max
  │       └─ libfuzzer -max_total_time: 17400s (4.8h) ← Fuzzing budget
  ├─ setup.activeDeadlineSeconds: 600s (10m)
  └─ seed-corpus.activeDeadlineSeconds: 300s (5m)
```

## Manual Cleanup Required

The kubectl credential check failed with authentication errors, so I could not delete the stuck workflow instances. The following manual steps are required:

### Option A: Via kubectl (preferred - credential must be refreshed)
```bash
# Delete stuck workflow instances
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig delete workflow \
  pdftract-nightly-fuzz-1784174400 \
  pdftract-nightly-fuzz-1784260800 \
  pdftract-nightly-fuzz-1784347200 \
  pdftract-nightly-fuzz-1784433600 \
  pdftract-nightly-fuzz-1784520000 \
  -n argo-workflows

# Verify all stuck instances are gone
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig get workflows -n argo-workflows | grep pdftract-nightly-fuzz
```

### Option B: Via Argo UI (alternative if kubectl credentials fail)
1. Access Argo UI at `https://argo-ci.ardenone.com` (Tailscale VPN only)
2. Navigate to the `argo-workflows` namespace
3. Select and delete each stuck workflow instance:
   - pdftract-nightly-fuzz-1784174400
   - pdftract-nightly-fuzz-1784260800
   - pdftract-nightly-fuzz-1784347200
   - pdftract-nightly-fuzz-1784433600
   - pdftract-nightly-fuzz-1784520000

### Option C: Refresh credentials first
The iad-ci.kubeconfig token appears to have expired (kubectl returned "server has asked the client to provide credentials"). To refresh:
1. Check if there's a renewal process in ~/CLAUDE.md or cluster documentation
2. The kubeconfig uses ServiceAccount `argocd-manager` with cluster-admin access
3. Token may need to be regenerated from the cluster's secret management system

## Verification

After manual cleanup and ArgoCD sync, verify:

1. **ArgoCD sync completes successfully**:
   ```bash
   # Check ArgoCD app status
   curl -sk https://argocd-ro-ardenone-manager-ts.ardenone.com:8444/api/v1/applications/argo-workflows-ns-iad-ci
   ```

2. **Next scheduled run completes or fails cleanly** (not stuck at 0/1):
   - CronWorkflow runs at 0400 UTC daily
   - Check next morning: `kubectl get workflows -n argo-workflows | grep pdftract-nightly-fuzz`
   - Status should be `Completed` or `Failed`, not `Running` with 0/1 progress

3. **Workflow-level timeout enforces**:
   - If a workflow still hangs, it will be killed after 24 hours by `activeDeadlineSeconds`
   - Status will show `Failed` with message about deadline exceeded

## Acceptance Criteria Status

- [x] Diagnose root cause (missing workflow-level timeout, outdated declarative-config template)
- [x] Add explicit activeDeadlineSeconds to workflowSpec level (24 hours)
- [x] Sync improved in-tree template to declarative-config
- [ ] Terminate current stuck instances (requires kubectl credential refresh - manual action needed)
- [ ] Verify next scheduled run completes cleanly (wait for 0400 UTC tomorrow)

## WARN Items

- **kubectl credential expired**: Could not delete stuck workflow instances or verify live cluster state. Manual cleanup required after credential refresh.
- **No live pod inspection performed**: Due to credential issue, could not capture pod logs from stuck workflows before GC deleted them (podGC: OnPodCompletion means pods vanish on step completion).

## References

- Bead description: bf-1toqu8
- Plan: docs/plan/plan.md (Tier 5 gate: Pre-Release Go/No-Go Checklist)
- Argo Workflows docs: https://argoproj.github.io/argo-workflows/cron-workflows/
