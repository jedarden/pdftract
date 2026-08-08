# Bead bf-57aisv - Verify SealedSecret resource created in cluster

## Task
Verify the SealedSecret `forgejo-ci-token` exists in the `argo-workflows` namespace in the iad-ci cluster.

## Investigation Attempted

### Command Run
```bash
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig get sealedsecret forgejo-ci-token -n argo-workflows
```

### Result
**Exit code: 1** - Authentication failed

Error message:
```
error: You must be logged in to the server (the server has asked the client to provide credentials)
```

Verbose output showed:
- HTTP 401 Unauthorized response from the API server
- Token in `/home/coding/.kube/iad-ci.kubeconfig` has expired

### Alternative Attempts

1. **Ardenone-manager kubeconfig** (`ardenone-manager-24h.kubeconfig`) - Also expired (401 error)
2. **kubectl-proxy** (`kubectl-proxy-iad-ci:8001`) - No response (proxy may not be running or not configured)

## Root Cause
The ServiceAccount token in the iad-ci kubeconfig at `/home/coding/.kube/iad-ci.kubeconfig` has expired. The token was last updated on June 7, 2025 (file date: Jun 7 08:31), and the OIDC/ServiceAccount credentials have a limited lifespan (typically ~3 days for OIDC, or may expire for ServiceAccount tokens).

## Acceptance Criteria Status

- **FAIL** - SealedSecret existence cannot be verified due to authentication failure
- **WARN** - Infrastructure issue: expired kubeconfig tokens prevent cluster access

## Resolution Path
To properly verify this bead, the iad-ci kubeconfig needs to be refreshed with valid credentials. This typically requires:
1. Regenerating the ServiceAccount token from the iad-ci cluster, OR
2. Obtaining a fresh OIDC token from the Rackspace Spot UI (if using cloudspace-admin OIDC authentication)

## Recommendation
This bead should remain **open** or be closed with **WARN status** documenting the infrastructure blocker. The SealedSecret may exist and be functioning correctly, but direct verification is currently blocked by expired credentials.

## Related
- Parent bead: bf-11sdod (apply SealedSecret to cluster)
- Cluster: iad-ci (Rackspace Spot, us-east-iad-1)
- Kubeconfig path: `/home/coding/.kube/iad-ci.kubeconfig`
- Resource: `sealedsecret/forgejo-ci-token` in namespace `argo-workflows`
