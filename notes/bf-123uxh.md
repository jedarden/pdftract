# sccache S3 Bucket Creation - Blocked on Admin Access

**Bead:** bf-123uxh  
**Date:** 2026-08-08  
**Status:** BLOCKED - Requires admin access to apexalgo-iad  
**Cluster:** apexalgo-iad (Rackspace Spot us-east-iad-1)

## Summary

**CANNOT PROCEED WITHOUT ADMIN ACCESS TO APEXALGO-IAD CLUSTER.**

The bucket creation requires either:
1. Admin kubeconfig for apexalgo-iad to create resources
2. OR direct Garage CLI access via `kubectl exec` into garage pod
3. OR request cluster admin to create bucket/key directly

## Current State

### Garage Deployment on apexalgo-iad
- **Garage Cluster:** apexalgo-iad (verified in parent bead bf-3ucuqi)
- **Garage Pod:** `garage-cnpg-0` (Running, 1/1) in `garage-operator` namespace
- **Garage Operator:** `garage-operator-5bffc7546d-98rnd` (Running, 1/1) - ✅ **Fixed** (was in CrashLoopBackOff during bf-3ucuqi verification)
- **S3 Endpoint:** `http://100.84.193.103:3900` (Tailscale, accessible from iad-ci)
- **Visible Secrets:** `garage-cnpg-secrets` visible via read-only proxy
- **Status:** Garage deployment is active and serving

### Access Blocker

**Required but unavailable:** Admin kubeconfig for apexalgo-iad cluster
- Expected path: `/home/coding/.kube/apexalgo-iad.kubeconfig`
- Status: **File does not exist** (likely expired OIDC token)
- Renewal: Requires cloudspace-admin OIDC token from Rackspace Spot UI (~3 day expiry)

### Available Access
- ✅ Read-only kubectl proxy: `kubectl --server=http://traefik-apexalgo-iad:8001`
- ❌ Cannot exec into pods (Forbidden by serviceaccount)
- ❌ Cannot create resources (Forbidden: User cannot create resource "garagebuckets")
- ❌ Cannot read secrets (Forbidden by serviceaccount)
- ❌ Cannot use Garage CLI or S3 API without credentials

### Attempted Approaches

1. **Read-only kubectl proxy (`traefik-apexalgo-iad:8001`):**
   - ❌ Cannot exec into pods: `Forbidden`
   - ❌ Cannot create CRDs: `Forbidden: User cannot create resource "garagebuckets"`
   - ✅ Can list secrets (but cannot read their contents)
   - ServiceAccount: `system:serviceaccount:devpod-observer:devpod-observer`

2. **No admin kubeconfig for apexalgo-iad:**
   - Expected location: `/home/coding/.kube/apexalgo-iad.kubeconfig`
   - Status: **DOES NOT EXIST**
   - According to CLAUDE.md: Should use cloudspace-admin OIDC token (~3 day expiry)
   - Token must be regenerated from Rackspace Spot UI

3. **rs-manager garage-operator is terminating:**
   - Status: `Terminating` (114 days)
   - Cannot use existing template: references rs-manager garage-operator
   - Template references rs-manager for credentials (obsolete)

4. **AWS CLI with S3 endpoint** - Requires credentials (no admin keys available)
5. **Direct S3 API** - Requires authentication (no keys accessible without secret read)

## Requirements to Complete

### Option 1: Regenerate Admin Kubeconfig (Recommended)

**User must manually perform these steps:**

1. **Login to Rackspace Spot UI:**
   - Navigate to the apexalgo-iad cloudspace
   - Generate new cloudspace-admin OIDC token (expires ~3 days)

2. **Create/update kubeconfig:**
   - Get the OIDC token and kubeconfig from Spot UI
   - Save to: `/home/coding/.kube/apexalgo-iad.kubeconfig`

3. **Once admin access is available, create bucket:**
   ```bash
   # Via Garage CLI directly in the pod:
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     exec -n garage-operator garage-cnpg-0 -c garage -- \
     garage bucket create sccache
   ```

4. **Create S3 credentials:**
   ```bash
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     exec -n garage-operator garage-cnpg-0 -c garage -- \
     garage key create sccache-key
   
   kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
     exec -n garage-operator garage-cnpg-0 -c garage -- \
     garage bucket allow sccache --read --write --key sccache-key
   ```

5. **Extract credentials and create SealedSecret for iad-ci:**
   - Get credentials from the created key
   - Run `kubeseal` to create `sccache-garage-sealedsecret.yml`
   - Apply to iad-ci argo-workflows namespace

### Option 2: Request Cluster Admin to Create

**Ask cluster administrator to create:**
- Bucket name: `sccache` or `rust-verify-sccache`
- Key name: `sccache-s3-key`
- Permissions: read+write on bucket
- Quota: 10Gi max size

**Then extract credentials:**
```bash
# Admin provides access-key-id and secret-access-key
# Create SealedSecret manually with provided values
```

### Option 3: Use Spot UI Console

**If Rackspace Spot provides a console for Garage:**
- Access Garage management interface via Spot UI
- Create bucket `sccache` directly
- Generate S3 credentials
- Provide credentials for SealedSecret creation

## Proposed Bucket Name

**Recommended:** `sccache` or `rust-verify-sccache`
- Rationale: Clear purpose identification, matches sccache expectations
- Conflict check: Cannot verify without list-bucket access

## Acceptance Criteria Status

- ❌ **FAIL:** S3 bucket created successfully in Garage (blocked on admin access)
- ❌ **FAIL:** Bucket name documented and confirmed unique (cannot list without admin access)
- ❌ **FAIL:** Bucket is empty (bucket doesn't exist yet)

## Next Steps

### Immediate Action Required

**USER MUST PROVIDE ONE OF:**

1. **Admin kubeconfig for apexalgo-iad** (regenerate OIDC token from Spot UI)
   - Save to: `/home/coding/.kube/apexalgo-iad.kubeconfig`
   - Re-run this bead with admin access

2. **OR request cluster admin to create bucket/key directly**
   - Provide bucket name: `sccache`
   - Provide key name: `sccache-s3-key`
   - Provide read+write permissions
   - Return credentials for SealedSecret creation

### Once Admin Access is Available

1. Create bucket via Garage CLI (see Option 1 commands above)
2. Create S3 key with read+write permissions
3. Verify bucket is empty and accessible
4. Extract credentials (access-key-id, secret-access-key)
5. Create SealedSecret: `sccache-garage-sealedsecret.yml`
6. Apply to iad-ci argo-workflows namespace
7. Update rust-verify WorkflowTemplate to use secret

## References

- Parent bead: bf-1u17s6 (sccache bucket creation)
- Depends on: bf-3ucuqi (Garage access verified - but only read-only)
- Plan: Rust build / test offloading (lines in plan.md)
- Secret Template: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/sccache-garage-secret.yml.template`
- S3 Endpoint: `http://100.84.193.103:3900` (Tailscale from iad-ci)
- CLAUDE.md: "kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig" (admin access, OIDC token from Spot UI)

## Documentation Created

- This note: `/home/coding/pdftract/notes/bf-123uxh.md`
- Previous verification: `/home/coding/pdftract/notes/bf-3ucuqi.md`

---

**BLOCKER SUMMARY:** This bead cannot close without admin access to apexalgo-iad cluster. The user must regenerate the OIDC token from Rackspace Spot UI and create the admin kubeconfig, OR request the cluster administrator to create the bucket and provide credentials. Once admin access is available, re-run this bead to complete the bucket creation.

**DO NOT CLOSE THIS BEAD** - Admin access must be obtained before bucket creation can proceed.
