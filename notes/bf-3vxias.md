# sccache Bucket Verification - INFRASTRUCTURE BLOCKER

**Bead:** bf-3vxias  
**Date:** 2026-08-09  
**Status:** BLOCKED - Garage Infrastructure Still Broken

## Summary

Cannot verify sccache bucket accessibility or contents because the bucket does not exist. The parent bead (bf-3lpe65) was closed with WARN/INFRA status due to broken Garage infrastructure, and no bucket was actually created.

## Current Infrastructure State

### Garage Operator Status
- **Pod:** `garage-operator-567bc7676f-4tnf7` - Status: `Running` (1/1)
- **Improvement:** No longer CrashLoopBackOff (was 21 restarts, now stable at 22)
- **However:** Multiple CRD errors preventing bucket operations

### Garage Cluster Status  
- **Pod:** `garage-cnpg-0` - Status: `Terminating` (0/1) - 5h30m in this state
- **IP:** `<none>` (no IP assigned)
- **Node:** `prod-instance-17854688307610730` (Spot instance likely terminated)

### Critical CRD Issues Found

**From garage-operator logs:**
```
error: "no matches for kind \"GarageCluster\" in version \"garage.rajsingh.info/v1beta2\""
error: "failed to list *v1beta1.GarageBucket: json: cannot unmarshal string into Go struct field KeyPermission.items.spec.keyPermissions.keyRef of type v1beta1.KeyRef"
error: "no matches for kind \"GarageKey\" in version \"garage.rajsingh.info/v1alpha1\""
```

**Root Cause:** Multiple CRD version conflicts:
- Missing `GarageCluster` v1beta2 CRD
- Schema mismatch in existing `GarageBucket` resources
- v1alpha1/v1beta1/v1beta2 version conflicts

## Verification Attempts

### 1. S3 Endpoint Reachability
```bash
$ curl -s --connect-timeout 5 http://100.84.193.103:3900
(no output - endpoint may be down or returning empty response)
```

### 2. Bucket Listing via AWS CLI
```bash
$ aws s3 ls --endpoint-url http://100.84.193.103:3900
Unable to locate credentials.
Command timed out or failed
```

### 3. GarageBucket Resource Check
```bash
$ kubectl get GarageBucket -A
Forbidden: User "system:serviceaccount:devpod-observer:devpod-observer" cannot list resource "garagebuckets"
```

### 4. Pod Status Check
```bash
$ kubectl get pods -n garage-operator
NAME                               READY   STATUS        RESTARTS   AGE
garage-cnpg-0                      0/1     Terminating   0          5h30m
garage-operator-567bc7676f-4tnf7   1/1     Running       22         139m
```

## Parent Bead Status

**bf-3lpe65 (Create sccache bucket):** Closed with WARN/INFRA status
- Close reason: "Documented Garage infrastructure blocker preventing sccache bucket creation"
- Bucket was NOT actually created
- Prepared YAML manifests exist but were never applied
- Status: Infrastructure blocked

## Expected Bucket Configuration

If bucket existed, it would be:
- **Name:** `sccache` (from bf-6coted)
- **Endpoint:** `http://100.84.193.103:3900`
- **Region:** `us-east-iad-1`
- **Quota:** 10Gi
- **Access:** S3-compatible via Garage

## Acceptance Criteria Status

- ❌ **FAIL:** Bucket accessible via Garage CLI/S3 tool (BLOCKED - bucket does not exist)
- ⚠️ **WARN:** Bucket contains zero keys/objects (cannot verify non-existent bucket)
- ❌ **FAIL:** Read/write access verified (BLOCKED - no bucket to access)
- ✅ **PASS:** Access details documented (in bf-5dpz4p.md)

## Blocker Analysis

### Why This Bead Cannot Complete

1. **Bucket doesn't exist:** Parent bead bf-3lpe65 closed with INFRA blocker - no bucket created
2. **Garage infrastructure broken:** CRD version conflicts prevent bucket operations
3. **No admin access:** Cannot create bucket even if Garage was functional
4. **Read-only proxy limits:** Cannot exec into pods or manage resources

### Required Fixes

1. **Fix CRD issues:** Install missing GarageCluster v1beta2 CRD
2. **Resolve schema conflicts:** Fix existing GarageBucket resource schema
3. **Restart Garage deployment:** Delete Terminating pod, allow proper restart
4. **Obtain admin kubeconfig:** Regenerate apexalgo-iad.kubeconfig from Spot UI (~3 day expiry)

## Alternative Approaches

### Option 1: Deploy Garage on iad-ci
- We have admin kubeconfig access to iad-ci
- Could deploy fresh Garage instance
- Avoids apexalgo-iad access issues

### Option 2: Use external S3
- Backblaze B2, MinIO, or other S3-compatible storage
- Would require different endpoint configuration
- May be simpler than fixing broken Garage

## References

- **Parent bead:** bf-123uxh
- **Dependency:** bf-3lpe65 (bucket creation - closed with INFRA blocker)
- **Configuration details:** notes/bf-5dpz4p.md
- **Infrastructure verification:** notes/bf-3ucuqi.md
- **Bucket name determination:** notes/bf-6coted.md

---

**BLOCKER VERIFIED:** Cannot verify non-existent bucket due to broken Garage infrastructure and parent bead closure with INFRA status.
