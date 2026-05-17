# Phase 0.1: pdftract-ci WorkflowTemplate Scaffolding - Status

## Summary

The `pdftract-ci.yaml` WorkflowTemplate scaffold was already created and committed in `jedarden/declarative-config` at commit `8248a1f` with message "feat(ci): add pdftract-ci WorkflowTemplate scaffolding".

## File Location

`jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-ci.yaml`

## Scaffold Components (All Present)

1. **Webhook Payload Schema**: Documented in comments at top of YAML
2. **WorkflowTemplate Metadata**: 
   - name: pdftract-ci
   - namespace: argo-workflows
   - labels: app.kubernetes.io/name, component, part-of
3. **Parameters**: commit-sha, ref, repo-url, is-tag
4. **DAG Structure**: setup -> [parallel: build-matrix, test-matrix, quality-matrix, bench-matrix] -> publish-if-tag
5. **Security Context**: runAsNonRoot: true, runAsUser: 1000, fsGroup: 1000
6. **Service Account**: argo-workflow
7. **VolumeClaimTemplates**: cargo-cache PVC (50Gi, sata-large StorageClass)
8. **Pod Metadata**: Labels including commit-sha
9. **Docker Config**: docker-hub-registry secret reference
10. **Empty Step Skeletons**: All templates have placeholder containers that exit 0

## Templates Defined

- pipeline (top-level DAG)
- on-exit (exit handler)
- setup (placeholder)
- build-matrix (placeholder)
- test-matrix (placeholder)
- quality-matrix (placeholder)
- bench-matrix (placeholder)
- publish-if-tag (placeholder)

## Current Status

| Criteria | Status |
|----------|--------|
| File exists in declarative-config | ✅ Yes |
| YAML syntax valid | ✅ Yes |
| Committed to git | ✅ Yes (8248a1f) |
| ArgoCD sync status | ⏳ OutOfSync (Healthy) |
| WorkflowTemplate in cluster | ❌ Not synced yet |
| Manual workflow test | ❌ Not possible without sync |

## ArgoCD Sync Pending

The ArgoCD application `argo-workflows-ns-iad-ci` is currently `OutOfSync` but `Healthy`. The WorkflowTemplate will appear in the cluster once ArgoCD syncs the changes. This requires:
- Either automated sync (not currently enabled based on OutOfSync status)
- Or manual sync by someone with write access to ArgoCD

The read-only kubectl-proxy at `http://traefik-rs-manager:8001` cannot trigger the sync.

## Next Steps for Sibling Beads

Once ArgoCD syncs, sibling beads can develop each DAG leg in parallel by editing their respective template sections:
- pdftract-xxxx: setup step implementation
- pdftract-yyyy: build-matrix implementation
- pdftract-zzzz: test-matrix implementation
- pdftract-wwww: quality-matrix implementation
- pdftract-vvvv: bench-matrix implementation
- pdftract-uuuu: publish-if-tag implementation

Each scaffold template has volumeMounts for cargo-cache and appropriate resource limits defined.
