# Garage Cluster Creation Blocker

**Bead:** bf-2aykgt  
**Date:** 2026-08-09  
**Task:** Create S3 bucket in Garage cluster  
**Status:** ❌ **BLOCKED - Infrastructure Failure**

## Summary

**Cannot complete bead bf-2aykgt.** The Garage cluster on apexalgo-iad is completely non-functional due to infrastructure failures. No bucket creation is possible until the Garage deployment is restored.

## Critical Blockers

### 1. Garage Pods Not Running
- **`garage-cnpg-0`**: Status=Terminating (has been terminating for 5+ hours)
- **`garage-operator`**: CrashLoopBackOff (18 restarts, 112 minutes old)
- **StatefulSet `garage-cnpg`**: 0/1 READY (no pods running)

### 2. Garage Operator Failure
The operator is crashing repeatedly with multiple errors:

**CRD Sync Errors:**
```
error: no matches for kind "GarageCluster" in version "garage.rajsingh.info/v1beta2"
error: failed to wait for garagecluster caches to sync
```

**JSON Unmarshal Errors:**
```
error: json: cannot unmarshal string into Go struct field KeyPermission.items.spec.keyPermissions.keyRef of type v1beta1.KeyRef
```

**Controller Timeouts:**
- All controllers failing to sync caches (Node, Secret, StatefulSet, Deployment, etc.)
- Manager fails to start: "problem running manager"

### 3. S3 Endpoint Inaccessible
- **Service `garage-cnpg`**: Exists but has **NO ENDPOINTS** (no backing pods)
- **S3 Endpoint**: `http://100.84.193.103:3900` - Connection refused
- **No active Garage deployment** found on any cluster

### 4. No Admin Access
- **Admin kubeconfig**: `/home/coding/.kube/apexalgo-iad.kubeconfig` does NOT exist
- Only read-only kubectl proxy access available
- Cannot perform bucket creation even if Garage was running

## Attempted Verification

```bash
# Check Garage pods
kubectl --server=http://traefik-apexalgo-iad:8001 get pods -n garage-operator
# garage-cnpg-0              0/1     Terminating        0              5h3m
# garage-operator-...         0/1     CrashLoopBackOff   18 (36s ago)   112m

# Check for Garage buckets (would show if cluster was working)
kubectl --server=http://traefik-apexalgo-iad:8001 get garagebuckets -n garage-operator
# No resources found (cannot query - operator not functional)

# Test S3 endpoint
curl -s http://100.84.193.103:3900
# Exit code 7 - connection refused
```

## Acceptance Criteria Status

All criteria **FAILED** due to infrastructure issues:

- ❌ **FAIL:** S3 bucket created successfully in Garage - **No Garage instance available**
- ❌ **FAIL:** Bucket name matches chosen name from bf-6coted - Cannot create bucket
- ❌ **FAIL:** Bucket creation response captured and saved - No creation possible
- ❌ **FAIL:** No errors in bucket creation process - Infrastructure prevents creation

## Dependencies Checked

| Dependency | Status | Notes |
|------------|--------|-------|
| bf-6coted | ✅ Closed | Bucket name determined: `sccache` |
| bf-42eo5r | ⚠️ WARN Closed | Documented same Garage failure - infrastructure investigation needed |

## Root Cause Analysis

The Garage deployment is experiencing **complete operational failure**:

1. **Operator Crash Loop**: The garage-operator cannot start due to CRD sync issues
2. **CRD Schema Mismatch**: CRD definitions or custom resources may have incompatible schema changes
3. **StatefulSet Stuck**: garage-cnpg-0 is stuck in Terminating state (possibly finalizer issues)
4. **No Recovery Path**: No admin access to intervene and fix the deployment

## Required Actions Before Bead Can Proceed

1. **Investigate Garage deployment status**
   - Why is garage-operator in CrashLoopBackOff?
   - Why is garage-cnpg-0 stuck terminating?
   - Was this intentional migration or infrastructure failure?

2. **Fix or restore Garage cluster**
   - Resolve CRD sync issues
   - Fix operator crash loop
   - Remove stuck pod if needed
   - Verify cluster health

3. **Obtain admin access**
   - Get or create `/home/coding/.kube/apexalgo-iad.kubeconfig`
   - Verify admin permissions for Garage operations

4. **Verify S3 endpoint accessibility**
   - Confirm garage-cnpg service has endpoints
   - Test S3 endpoint connectivity
   - Verify bucket creation API is accessible

## Alternative Approaches

If Garage is deprecated or cannot be restored:

1. **Use different S3-compatible backend for sccache**
   - Consider MinIO deployment on iad-ci
   - Use B2, Wasabi, or other S3-compatible service
   - Local cache on iad-ci workers

2. **Wait for Garage migration**
   - If this is planned infrastructure migration, wait for new deployment
   - Update documentation once new endpoint is known

## Technical Details

**Cluster:** apexalgo-iad  
**Namespace:** garage-operator  
**Service:** garage-cnpg (ClusterIP: 10.21.204.105:3900) - **NO ENDPOINTS**  
**StatefulSet:** garage-cnpg (0/1 READY)  
**Deployment:** garage-operator (0/1 READY)  

**Bucket Name (from bf-6coted):** `sccache`  
**Expected Endpoint:** `http://100.84.193.103:3900` (INACCESSIBLE)

## Next Steps

This bead **cannot be completed** without infrastructure intervention. Options:

1. **Route to infrastructure team** - Garage deployment requires investigation and repair
2. **Update parent bead bf-3lpe65** - Document that sccache bucket creation is blocked
3. **Consider alternative storage** - If Garage is deprecated, pivot to different solution

## References

- Parent bead: bf-3lpe65 (sccache bucket creation - also blocked)
- Dependency verification: notes/bf-42eo5r.md (first documented Garage failure)
- Bucket name: notes/bf-6coted.md (bucket name `sccache` determined)

---

**Status:** ❌ **BLOCKED** - Requires infrastructure investigation and Garage cluster restoration before bucket creation can proceed.

**Recommendation:** Do NOT close this bead. Return to worker pool for retry after infrastructure is resolved.
