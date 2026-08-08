# sccache S3 Bucket Creation - Blocked on Admin Access

**Bead:** bf-123uxh  
**Date:** 2026-08-08  
**Cluster:** apexalgo-iad (Garage deployment target)

## Summary

**TASK BLOCKED** - Cannot create sccache S3 bucket without Garage admin access to apexalgo-iad cluster.

## Current State

### Garage Deployment Status
- **Garage Cluster:** apexalgo-iad (verified in parent bead bf-3ucuqi)
- **Garage Pod:** `garage-cnpg-0` (Running, 1/1) in `garage-operator` namespace
- **S3 Endpoint:** `http://100.84.193.103:3900` (Tailscale, accessible from iad-ci)
- **Status:** Garage deployment is active and serving

### Access Blocker

**Required but unavailable:** Admin kubeconfig for apexalgo-iad cluster
- Expected path: `/home/coding/.kube/apexalgo-iad.kubeconfig`
- Status: **File does not exist** (likely expired OIDC token)
- Renewal: Requires cloudspace-admin OIDC token from Rackspace Spot UI (~3 day expiry)

### Available Access
- ✅ Read-only kubectl proxy: `kubectl --server=http://traefik-apexalgo-iad:8001`
- ❌ Cannot exec into pods (Forbidden by serviceaccount)
- ❌ Cannot read secrets (Forbidden by serviceaccount)
- ❌ Cannot use Garage CLI or S3 API without credentials

### Attempted Approaches

1. **Read-only kubectl proxy** - Can view pods/namespaces but cannot create resources
2. **kubectl exec into Garage pod** - Forbidden by serviceaccount RBAC
3. **AWS CLI with S3 endpoint** - Requires credentials (no admin keys available)
4. **Direct S3 API** - Requires authentication (no keys accessible without secret read)

## What's Needed

To create the sccache bucket, one of the following is required:

### Option 1: Admin Kubeconfig (Recommended)
```bash
# Regenerate from Rackspace Spot UI
# cloudspace-admin OIDC token → apexalgo-iad.kubeconfig

# Then use Garage CLI or S3 API to create bucket:
kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
  exec -n garage-operator garage-cnpg-0 -- \
  garage bucket create sccache-cache
```

### Option 2: Existing Garage Admin Keys
If Garage admin keys exist in a secret accessible via read-only proxy:
```bash
# Would require admin to reveal or make available
aws --endpoint-url http://100.84.193.103:3900 s3 mb s3://sccache-cache
```

### Option 3: Direct Garage Admin Access
Access Garage cluster via admin API (requires cluster admin rights)

## Proposed Bucket Name

**Recommended:** `sccache-cache` or `rust-verify-sccache`
- Rationale: Clear purpose identification
- Conflict check: Cannot verify without list-bucket access

## Acceptance Criteria Status

- ❌ **FAIL:** S3 bucket created successfully in Garage (blocked on admin access)
- ❌ **FAIL:** Bucket name documented and confirmed unique (cannot list without admin access)
- ❌ **FAIL:** Bucket is empty (bucket doesn't exist yet)

## Next Steps (When Admin Access Available)

1. **Obtain admin access:** Regenerate apexalgo-iad.kubeconfig from Spot UI or obtain Garage admin keys
2. **Create bucket:**
   ```bash
   # Via Garage CLI in pod:
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     exec -n garage-operator garage-cnpg-0 -- \
     garage bucket create sccache-cache
   
   # Or via AWS CLI with Garage credentials:
   aws --endpoint-url http://100.84.193.103:3900 s3 mb s3://sccache-cache
   ```
3. **Generate S3 credentials** for sccache user (if not using existing admin keys)
4. **Create SealedSecret** in declarative-config for iad-ci workflows
5. **Verify bucket:** List and confirm empty

## References

- Parent bead: bf-1u17s6 (sccache S3 bucket setup)
- Dependency: bf-3ucuqi (Garage deployment verification)
- Template: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/sccache-garage-secret.yml.template`
- Garage Endpoint: `http://100.84.193.103:3900` (apexalgo-iad via Tailscale)

## Blocker Summary

**CRITICAL BLOCKER:** This task cannot be completed without Garage admin access to the apexalgo-iad cluster. The required kubeconfig at `/home/coding/.kube/apexalgo-iad.kubeconfig` does not exist (expired OIDC token), and read-only access via kubectl proxy is insufficient for bucket creation.

**DO NOT CLOSE THIS BEAD** - Admin access must be obtained before bucket creation can proceed.
