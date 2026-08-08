# sccache S3 Bucket Creation - Admin Access Required

**Bead:** bf-123uxh
**Date:** 2026-08-08
**Status:** BLOCKED - Requires admin access to apexalgo-iad

## Summary

Cannot create sccache S3 bucket in Garage without admin access to apexalgo-iad cluster.

## Current Situation

### What I Verified

1. **Garage Deployment Location (from bf-3ucuqi):**
   - Garage is running on **apexalgo-iad** cluster
   - Pod: `garage-cnpg-0` (Running, 1/1) in `garage-operator` namespace
   - S3 Endpoint: `http://100.84.193.103:3900` (Tailscale, accessible from iad-ci)

2. **Admin Access Status:**
   - ❌ `/home/coding/.kube/apexalgo-iad.kubeconfig` **DOES NOT EXIST**
   - ❌ Read-only proxy (`traefik-apexalgo-iad:8001`) cannot:
     - Create GarageBucket/GarageKey resources (Forbidden)
     - Read secrets (Forbidden)
     - Exec into pods (Forbidden)

3. **Alternative Clusters Checked:**
   - ❌ **rs-manager:** garage-operator namespace in "Terminating" state (114 days)
   - ❌ **ardenone-manager:** Garage operator running but NO Garage pods deployed
   - ❌ **ardenone-hub:** Query timed out (cluster unreachable)

### What I Attempted

```bash
# Checked for admin kubeconfig
$ test -f /home/coding/.kube/apexalgo-iad.kubeconfig
DOES NOT EXIST

# Tried read-only proxy permissions
$ kubectl --server=http://traefik-apexalgo-iad:8001 auth can-i create garagebuckets -n garage-operator
no

# Checked Garage CRD access
$ kubectl --server=http://traefik-apexalgo-iad:8001 get garage -n garage-operator
error: the server doesn't have a resource type "garage"

# Tried reading secrets
$ kubectl --server=http://traefik-apexalgo-iad:8001 get secrets -n garage-operator garage-cnpg-secrets
Error: Forbidden
```

## What's Needed

### Option 1: Regenerate Admin Kubeconfig (RECOMMENDED)

**User must perform these steps:**

1. Login to Rackspace Spot UI
2. Navigate to apexalgo-iad cloudspace
3. Generate new cloudspace-admin OIDC token (expires ~3 days)
4. Create/update kubeconfig at `/home/coding/.kube/apexalgo-iad.kubeconfig`

**Then create bucket with:**
```bash
# Create bucket via Garage CLI in pod
kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
  exec -n garage-operator garage-cnpg-0 -c garage -- \
  garage bucket create sccache

# Create S3 credentials
kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
  exec -n garage-operator garage-cnpg-0 -c garage -- \
  garage key create sccache-key --permission-read=true --permission-write=true

# Grant key access to bucket
kubectl --kubeconfig=/home/coding/.kube/apexalgo-iad.kubeconfig \
  exec -n garage-operator garage-cnpg-0 -c garage -- \
  garage bucket allow sccache --key sccache-key --read --write
```

### Option 2: Alternative Deployment Location

Deploy Garage on a cluster where we have admin access (e.g., iad-ci itself) instead of relying on apexalgo-iad. This would require:
- Installing Garage on iad-ci cluster
- Creating bucket locally
- Updating rust-verify WorkflowTemplate to use local endpoint

## Acceptance Criteria Status

- ❌ **FAIL:** S3 bucket created successfully in Garage (blocked on admin access)
- ❌ **FAIL:** Bucket name documented and confirmed unique (cannot create without admin access)
- ❌ **FAIL:** Bucket is empty (bucket doesn't exist yet)

## Prepared Files

The following YAML files are already prepared in `.cli/tmp/`:
- `sccache-garage-bucket.yml` - GarageBucket definition
- `sccache-garage-key.yml` - GarageKey definition

**However, these reference `clusterRef: garage-rs-manager`** which is on rs-manager (terminating). They need to be applied to apexalgo-iad instead.

## Next Steps

**DO NOT CLOSE THIS BEAD** - Admin access must be obtained before bucket creation can proceed.

**USER ACTION REQUIRED:**
1. Regenerate apexalgo-iad admin kubeconfig from Rackspace Spot UI
2. Save to `/home/coding/.kube/apexalgo-iad.kubeconfig`
3. Re-run this bead to complete bucket creation

## References

- Parent bead: bf-1u17s6 (sccache bucket creation)
- Depends on: bf-3ucuqi (Garage access verified - but only read-only)
- Previous verification: `/home/coding/pdftract/notes/bf-3ucuqi.md`
- S3 Endpoint: `http://100.84.193.103:3900` (Tailscale from iad-ci)
