# bf-5wcyer: Apply forgejo-ci-token SealedSecret to iad-ci cluster

## Task Summary
Apply the SealedSecret manifest to iad-ci cluster so rust-verify can authenticate to git.ardenone.com.

## Current Status
**BLOCKED** - Cannot complete due to missing iad-ci kubeconfig

## Investigation Results

### What Exists
1. **SealedSecret manifest**: Located at `/home/coding/declarative-config/k8s/iad-ci/external-secrets/forgejo-ci-token.yaml`
   - Contains encrypted token for forgejo-ci authentication
   - Namespace: argo-workflows
   - Secret name: forgejo-ci-token

2. **sealed-secrets controller**: Running on iad-ci cluster
   - Deployment: `sealed-secrets-iad-ci` in `sealed-secrets` namespace
   - Status: 1/1 ready, 134 days old
   - Will decrypt SealedSecret and create the actual Secret

3. **external-secrets infrastructure**: Exists on iad-ci
   - Namespace: external-secrets (Active, 134 days old)
   - Application: external-secrets-iad-ci managed by ArgoCD

### Blocker
**Missing kubeconfig**: The iad-ci kubeconfig referenced in CLAUDE.md (`/home/coding/.kube/iad-ci.kubeconfig`) does not exist on this system.

- Attempted kubectl operations fail with: "stat /home/coding/.kube/iad-ci.kubeconfig: no such file or directory"
- Read-only proxy access (traefik-iad-ci:8001) has insufficient RBAC to manage sealed-secrets
- Alternative kubeconfigs checked (iad-acb, ardenone-manager-24h, ord-devimprint*) are for different clusters

## What Would Need to Be Done

### Step 1: Obtain iad-ci kubeconfig
The kubeconfig for iad-ci needs to be obtained/recreated. According to CLAUDE.md:
- ServiceAccount: `argocd-manager` with cluster-admin access
- Cluster: Rackspace Spot cluster in us-east-iad-1
- Direct kubeconfig (not proxied)

**Action required**: Regenerate or retrieve the iad-ci kubeconfig and place it at `/home/coding/.kube/iad-ci.kubeconfig`

### Step 2: Apply the SealedSecret
Once kubeconfig is available:
```bash
kubectl --kubeconfig /home/coding/.kube/iad-ci.kubeconfig apply -f /home/coding/declarative-config/k8s/iad-ci/external-secrets/forgejo-ci-token.yaml
```

Expected output: `sealedsecret.bitnami.com/forgejo-ci-token created`

### Step 3: Verify Secret Creation
The sealed-secrets controller will decrypt the SealedSecret and create the actual Secret. Wait a few seconds for the controller to process, then verify:

```bash
kubectl --kubeconfig /home/coding/.kube/iad-ci.kubeconfig get secret forgejo-ci-token -n argo-workflows
```

Expected output should show:
- Name: forgejo-ci-token
- Namespace: argo-workflows
- Type: Opaque (or appropriate)
- Age: < few seconds

### Step 4: Decode and Verify Token
```bash
kubectl --kubeconfig /home/coding/.kube/iad-ci.kubeconfig get secret forgejo-ci-token -n argo-workflows -o jsonpath='{.data.token}' | base64 -d
```

This should decode to a valid forgejo API token.

## Acceptance Criteria Status

### PASS
- [x] SealedSecret manifest exists at correct location
- [x] sealed-secrets controller is running on iad-ci
- [x] external-secrets infrastructure is in place

### BLOCKED
- [ ] kubectl apply succeeds without error - **BLOCKED: No iad-ci kubeconfig**
- [ ] Secret forgejo-ci-token exists in argo-workflows namespace - **BLOCKED: Cannot verify**
- [ ] Secret data contains valid token (base64-decodable) - **BLOCKED: Cannot verify**
- [ ] Secret type is Opaque (or appropriate) - **BLOCKED: Cannot verify**

## Notes

### ArgoCD Management Consideration
The task description notes: "DO NOT use kubectl on ArgoCD-managed resources — this is a one-time secret deployment"

This indicates the SealedSecret is meant to be a one-time deployment, not continuously managed by ArgoCD. The sealed-secrets controller will handle the actual Secret creation and ongoing management.

### Alternative Approach
If direct kubectl apply continues to be blocked, consider:
1. Creating the Secret directly via ArgoCD sync (if appropriate for this use case)
2. Using external-secrets operator instead of sealed-secrets
3. Manually creating the Secret in the cluster (if credentials are available)

## Related Files
- SealedSecret manifest: `/home/coding/declarative-config/k8s/iad-ci/external-secrets/forgejo-ci-token.yaml`
- External secrets application: `/home/coding/declarative-config/k8s/iad-ci/external-secrets/external-secrets-application.yml`
- CLAUDE.md reference: iad-ci cluster section

## Resolution Required
This task cannot be completed until the iad-ci kubeconfig is available. The kubeconfig needs to be obtained from the Rackspace Spot cluster credentials or regenerated from the cluster access configuration.
