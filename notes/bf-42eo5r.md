# Garage Cluster Access and CLI Tools Verification

**Bead:** bf-42eo5r  
**Date:** 2026-08-09  
**Task:** Verify Garage cluster access and CLI tools

## Summary

Verified Garage cluster access and CLI tool availability. **Critical finding: Garage deployment on apexalgo-iad is terminating and S3 endpoint is no longer accessible.**

## Detailed Findings

### 1. Garage CLI Status
- ❌ **FAIL:** Garage CLI is **NOT installed** on this system
- ✅ **PASS:** AWS CLI is available (aws-cli/1.37.21) - can interact with S3-compatible endpoints
- ❌ **FAIL:** s3cmd is NOT installed

### 2. kubectl Access to Garage Cluster

#### Read-Only Proxy Access (Working)
- ✅ **PASS:** Read-only kubectl proxy works: `kubectl --server=http://traefik-apexalgo-iad:8001`
- ✅ **PASS:** Can view basic resources (pods, services)
- ❌ **FAIL:** Cannot view Garage CRDs (forbidden by RBAC: `garageclusters.garage.rajsingh.info is forbidden`)

#### Garage Deployment Status
- ❌ **FAIL:** **Garage pods on apexalgo-iad are TERMINATING**
  - `garage-cnpg-0`: Status=Terminating (was Running in bf-3ucuqi verification)
  - `garage-operator`: CrashLoopBackOff (17 restarts, 105m old)

- ⚠️ **WARN:** **No active Garage deployment found on any cluster**
  - apexalgo-iad: Pods terminating
  - rs-manager: Namespace in "Terminating" state
  - ardenone-manager: Only garage-operator running (no Garage deployment)

### 3. Garage S3 Endpoint
- ❌ **FAIL:** **S3 endpoint NO LONGER ACCESSIBLE**
  - Expected endpoint: `http://100.84.193.103:3900`
  - Test result: Connection refused (curl exit code 7)
  - Previous verification (bf-3ucuqi): Endpoint was reachable

- ✅ **PASS:** Garage service definition exists on apexalgo-iad:
  ```yaml
  Service: garage-cnpg
  Namespace: garage-operator
  ClusterIP: 10.21.204.105
  Ports: 3900/TCP (S3), 3903/TCP (admin), 3902/TCP (web)
  Endpoints: <none>  # NO BACKING PODS
  ```

### 4. Authentication/Credentials
- ❌ **FAIL:** Admin kubeconfig does NOT exist
  - Expected path: `/home/coding/.kube/apexalgo-iad.kubeconfig`
  - Status: File does not exist
  - Impact: Cannot perform admin operations (create buckets, manage keys)

- ✅ **PASS:** DNS resolution works
  - `traefik-apexalgo-iad.ardenone.com` resolves to IPv6 addresses
  - Tailscale routing functional

### 5. Garage Cluster Endpoint Documentation

| Cluster | Status | Endpoint | Pods | Notes |
|---------|--------|----------|------|-------|
| apexalgo-iad | TERMINATING | `garage-cnpg.garage-operator.svc.cluster.local:3900` (10.21.204.105:3900) | 0/1 Terminating | Service exists, no endpoints |
| rs-manager | TERMINATING | N/A | N/A | Namespace terminating |
| ardenone-manager | OPERATOR ONLY | N/A | 1/1 Running | Operator only, no Garage deployment |

## Acceptance Criteria Status

- ❌ **FAIL:** Garage CLI responds to version/status commands - CLI NOT installed
- ⚠️ **WARN:** kubectl can access Garage cluster resources - read-only works, admin access missing
- ❌ **FAIL:** Authentication credentials are valid - admin kubeconfig missing
- ✅ **PASS:** Garage cluster endpoint URL documented - see table above

## Issues Found

### Critical Issues
1. **Garage deployment on apexalgo-iad is TERMINATING**
   - Pod `garage-cnpg-0` is in Terminating state
   - S3 endpoint at `http://100.84.193.103:3900` is no longer accessible
   - Service has no endpoints (no backing pods)

2. **No active Garage deployment**
   - Checked apexalgo-iad, rs-manager, ardenone-manager
   - Only operator pods found, no actual Garage storage pods

3. **No admin access to apexalgo-iad**
   - Admin kubeconfig missing: `/home/coding/.kube/apexalgo-iad.kubeconfig`
   - Cannot create buckets or manage keys without admin access

### Warnings
1. **Garage CLI not installed**
   - Will need to install for direct Garage management
   - AWS CLI available as alternative for S3 operations

2. **s3cmd not installed**
   - Alternative S3 client not available

## Recommendations

### Immediate Actions Needed
1. **Investigate Garage deployment status**
   - Why are Garage pods terminating on apexalgo-iad?
   - Was this intentional migration or failure?
   - Check with infrastructure team about Garage deployment plan

2. **Obtain admin access**
   - If Garage is being migrated: Get admin kubeconfig for new cluster
   - If Garage is deprecated: Determine alternative storage solution for sccache

3. **Install Garage CLI**
   - Install via: `cargo install garage` OR package manager
   - Required for bucket management and key operations

### Alternative Approaches
1. **Use different storage backend for sccache**
   - Consider local cache if Garage is deprecated
   - Check if other S3-compatible storage available

2. **Wait for Garage migration**
   - If this is planned migration, wait for new deployment
   - Update documentation once new endpoint is known

## Technical Details

### CLI Availability
```bash
# Missing tools
which garage    # NOT FOUND
which s3cmd     # NOT FOUND

# Available tools
which aws       # /home/coding/.nix-profile/bin/aws (v1.37.21)
```

### Service Endpoints
```bash
# apexalgo-iad (terminating)
kubectl --server=http://traefik-apexalgo-iad:8001 get svc -n garage-operator
# garage-cnpg: 10.21.204.105:3900 (S3) - NO ENDPOINTS

# Tailscale endpoint (no longer accessible)
curl -s http://100.84.193.103:3900
# Exit code 7 - connection refused
```

### Kubectl Access
```bash
# Read-only proxy (working)
kubectl --server=http://traefik-apexalgo-iad:8001 get pods -n garage-operator
# SUCCESS: Shows pods (terminating state)

# Admin kubeconfig (missing)
ls -la /home/coding/.kube/apexalgo-iad.kubeconfig
# ERROR: No such file or directory

# CRD access (forbidden)
kubectl --server=http://traefik-apexalgo-iad:8001 get garageclusters -n garage-operator
# ERROR: Forbidden by RBAC
```

## References

- Parent bead: bf-3lpe65 (sccache bucket creation)
- Previous verification: notes/bf-3ucuqi.md (Garage was accessible)
- Bucket manifest: .cli/tmp/sccache-garage-bucket.yml
- CLAUDE.md: apexalgo-iad access instructions

## Next Steps

This bead (bf-42eo5r) has **identified a critical infrastructure issue**. Before proceeding with bucket creation (bf-3lpe65):

1. **Determine Garage deployment status** - Is this migration or failure?
2. **Obtain admin access** - Get kubeconfig for cluster with active Garage
3. **Install Garage CLI** - Required for management operations
4. **Update documentation** - Record new endpoint if migrated

**Status:** BLOCKED - Requires investigation into Garage deployment status and admin access setup before bucket creation can proceed.

---
**Committed:** 2026-08-09  
**Commit:** [Pending]
