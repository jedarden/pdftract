# sccache Bucket Creation - INFRASTRUCTURE BLOCKER

**Bead:** bf-3lpe65  
**Date:** 2026-08-09  
**Status:** BLOCKED - Garage Infrastructure Broken

## Summary

Cannot create sccache S3 bucket in Garage cluster due to infrastructure failure. Garage deployment on apexalgo-iad is non-functional.

## Current Infrastructure State

### apexalgo-iad Cluster (Target Cluster)

**Garage Pod Status:**
- Pod: `garage-cnpg-0` - Status: `Terminating` (0/1)
- Operator: `garage-operator-567bc7676f-4tnf7` - Status: `CrashLoopBackOff` (21 restarts, 133m old)
- Namespace: `garage-operator` - Status: Active (but resources broken)

**Admin Access:**
- ❌ `/home/coding/.kube/apexalgo-iad.kubeconfig` - DOES NOT EXIST
- ❌ Read-only proxy (`traefik-apexalgo-iad:8001`) - Cannot create resources/exec into pods (Forbidden)

### rs-manager Cluster (Legacy Garage)

- Namespace: `garage-operator` - Status: `Terminating` (115 days - likely abandoned)
- No functional Garage deployment

### iad-ci Cluster (Consuming Cluster)

- Kubeconfig exists but has auth issues
- Target cluster for sccache usage, but Garage is not deployed here

## Bucket Name (Verified)

✅ **Bucket name: `sccache`** (determined in bf-6coted)
- Simple, clear, follows S3 naming conventions
- Verified unique against existing buckets (only `openbao` on rs-manager)
- Ready for use once Garage is functional

## What I Attempted

### 1. Admin Kubeconfig Check
```bash
$ test -f /home/coding/.kube/apexalgo-iad.kubeconfig
DOES NOT EXIST
```

### 2. Garage Pod CLI Access
```bash
$ kubectl --server=http://traefik-apexalgo-iad:8001 exec -n garage-operator garage-cnpg-0 -c garage -- garage --help
error: unable to upgrade connection: Forbidden
```
- Read-only proxy forbids exec access
- Pod is in "Terminating" state anyway

### 3. YAML Manifest Application
Prepared YAML files exist in `.cli/tmp/`:
- `sccache-garage-bucket.yml` - GarageBucket definition for `sccache`
- `sccache-garage-key.yml` - GarageKey definition for `sccache-s3-key`

❌ Cannot apply due to:
- No admin access to apexalgo-iad
- Garage operator non-functional (CrashLoopBackOff)
- No active Garage cluster to accept resources

### 4. Alternative Cluster Check
```bash
$ kubectl --server=http://traefik-rs-manager:8001 get namespaces | grep garage
garage-operator             Terminating   115d
```
- rs-manager Garage is also in long-term termination

## Root Cause Analysis

### Why Garage is Broken on apexalgo-iad

**Garage Operator Status:**
- 21 restarts in 133 minutes (≈6.3 minutes per crash cycle)
- This indicates a persistent configuration or dependency issue

**Possible Causes:**
1. Storage backend unavailable (CNPG database issue)
2. Configuration mismatch in GarageCluster CRD
3. Resource constraints or missing dependencies
4. Certificate/TLS issues
5. Database connection problems

**Impact:**
- Cannot create S3 buckets
- Cannot manage Garage keys
- Existing buckets may be inaccessible
- sccache deployment completely blocked

## Prepared Artifacts

### YAML Manifests (Ready for Use)

**File:** `/home/coding/pdftract/.cli/tmp/sccache-garage-bucket.yml`
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
```

**File:** `/home/coding/pdftract/.cli/tmp/sccache-garage-key.yml`
```yaml
apiVersion: garage.rajsingh.info/v1beta1
kind: GarageKey
metadata:
  name: sccache-s3-key
  namespace: garage-operator
spec:
  clusterRef:
    name: garage-cnpg
```

### Environment Configuration

When bucket is created, sccache will use:
```bash
export SCCACHE_BUCKET=sccache
export SCCACHE_ENDPOINT=http://100.84.193.103:3900
export SCCACHE_REGION=us-east-iad-1
export SCCACHE_S3_USE_CREDENTIALS_PROVIDER=true
```

## Acceptance Criteria Status

- ❌ **FAIL:** S3 bucket created successfully in Garage (BLOCKED - infrastructure broken)
- ✅ **PASS:** Bucket name matches chosen name from child 1 (`sccache`)
- ❌ **FAIL:** Bucket creation confirmed via Garage CLI/API (BLOCKED - no access)

## Required Actions to Unblock

### Immediate (Must happen first)

1. **Repair Garage deployment on apexalgo-iad:**
   - Investigate garage-operator CrashLoopBackOff cause
   - Fix underlying CNPG database or configuration issue
   - Restore Garage to functional state

2. **OR obtain admin kubeconfig for apexalgo-iad:**
   - Regenerate from Rackspace Spot UI (expires ~3 days)
   - Save to `/home/coding/.kube/apexalgo-iad.kubeconfig`
   - Use for direct Garage CLI access

### Once Garage is Functional

3. **Apply prepared YAML manifests**
4. **Create SealedSecret for iad-ci**
5. **Verify bucket creation**

## Alternative Approach

Consider deploying Garage on iad-ci cluster itself (where we have admin access).

## References

- Parent bead: bf-123uxh
- Depends on: bf-6coted (bucket name: `sccache`)
- Infrastructure verification: `notes/bf-3ucuqi.md`
- Bucket name determination: `notes/bf-6coted.md`

---

**BLOCKER VERIFIED:** sccache bucket creation is blocked on non-functional Garage infrastructure.
