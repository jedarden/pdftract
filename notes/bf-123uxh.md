# sccache S3 Bucket Creation - INFRASTRUCTURE CRITICAL FAILURE

**Bead:** bf-123uxh
**Date:** 2026-08-09 (Updated from 2026-08-08)
**Status:** BLOCKED - Garage Infrastructure Completely Broken
**Cluster:** apexalgo-iad (Rackspace Spot us-east-iad-1)

## Summary

**CANNOT PROCEED - GARAGE INFRASTRUCTURE ON APEXALGO-IAD IS COMPLETELY NON-FUNCTIONAL.**

The bucket creation requires either:
1. Admin kubeconfig for apexalgo-iad to create resources
2. OR direct Garage CLI access via `kubectl exec` into garage pod
3. OR request cluster admin to create bucket/key directly

**However, even if admin access were obtained, the Garage deployment is completely broken.**

## Current State (2026-08-09)

### Garage Deployment on apexalgo-iad - CRITICAL FAILURE
- **Garage Cluster:** apexalgo-iad (verified in parent bead bf-3ucuqi)
- **Garage Pod:** `garage-cnpg-0` (Terminating, 0/1) - **STUCK TERMINATING FOR 5h32m**
- **Garage Operator:** `garage-operator-567bc7676f-4tnf7` (CrashLoopBackOff, 22 restarts in 140m)
- **S3 Endpoint:** `http://100.84.193.103:3900` - **COMPLETELY DOWN** (curl: exit code 7, connection refused)
- **Status:** Garage deployment is **NON-FUNCTIONAL**

### Recent Critical Events (2026-08-09)
```
43s FailedKillPod pod/garage-cnpg-0
  error killing pod: failed to "KillPodSandbox" with KillPodSandboxError:
  "rpc error: code = Unknown desc = failed to destroy network for sandbox:
  plugin type="calico" failed (delete): error getting ClusterInformation:
  dial tcp 10.21.0.1:443: connect: connection refused"
```

The stuck Garage pod cannot be cleaned up due to Calico network plugin failures.

### Access Blocker (MULTIPLE LAYERS)

**Required but unavailable:** Admin kubeconfig for apexalgo-iad cluster
- Expected path: `/home/coding/.kube/apexalgo-iad.kubeconfig`
- Status: **File does not exist** (likely expired OIDC token)
- Renewal: Requires cloudspace-admin OIDC token from Rackspace Spot UI (~3 day expiry)

**New Critical Issue:** Garage infrastructure is completely broken
- Even if admin access were obtained, Garage is non-functional
- Pod stuck terminating (5.5 hours)
- Operator in CrashLoopBackOff
- S3 endpoint completely down
- Network plugin failing

### Available Access
- YES: Read-only kubectl proxy: `kubectl --server=http://traefix-apexalgo-iad:8001`
- NO: Cannot exec into pods (Forbidden by serviceaccount, and pod is terminating anyway)
- NO: Cannot create resources (Forbidden: User cannot create resource "garagebuckets")
- NO: Cannot read secrets (Forbidden by serviceaccount)
- NO: Cannot use Garage CLI or S3 API (endpoint down + no credentials)
- NO: Garage S3 endpoint: `http://100.84.193.103:3900` - **CONNECTION REFUSED**

### Infrastructure Comparison: 2026-08-08 vs 2026-08-09

| Component | 2026-08-08 State | 2026-08-09 State | Change |
|-----------|-----------------|-----------------|---------|
| Garage Pod | Running (1/1) | Terminating (0/1) | CRITICAL DEGRADATION |
| Garage Operator | Running (1/1) | CrashLoopBackOff (22 restarts) | CRITICAL DEGRADATION |
| S3 Endpoint | Reachable | Connection refused | CRITICAL DEGRADATION |
| Network Plugin | Functional | Failing (Calico errors) | CRITICAL DEGRADATION |

### Attempted Approaches (2026-08-09)

1. **Read-only kubectl proxy (`traefix-apexalgo-iad:8001`):**
   - NO: Cannot exec into pods: `Forbidden`
   - NO: Cannot create CRDs: `Forbidden: User cannot create resource "garagebuckets"`
   - NO: Pod is in "Terminating" state anyway
   - ServiceAccount: `system:serviceaccount:devpod-observer:devpod-observer`

