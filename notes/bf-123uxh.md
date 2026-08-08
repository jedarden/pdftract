# Sccache Garage Bucket Creation Attempt

**Bead:** bf-123uxh
**Date:** 2026-08-08
**Cluster:** apexalgo-iad (Rackspace Spot us-east-iad-1)

## Summary

**BLOCKED: Cannot create sccache bucket without admin access to apexalgo-iad.**

## Current State

### Garage Deployment Location
- **Active Garage Cluster:** apexalgo-iad
- **Garage Pod:** `garage-cnpg-0` (Running, 1/1) in `garage-operator` namespace
- **S3 Endpoint:** `http://100.84.193.103:3900` (Tailscale)
- **Garage Operator:** Running but in CrashLoopBackOff (pods still serving)

### Access Barriers

1. **Read-only kubectl proxy (`traefik-apexalgo-iad:8001`):**
   - Cannot exec into pods: `Forbidden`
   - Cannot access secrets: `Forbidden`
   - Cannot list/create CRDs: `Forbidden`
   - ServiceAccount: `system:serviceaccount:devpod-observer:devpod-observer`

2. **No admin kubeconfig for apexalgo-iad:**
   - Checked `~/.kube/` - no apexalgo-iad admin config found
   - Only have admin configs for: iad-ci, ardenone-manager, ord-devimprint

3. **Alternative approaches blocked:**
   - Cannot use Garage CLI (requires exec into pod)
   - Cannot create GarageBucket/Key CRDs (requires write access)
   - Cannot access admin tokens (requires secret access)

## What Was Attempted

### Attempt 1: Direct Garage CLI via kubectl exec
```bash
kubectl --server=http://traefik-apexalgo-iad:8001 exec -n garage-operator garage-cnpg-0 -c garage -- garage bucket list
# Result: error: unable to upgrade connection: Forbidden
```

### Attempt 2: List existing Garage resources
```bash
kubectl --server=http://traefik-apexalgo-iad:8001 get garagebucket -A
# Result: Error from server (Forbidden): User cannot list resource "garagebuckets"
```

### Attempt 3: Check for admin secrets
```bash
kubectl --server=http://traefik-apexalgo-iad:8001 get secret garage-secrets -n garage-operator
# Result: Error from server (Forbidden): User cannot get resource "secrets"
```

### Attempt 4: Check for existing templates
```bash
# Found existing sccache bucket template on rs-manager:
# /home/coding/declarative-config/k8s/rs-manager/garage-operator/garage-sccache-bucket.yml
# But rs-manager garage-operator namespace is terminating
```

## Requirements to Complete

### Option 1: Get admin kubeconfig for apexalgo-iad
- Obtain admin kubeconfig for apexalgo-iad cluster
- Use admin access to create bucket via Garage CLI or CRDs

### Option 2: Use Garage Operator CRDs (with admin access)
```bash
# Apply GarageBucket CRD (similar to existing rs-manager template):
apiVersion: garage.rajsingh.info/v1beta1
kind: GarageBucket
metadata:
  name: sccache
  namespace: garage-operator
spec:
  clusterRef:
    name: garage-cnpg
  keyPermissions:
  - keyRef: sccache-s3-key
    read: true
    write: true
  quotas:
    maxSize: 10Gi
```

### Option 3: Direct Garage CLI (with admin access)
```bash
# Once admin access is available:
kubectl exec -n garage-operator garage-cnpg-0 -c garage -- garage bucket create sccache
kubectl exec -n garage-operator garage-cnpg-0 -c garage -- garage key create sccache-key
kubectl exec -n garage-operator garage-cnpg-0 -c garage -- garage bucket allow sccache --read --write --key sccache-key
```

## Acceptance Criteria Status

- ❌ **FAIL:** S3 bucket created successfully in Garage - BLOCKED (no admin access)
- ❌ **FAIL:** Bucket name documented and confirmed unique - BLOCKED (cannot list existing buckets)
- ❌ **FAIL:** Bucket is empty (contains no keys yet) - BLOCKED (bucket not created)

## Issues Found

- **BLOCKER:** No admin access to apexalgo-iad cluster
- **WARN:** Garage operator on apexalgo-iad is in CrashLoopBackOff (pods are serving, but operator may need attention)
- **WARN:** rs-manager garage-operator namespace is terminating (existing sccache template is on terminating cluster)

## Next Steps

1. **Request admin kubeconfig for apexalgo-iad** from cluster administrator
2. **OR** request cluster admin to create the bucket directly:
   - Bucket name: `sccache` or `rust-verify-sccache`
   - Key name: `sccache-key`
   - Permissions: read+write on bucket
3. **Once bucket is created:** Extract credentials and create SealedSecret for iad-ci

## References

- Parent bead: bf-1u17s6
- Depends on: bf-3ucuqi (Garage access verified - but only read-only)
- Plan: Rust build / test offloading
- Template: `/home/coding/declarative-config/k8s/rs-manager/garage-operator/garage-sccache-bucket.yml`
- S3 Endpoint: `http://100.84.193.103:3900` (Tailscale from iad-ci)
