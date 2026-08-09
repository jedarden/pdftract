# sccache Bucket Documentation and Accessibility Verification - INFRASTRUCTURE BLOCKER

**Bead:** bf-66s9do
**Date:** 2026-08-09 (Updated 13:19)
**Status:** BLOCKED - Cannot Complete (Parent bead bf-123uxh closed with infrastructure failure)

## Summary

**CANNOT DOCUMENT OR VERIFY ACCESSIBILITY - BUCKET DOES NOT EXIST.**

The parent bead (bf-123uxh) closed with CRITICAL infrastructure failure. The sccache bucket was never created due to complete Garage deployment failure on apexalgo-iad. This bead cannot proceed without:
1. Functional Garage deployment on apexalgo-iad, OR
2. Alternative Garage deployment on iad-ci cluster, OR
3. Admin access to fix the infrastructure

## Planned Bucket Configuration (Does NOT Exist)

Based on the workflow template and parent bead planning, the bucket SHOULD be configured as:

### Bucket Details (Planned)
- **Bucket Name:** `sccache`
- **Region:** `us-east-iad-1` (Rackspace Spot region for apexalgo-iad cluster)
- **S3 Endpoint:** `http://100.84.193.103:3900` (Tailscale IP from iad-ci to apexalgo-iad)
- **Protocol:** HTTP (no SSL, accessed via Tailscale VPN)
- **Storage Class:** Standard Garage S3-compatible storage
- **Quota:** 10Gi max size (from parent bead YAML)

### Kubernetes Configuration (Planned)
- **Secret Name:** `sccache-garage`
- **Secret Namespace:** `argo-workflows` (on iad-ci cluster)
- **Secret Type:** Opaque (sealed as SealedSecret)
- **Required Keys:**
  - `bucket`: "sccache"
  - `endpoint`: "http://100.84.193.103:3900"
  - `access-key-id`: <from Garage key creation>
  - `secret-access-key`: <from Garage key creation>

### CI/CD Integration (Planned)
- **WorkflowTemplate:** `rust-verify` in `k8s/iad-ci/argo-workflows/`
- **Environment Variables:**
  - `SCCACHE_BUCKET` (from secret)
  - `SCCACHE_ENDPOINT` (from secret)
  - `RUSTC_WRAPPER=sccache`
  - `SCCACHE_S3_USE_SSL=true`
- **Behavior:** Builds run cold when secret is missing; warm cache when secret exists

## Actual State (2026-08-09)

### Bucket Status
- **Bucket:** DOES NOT EXIST
- **Status:** Never created (blocked by infrastructure failure)
- **Parent Bead:** bf-123uxh closed with INFRASTRUCTURE FAILURE

### Garage Infrastructure Status (apexalgo-iad)
**COMPLETELY BROKEN - CRITICAL FAILURE**

| Component | Status | Duration |
|-----------|--------|----------|
| Garage Pod (`garage-cnpg-0`) | Stuck Terminating (0/1) | 5h 43m |
| Garage Operator | CrashLoopBackOff (23 restarts, last 2m55s ago) | 151m |
| S3 Endpoint (`http://100.84.193.103:3900`) | CONNECTION REFUSED | Since 2026-08-09 |
| Calico Network Plugin | Failing (delete errors) | Recurring |
| CRD Schema | v1alpha1/v1beta1/v1beta2 conflicts | Chronic |

### Root Causes (from parent bead documentation)

1. **Pod stuck terminating** - Cannot be cleaned up due to Calico network plugin failures
2. **Operator CrashLoopBackOff** - CRD version conflicts (missing v1beta2, schema mismatches)
3. **S3 endpoint down** - Connection refused on Tailscale IP
4. **No admin access** - OIDC token expired, requires regeneration from Spot UI

### Accessibility Test Results

**S3 Endpoint Test (2026-08-09 13:19):**
```bash
$ curl -v --connect-timeout 5 http://100.84.193.103:3900
*   Trying 100.84.193.103:3900...
* connect to 100.84.193.103 port 3900 from 100.81.129.38 port 59084 failed: Connection refused
* Failed to connect to 100.84.193.103 port 3900 after 11 ms: Could not connect to server
curl: (7) Failed to connect to 100.84.193.103 port 3900 after 11 ms: Could not connect to server
```
**Result:** FAIL - Endpoint completely unreachable (connection refused)

**kubectl Access Test:**
```bash
$ kubectl --server=http://traefik-apexalgo-iad:8001 get pods -n garage-operator
(error: Forbidden)
```
**Result:** FAIL - Read-only proxy cannot exec into terminating pod

**Admin Kubeconfig Check:**
```bash
$ ls /home/coding/.kube/apexalgo-iad.kubeconfig
ls: cannot access '/home/coding/.kube/apexalgo-iad.kubeconfig': No such file or directory
```
**Result:** FAIL - Admin kubeconfig expired/missing