2. **No admin kubeconfig for apexalgo-iad:**
   - Expected location: `/home/coding/.kube/apexalgo-iad.kubeconfig`
   - Status: **DOES NOT EXIST**
   - According to CLAUDE.md: Should use cloudspace-admin OIDC token (~3 day expiry)
   - Token must be regenerated from Rackspace Spot UI

3. **S3 Endpoint reachability test:**
   ```bash
   $ curl -s --connect-timeout 5 http://100.84.193.103:3900
   Exit code 7 - Failed to connect to host
   ```
   Previously reachable on 2026-08-08, now completely down.

4. **Alternative clusters checked:**
   - rs-manager: garage-operator namespace "Terminating" (115 days)
   - ardenone-manager: Garage operator running but NO Garage pods deployed
   - iad-options: No Garage deployment
   - iad-ci: No Garage deployment (kubeconfig credentials expired)

5. **AWS CLI with S3 endpoint** - Requires credentials (no admin keys available)
6. **Direct S3 API** - Requires authentication (no keys accessible without secret read)

## Requirements to Complete

### Option 1: Fix apexalgo-iad Garage (REQUIRES ADMIN ACCESS)

**User must manually perform these steps:**

1. **Regenerate admin kubeconfig for apexalgo-iad:**
   - Login to Rackspace Spot UI
   - Navigate to the apexalgo-iad cloudspace
   - Generate new cloudspace-admin OIDC token (expires ~3 days)
   - Create kubeconfig: `/home/coding/.kube/apexalgo-iad.kubeconfig`

2. **Force-delete stuck Garage pod and fix network:**
   ```bash
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     delete pod garage-cnpg-0 -n garage-operator --force --grace-period=0
   ```

3. **Investigate and fix Calico network plugin issues**

4. **Restart Garage deployment and verify health:**
   ```bash
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     rollout restart deployment garage-operator -n garage-operator
   
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     get pods -n garage-operator
   ```

5. **Once Garage is healthy, create bucket:**
   ```bash
   # Via Garage CLI directly in the pod:
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     exec -n garage-operator garage-cnpg-0 -c garage -- \
     garage bucket create sccache
   ```

6. **Create S3 credentials:**
   ```bash
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     exec -n garage-operator garage-cnpg-0 -c garage -- \
     garage key create sccache-key
   
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     exec -n garage-operator garage-cnpg-0 -c garage -- \
     garage bucket allow sccache --read --write --key sccache-key
   ```

7. **Extract credentials and create SealedSecret for iad-ci:**
   - Get credentials from the created key
   - Run `kubeseal` to create `sccache-garage-sealedsecret.yml`
   - Apply to iad-ci argo-workflows namespace

### Option 2: Deploy Garage on iad-ci Cluster (ALTERNATIVE)

Since iad-ci is the consuming cluster and we have admin access there (when credentials are valid):

1. **Refresh iad-ci admin credentials** (if expired)
2. **Install Garage operator on iad-ci**
3. **Deploy Garage instance on iad-ci**
4. **Create bucket locally:**
   ```bash
   kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig \
     exec -n garage-operator <garage-pod> -c garage -- \
     garage bucket create sccache
   ```
5. **Update rust-verify WorkflowTemplate** to use local endpoint instead of Tailscale remote

### Option 3: Request Cluster Admin to Create

