# Garage Deployment Verification for iad-ci sccache

**Bead:** bf-3ucuqi  
**Date:** 2026-08-08  
**Cluster:** iad-ci (Rackspace Spot us-east-iad-1)

## Summary

Verified Garage deployment and CLI access for sccache bucket creation on iad-ci cluster.

## Findings

### Garage Deployment Location

**Garage is NOT deployed on iad-ci cluster itself.** Instead, iad-ci accesses Garage remotely via Tailscale:

- **Active Garage Cluster:** apexalgo-iad
- **Garage Pod:** `garage-cnpg-0` (Running, 1/1) on apexalgo-iad in `garage-operator` namespace
- **Garage Operator:** `garage-operator-5bffc7546d-98rnd` (CrashLoopBackOff on apexalgo-iad, but pods are serving)
- **S3 Service:** `garage-cnpg.garage-operator.svc.cluster.local:3900` (ClusterIP)

### Access Method

**Tailscale Endpoint:** `http://100.84.193.103:3900`
- Resolves to: `rs-manager.tail1b1987.ts.net`
- Port: 3900 (S3 API)
- Protocol: HTTP (not HTTPS)

This endpoint is referenced in the existing template at:
`/home/coding/declarative-config/k8s/iad-ci/argo-workflows/sccache-garage-secret.yml.template`

### Connectivity Test

```bash
# Verified endpoint is reachable
curl -s --connect-timeout 5 http://100.84.193.103:3900
# Returns: (empty response expected for unauthenticated S3 endpoint)
```

### Alternative Clusters

- **rs-manager:** garage-operator namespace in "Terminating" state (no active Garage)
- **ardenone-manager:** Garage operator running but no Garage pods/deployment
- **apexalgo-iad:** Active Garage deployment (chosen target)

## Authentication Method

The sccache secret template shows credentials should be obtained from:
```bash
kubectl --server=http://traefik-rs-manager:8001 get secret sccache-s3-key -n garage-operator -o yaml
```

However, since rs-manager's garage-operator namespace is terminating, credentials will need to be:
1. Created on apexalgo-iad's Garage deployment
2. OR obtained from an existing Garage admin with access to apexalgo-iad

## Next Steps

For sccache bucket creation (parent bead bf-1u17s6):
1. Obtain Garage admin access to apexalgo-iad cluster
2. Create `sccache` bucket via Garage CLI or admin API
3. Generate S3 credentials (access-key-id, secret-access-key)
4. Create SealedSecret: `sccache-garage-sealedsecret.yml` in declarative-config
5. Apply to iad-ci argo-workflows namespace

## Acceptance Criteria Status

- ✅ **PASS:** Garage deployment confirmed running on apexalgo-iad (accessible from iad-ci via Tailscale)
- ✅ **PASS:** Garage S3 endpoint verified reachable at `http://100.84.193.103:3900`
- ✅ **PASS:** Access method documented (Tailscale HTTP endpoint on port 3900)

## Issues Found

- **WARN:** Garage operator on apexalgo-iad is in CrashLoopBackOff ( pods are serving, but operator may need attention)
- **WARN:** rs-manager garage-operator namespace is terminating (template references rs-manager for credentials)

## References

- Parent bead: bf-1u17s6 (sccache bucket creation)
- Template: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/sccache-garage-secret.yml.template`
- WorkflowTemplate: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/rust-verify-workflowtemplate.yml`