## Acceptance Criteria Status

### ❌ FAIL: Bucket name, region, and endpoint documented
- **Status:** PASS (PLANNED configuration documented above)
- **Caveat:** Bucket does NOT exist - this is planned configuration only

### ❌ FAIL: Bucket accessibility confirmed (can list/read/write)
- **Status:** FAIL - Bucket does not exist, cannot test accessibility
- **Blocker:** Garage deployment completely broken, no admin access

### ✅ PASS: Documentation note created
- **Status:** PASS - This note at `notes/bf-66s9do.md`
- **Content:** Planned configuration + actual failure state

### ❌ FAIL: Bucket state (empty, no keys) confirmed
- **Status:** FAIL - Bucket does not exist, cannot verify state
- **Blocker:** Infrastructure failure prevents bucket creation

## Requirements to Complete This Bead

### Option 1: Fix apexalgo-iad Garage Deployment

**User must manually perform:**

1. **Regenerate admin kubeconfig:**
   - Login to Rackspace Spot UI
   - Navigate to apexalgo-iad cloudspace
   - Generate new cloudspace-admin OIDC token (~3 day expiry)
   - Save to: `/home/coding/.kube/apexalgo-iad.kubeconfig`

2. **Force-delete stuck pod and fix network:**
   ```bash
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     delete pod garage-cnpg-0 -n garage-operator --force --grace-period=0
   ```

3. **Fix CRD schema conflicts:**
   - Reconcile v1alpha1/v1beta1/v1beta2 versions
   - Install missing v1beta2 CRDs
   - Fix schema mismatches in existing resources

4. **Restart Garage deployment:**
   ```bash
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     rollout restart deployment garage-operator -n garage-operator
   ```

5. **Verify health:**
   ```bash
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     get pods -n garage-operator
   ```

6. **Then re-run bf-123uxh to create bucket**

### Option 2: Deploy Garage on iad-ci Cluster (Alternative)

Since iad-ci is the consuming cluster and admin access exists:

1. **Install Garage operator on iad-ci**
2. **Deploy Garage instance on iad-ci**
3. **Create bucket locally (no cross-cluster Tailscale needed)**
4. **Update rust-verify WorkflowTemplate** to use local endpoint
5. **Re-run bf-123uxh to create bucket**

### Option 3: Request Cluster Admin Intervention

**Ask cluster administrator to:**
1. Fix Garage deployment on apexalgo-iad
2. Create bucket: `sccache`
3. Create key: `sccache-s3-key` with read+write permissions
4. Provide credentials for SealedSecret creation

## Verification Steps (Once Infrastructure is Fixed)

**After bucket creation, verify with:**

```bash
# Test S3 endpoint reachability
curl -s --connect-timeout 5 http://100.84.193.103:3900

# List bucket (via AWS CLI or Garage CLI)
aws s3 ls s3://sccache --endpoint-url http://100.84.193.103:3900

# Test write operation
echo "test" | aws s3 cp - s3://sccache/test-object --endpoint-url http://100.84.193.103:3900

# Test read operation
aws s3 cp s3://sccache/test-object - --endpoint-url http://100.84.193.103:3900

# Verify empty state (should be no objects)
aws s3 ls s3://sccache --endpoint-url http://100.84.193.103:3900
```

## References

- **Parent bead:** bf-123uxh (Create sccache S3 bucket in Garage) - CLOSED WITH INFRASTRUCTURE FAILURE
- **Grandparent bead:** bf-1u17s6 (sccache bucket creation coordinator)
- **Secret Template:** `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/sccache-garage-secret.yml.template`
- **Workflow Template:** `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/rust-verify-workflowtemplate.yml`
- **Infrastructure failure details:** `notes/bf-123uxh.md` (2026-08-09)
- **Previous Garage verification:** `notes/bf-3ucuqi.md` (2026-08-08 - Garage was running then)

## Conclusion

**This bead CANNOT CLOSE until infrastructure is fixed.** The parent bead (bf-123uxh) documented complete Garage deployment failure, and this verification cannot proceed without:
1. Functional Garage deployment, OR
2. Alternative deployment strategy, OR
3. Admin intervention to fix infrastructure

The planned configuration is documented above for reference, but the bucket does not exist and is not accessible in the current infrastructure state.

---

**INFRASTRUCTURE BLOCKER:** This bead is blocked by complete Garage deployment failure on apexalgo-iad. The bucket does not exist, the endpoint is unreachable, and no admin access is available to fix the infrastructure.

**Verification Time:** 2026-08-09 13:19:14 - Infrastructure state unchanged from initial documentation. The Garage deployment remains completely non-functional with no path to resolution without admin intervention.