**Ask cluster administrator to create on apexalgo-iad (once it's fixed):**
- Bucket name: `sccache` or `rust-verify-sccache`
- Key name: `sccache-s3-key`
- Permissions: read+write on bucket
- Quota: 10Gi max size

**Then extract credentials:**
```bash
# Admin provides access-key-id and secret-access-key
# Create SealedSecret manually with provided values
```

## Proposed Bucket Name

**Recommended:** `sccache` or `rust-verify-sccache`
- Rationale: Clear purpose identification, matches sccache expectations
- Conflict check: Cannot verify without list-bucket access (infrastructure broken)

## Acceptance Criteria Status

- **FAIL:** S3 bucket created successfully in Garage (BLOCKED - infrastructure completely broken)
- **WARN:** Bucket name documented as `sccache` (from bf-6coted) but bucket cannot be created
- **FAIL:** Bucket is empty (bucket doesn't exist and cannot be created)

## Prepared Artifacts (Ready but Cannot Apply)

The following YAML manifests exist in `/home/coding/pdftract/.cli/tmp/`:

**File:** `sccache-garage-bucket.yml`
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

**File:** `sccache-garage-key.yml`
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

**However, these reference `garage-cnpg` on apexalgo-iad** which is now non-functional.

## Next Steps

### Immediate Action Required

**USER MUST PROVIDE ONE OF:**

1. **Admin kubeconfig for apexalgo-iad** AND fix the broken Garage deployment
   - Regenerate OIDC token from Spot UI
   - Save to: `/home/coding/.kube/apexalgo-iad.kubeconfig`
   - Force-delete stuck pods and fix network issues
   - Restart Garage deployment
   - Re-run this bead with functional Garage

2. **OR deploy Garage on iad-ci cluster** (alternative approach)
   - We have admin access to iad-ci
   - Install Garage locally on the consuming cluster
   - Create bucket and use local endpoint

3. **OR request cluster admin to create bucket/key directly**
   - Once Garage deployment is fixed on apexalgo-iad
   - Provide bucket name: `sccache`
   - Provide key name: `sccache-s3-key`
   - Provide read+write permissions
   - Return credentials for SealedSecret creation

### Once Admin Access + Functional Garage are Available

1. Force-delete stuck Garage pod and fix network issues
2. Restart Garage deployment and verify health
3. Create bucket via Garage CLI (see Option 1 commands above)
4. Create S3 key with read+write permissions
5. Verify bucket is empty and accessible
6. Extract credentials (access-key-id, secret-access-key)
7. Create SealedSecret: `sccache-garage-sealedsecret.yml`
8. Apply to iad-ci argo-workflows namespace
9. Update rust-verify WorkflowTemplate to use secret

## References

- Parent bead: bf-1u17s6 (sccache bucket creation)
- Depends on: bf-3ucuqi (Garage access verified - but infrastructure since failed)
- Previous verification: `notes/bf-3ucuqi.md` (2026-08-08 - Garage was running)
- Infrastructure blocker: `notes/bf-3lpe65.md` (2026-08-09 - documented same issues)
- Bucket name determination: `notes/bf-6coted.md`
- Secret Template: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/sccache-garage-secret.yml.template`
- S3 Endpoint: `http://100.84.193.103:3900` (Tailscale from iad-ci) - CURRENTLY DOWN
- CLAUDE.md: "kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig" (admin access, OIDC token from Spot UI)

## Documentation Created

- This note: `/home/coding/pdftract/notes/bf-123uxh.md` (updated 2026-08-09)
- Previous verification: `/home/coding/pdftract/notes/bf-3ucuqi.md` (2026-08-08)

---

**CRITICAL INFRASTRUCTURE FAILURE:** The Garage deployment on apexalgo-iad has suffered a complete failure since 2026-08-08. The pod is stuck terminating, the operator is in CrashLoopBackOff, the S3 endpoint is down, and the network plugin is failing. This represents a critical degradation from the previous "Running, 1/1" state.

**BLOCKER SUMMARY:** This bead cannot close without:
1. Admin access to apexalgo-iad cluster (regenerate OIDC token from Spot UI)
2. AND functional Garage deployment (currently broken - requires repair)
3. OR alternative Garage deployment on iad-ci cluster

**CRD Version Conflicts Also Identified:**

From garage-operator logs (discovered in sibling beads bf-3lpe65 and bf-3vxias):
```
error: "no matches for kind \"GarageCluster\" in version \"garage.rajsingh.info/v1beta2\""
error: "failed to list *v1beta1.GarageBucket: json: cannot unmarshal string into Go struct field KeyPermission.items.spec.keyPermissions.keyRef of type v1beta1.KeyRef"
error: "no matches for kind \"GarageKey\" in version \"garage.rajsingh.info/v1alpha1\""
```

**Additional Root Causes:**
- Missing `GarageCluster` v1beta2 CRD
- Schema mismatch in existing `GarageBucket` resources
- v1alpha1/v1beta1/v1beta2 version conflicts

This means even if the pod issues were fixed, the operator cannot function until the CRD schema conflicts are resolved.

**DO NOT CLOSE THIS BEAD** - Admin access AND infrastructure repair must be completed before bucket creation can proceed. The infrastructure is significantly worse than when this bead was initially attempted, with both pod/network issues AND CRD schema conflicts blocking operations.
