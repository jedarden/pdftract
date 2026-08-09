# sccache Bucket Creation in Garage

**Bead:** bf-3lpe65  
**Date:** 2026-08-09  
**Cluster:** apexalgo-iad (Rackspace Spot us-east-iad-1)

## Summary

Attempted to create the `sccache` S3 bucket in Garage on apexalgo-iad cluster. **Blocked on admin access.**

## Current State

### Prerequisites Completed
- ✅ **Bucket name determined**: `sccache` (from bf-6coted)
- ✅ **Garage deployment verified**: Running on apexalgo-iad (from bf-3ucuqi)
- ✅ **GarageBucket manifest prepared**: `.cli/tmp/sccache-garage-bucket.yml`

### Blocker Identified
- ❌ **Admin access to apexalgo-iad not available**
  - Read-only proxy exists: `kubectl --server=http://traefik-apexalgo-iad:8001`
  - Admin kubeconfig missing: `/home/coding/.kube/apexalgo-iad.kubeconfig` (does not exist)
  - Admin kubeconfig requires cloudspace-admin OIDC token (regenerate from Rackspace Spot UI)

## What Needs to Be Done

### Step 1: Obtain Admin Access to apexalgo-iad
```bash
# From Rackspace Spot UI:
# 1. Navigate to apexalgo-iad cloudspace
# 2. Get cloudspace-admin OIDC token (expires ~3 days)
# 3. Save to /home/coding/.kube/apexalgo-iad.kubeconfig
```

### Step 2: Create the Bucket
Once admin access is available, apply the GarageBucket manifest:
```bash
kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
  apply -f .cli/tmp/sccache-garage-bucket.yml
```

This will create the `sccache` bucket with:
- Name: `sccache`
- Namespace: `garage-operator`
- Cluster: `garage-cnpg`
- Quota: 10Gi max size
- Permissions: Read+Write via `sccache-s3-key`

### Step 3: Verify Creation
```bash
# List buckets to confirm creation
kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
  get garagebuckets -n garage-operator

# Should show: sccache
```

## Acceptance Criteria Status

- ❌ **FAIL**: S3 bucket created successfully in Garage
- ❌ **FAIL**: Bucket creation confirmed via Garage CLI/API
- ⏸️ **BLOCKED**: Waiting for admin access to apexalgo-iad

## Technical Details

### GarageBucket Manifest Location
`.cli/tmp/sccache-garage-bucket.yml`

```yaml
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

### Alternative Creation Methods
If GarageBucket CRD doesn't work, alternatives include:
1. **Garage CLI**: `garage bucket create sccache` (requires Garage CLI installation)
2. **Direct S3 API**: Using s3cmd or AWS CLI with Garage admin credentials
3. **kubectl exec**: Run commands inside Garage pod directly

## Next Steps

This bead (bf-3lpe65) is **blocked on admin access**. Options:
1. **Manual intervention**: Obtain admin kubeconfig from Rackspace Spot UI
2. **Alternative cluster**: Check if Garage is available on a cluster with admin access
3. **Automation**: Set up automated token refresh for admin kubeconfig

## Issues Found

- **BLOCKER**: No admin access to apexalgo-iad cluster
- **WARN**: Garage operator on apexalgo-iad in CrashLoopBackOff (pods still serving)
- **WARN**: Admin kubeconfig requires periodic renewal (3-day expiry)

## References

- Parent bead: bf-123uxh (Create sccache S3 bucket in Garage)
- Depends on: bf-6coted (bucket name determination)
- Access verification: notes/bf-3ucuqi.md
- Bucket name decision: notes/bf-6coted.md
- GarageBucket manifest: .cli/tmp/sccache-garage-bucket.yml
- CLAUDE.md: apexalgo-iad access instructions

---
**Status**: BLOCKED - Requires admin kubeconfig for apexalgo-iad to proceed with bucket creation.
