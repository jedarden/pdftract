# Bead bf-5h60ld: Verify iad-ci cluster access and namespace

## Summary
Verified iad-ci cluster connectivity and namespace access as required for applying SealedSecret resources.

## Findings

### Kubeconfig File
- **Location**: `/home/coding/.kube/iad-ci.kubeconfig`
- **Status**: EXISTS ✓ (2809 bytes, last modified Jun 7 08:31)
- **Context**: `iad-ci` with user `argocd-manager` (ServiceAccount token)

### Cluster Connectivity Test
```bash
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig get nodes
```

**Result**: FAIL - Authentication error
```
error: You must be logged in to the server (the server has asked for the client to provide credentials)
```

### Root Cause
The kubeconfig contains a ServiceAccount JWT token for `argocd-manager`, but the token has expired. The token in the file shows:
- Issuer: `kubernetes/serviceaccount`
- Namespace: `argocd-manager`
- ServiceAccount: `argocd-manager`

ServiceAccount tokens in Kubernetes 1.24+ are time-bound and expire after a set period (typically 1 year for auto-generated tokens, but can be shorter depending on cluster configuration).

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| PASS: kubectl can connect to iad-ci cluster | ❌ FAIL | Token expired, authentication fails |
| PASS: argo-workflows namespace exists | ⚠️ UNTESTED | Cannot verify without valid authentication |
| FAIL: Cannot connect to cluster or namespace does not exist | ❌ ACTUAL FAIL | Cannot connect due to expired token |
| WARN: kubeconfig file exists but connection times out | ⚠️ PARTIAL | File exists, but error is auth failure, not timeout |

## Next Steps Required
To restore iad-ci access and proceed with bead bf-11sdod (SealedSecret deployment):
1. **Regenerate ServiceAccount token**: The `argocd-manager` ServiceAccount token in the `argocd-manager` namespace needs to be regenerated
2. **Update kubeconfig**: Replace the expired `token` field in `/home/coding/.kube/iad-ci.kubeconfig` with the new token
3. **Re-verify connectivity**: Re-run the cluster connectivity tests

## How to Regenerate the Token
The token must be generated from a cluster with admin access to iad-ci. Options:
1. From `rs-manager` cluster (which manages iad-ci): Create a temporary pod with access to generate the token
2. From iad-ci cluster directly using existing admin access
3. From the Rackspace Spot cloudspace admin interface

The token can be regenerated via:
```bash
kubectl --kubeconfig=<admin-kubeconfig> create token argocd-manager -n argocd-manager --duration=8760h
```

## Impact on Bead bf-11sdod
The parent bead (bf-11sdod) requires applying SealedSecret resources to the argo-workflows namespace in iad-ci. This cannot proceed until cluster connectivity is restored.

## Verification Timestamp
2026-08-08 19:19:53 UTC

## References
- Parent bead: bf-11sdod
- CLAUDE.md: iad-ci cluster access instructions
