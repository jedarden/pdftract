# sccache Bucket Configuration and Connection Details

**Bead:** bf-5dpz4p  
**Date:** 2026-08-09  
**Status:** Configuration documented (bucket creation pending Garage redeployment)

## Summary

This document captures all critical configuration details, endpoints, and connection parameters needed for sccache S3 integration with the Garage bucket on apexalgo-iad cluster. Future beads implementing sccache will reference these values to configure the S3 backend.

## Bucket Details

| Parameter | Value | Source |
|-----------|-------|--------|
| **Bucket Name** | `sccache` | bf-6coted |
| **Bucket Type** | S3-compatible (Garage) | Garage deployment |
| **Quota** | 10Gi | sccache-garage-bucket.yml |
| **Permissions** | Read + Write | sccache-s3-key |

## Cluster and Region Information

| Parameter | Value | Source |
|-----------|-------|--------|
| **Target Cluster** | apexalgo-iad | bf-3ucuqi |
| **Cluster Type** | Rackspace Spot | Infrastructure |
| **Region** | us-east-iad-1 | Spot region |
| **Garage Namespace** | garage-operator | Garage deployment |
| **Garage Deployment** | garage-cnpg | bf-3ucuqi |
| **Garage Pod** | garage-cnpg-0 (Running, 1/1) | bf-3ucuqi |

**Note:** While the bucket is accessed from iad-ci workflows, the actual Garage deployment runs on apexalgo-iad. iad-ci reaches Garage via Tailscale mesh connectivity.

## S3 Endpoint Configuration

### Primary Endpoint (Tailscale)

| Parameter | Value | Usage |
|-----------|-------|-------|
| **Endpoint URL** | `http://100.84.193.103:3900` | S3 API access from iad-ci |
| **Protocol** | HTTP | Not HTTPS |
| **Port** | 3900 | S3 API port |
| **Tailscale Hostname** | rs-manager.tail1b1987.ts.net | Resolves to endpoint |
| **Accessibility** | Tailscale mesh only | Not public internet |

### Alternative ClusterIP Endpoint

| Parameter | Value | Usage |
|-----------|-------|-------|
| **Internal Service** | `garage-cnpg.garage-operator.svc.cluster.local:3900` | In-cluster access |
| **Service Type** | ClusterIP | Internal to apexalgo-iad |

### Connectivity Verification

```bash
# Test endpoint reachability from iad-ci
curl -s --connect-timeout 5 http://100.84.193.103:3900
# Expected: Empty response (unauthenticated S3 endpoint)

# Test with credentials once bucket is created
aws s3 ls \
  --endpoint-url http://100.84.193.103:3900 \
  --bucket sccache
```

## sccache Configuration Examples

### Environment Variables

```bash
# Core sccache S3 configuration
export SCCACHE_BUCKET=sccache
export SCCACHE_ENDPOINT=http://100.84.193.103:3900
export SCCACHE_REGION=us-east-iad-1

# Authentication (use credentials provider or direct keys)
export SCCACHE_S3_USE_CREDENTIALS_PROVIDER=true
# OR
export SCCACHE_S3_ACCESS_KEY_ID="<access-key-id>"
export SCCACHE_S3_SECRET_ACCESS_KEY="<secret-access-key>"
```

### Rust / Cargo Integration

```toml
# In .cargo/config.toml or via environment
[build]
# sccache will use the environment variables above
```

### Argo Workflow Integration

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: sccache-garage
  namespace: argo-workflows
type: Opaque
stringData:
  bucket: "sccache"
  endpoint: "http://100.84.193.103:3900"
  access-key-id: "<from GarageKey>"
  secret-access-key: "<from GarageKey>"
```

### WorkflowTemplate Usage

```yaml
spec:
  templates:
  - name: rust-verify
    env:
    - name: SCCACHE_BUCKET
      value: "sccache"
    - name: SCCACHE_ENDPOINT
      value: "http://100.84.193.103:3900"
    - name: SCCACHE_REGION
      value: "us-east-iad-1"
    - name: SCCACHE_S3_USE_CREDENTIALS_PROVIDER
      value: "true"
    envFrom:
    - secretRef:
        name: sccache-garage
```

## S3-Compatible Endpoint Specifics

### Compatibility Notes

1. **Protocol**: HTTP only, not HTTPS
   - sccache must be configured with `http://` endpoint URL
   - SSL verification should be disabled or handled via environment

2. **Path Style vs. Domain Style**: Garage supports both, but path-style is recommended:
   ```
   http://100.84.193.103:3900/sccache/object-key
   ```

3. **Region**: `us-east-iad-1` matches Spot region naming
   - Required for AWS SDK compatibility
   - May be used for metadata operations

4. **Credentials Provider**: Set `SCCACHE_S3_USE_CREDENTIALS_PROVIDER=true`
   - Allows sccache to use standard AWS credential chain
   - Works with environment variables or AWS profiles

### Garage-Specific Behavior

| Feature | Support | Notes |
|---------|---------|-------|
| ListObjects | ✅ Yes | Standard S3 API |
| PutObject | ✅ Yes | Write operation |
| GetObject | ✅ Yes | Read operation |
| DeleteObject | ✅ Yes | Cleanup operation |
| Multipart Upload | ⚠️ Partial | May need testing |
| Versioning | ❌ No | Garage limitation |
| Encryption | ⚠️ Check | Garage has built-in replication |

## Related Configuration Files

### Garage Operator Manifests

1. **GarageBucket** (apexalgo-iad):
   - File: `/home/coding/pdftract/.cli/tmp/sccache-garage-bucket.yml`
   - Namespace: `garage-operator`
   - Quotas: 10Gi max size
   - Permissions: Read + Write via `sccache-s3-key`

2. **GarageKey** (apexalgo-iad):
   - File: `/home/coding/pdftract/.cli/tmp/sccache-garage-key.yml`
   - Namespace: `garage-operator`
   - Permissions: No bucket creation (bucket exists)

### iad-ci Integration

3. **Secret Template**:
   - File: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/sccache-garage-secret.yml.template`
   - Target namespace: `argo-workflows`
   - Requires sealing with kubeseal

4. **WorkflowTemplate** (rust-verify):
   - File: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/pdftract-ci.yaml`
   - Will consume sccache-garage secret
   - Sets SCCACHE_* environment variables

## Credentials and Authentication

### Credential Source

Credentials will be generated from GarageKey `sccache-s3-key` once bucket is created:

```bash
# Get credentials from apexalgo-iad (after bucket creation)
kubectl --server=http://traefik-apexalgo-iad:8001 \
  get secret sccache-s3-key \
  -n garage-operator \
  -o yaml

# Extract values:
# - access-key-id: secret data key
# - secret-access-key: secret data key
```

### Secret Creation Workflow

1. Create bucket on apexalgo-iad via Garage CLI or operator
2. GarageKey `sccache-s3-key` generates S3 credentials
3. Extract credentials from secret
4. Create SealedSecret for iad-ci argo-workflows namespace
5. Apply to iad-ci (sealed secret is safe to commit)

## Infrastructure Status

### Current Blocker

⚠️ **INFRASTRUCTURE BLOCKER**: As of 2026-08-09, the bucket does not exist because:

- **Garage cluster creation failed** (bf-2aykgt documented this)
- **apexalgo-iad Garage deployment** exists but bucket creation was blocked
- **rs-manager garage-operator namespace** is terminating (old location)

### Resolution Path

Before sccache can be used, one of the following must occur:

1. **Redeploy Garage cluster** on apexalgo-iad or iad-ci
2. **Create bucket manually** on existing apexalgo-iad Garage deployment
3. **Use alternative S3 storage** (e.g., Backblaze B2, MinIO on iad-ci)

### Bead Chain Context

- **bf-6coted**: ✅ Closed - Bucket name determined (`sccache`)
- **bf-2aykgt**: ❌ Closed - Infrastructure blocker (bucket not created)
- **bf-3n6niq**: ❌ Closed - Cannot verify non-existent bucket
- **bf-5dpz4p**: ✅ Current - Documenting configuration (this bead)
- **bf-3lpe65**: ⏳ Blocked - Waiting for bucket creation infrastructure

## Acceptance Criteria Status

- ✅ **PASS**: Bucket endpoint URL documented (`http://100.84.193.103:3900`)
- ✅ **PASS**: Cluster/region details recorded (apexalgo-iad, us-east-iad-1)
- ✅ **PASS**: Bucket name from bf-6coted referenced (`sccache`)
- ✅ **PASS**: Configuration saved in accessible location (`notes/bf-5dpz4p.md`)
- ✅ **PASS**: Documentation includes example sccache S3 configuration snippet

## References

- **Parent bead**: bf-3lpe65 (Create sccache bucket in Garage cluster)
- **Dependency**: bf-3n6niq (bucket verification - blocked by infrastructure)
- **Bucket name**: bf-6coted (determined `sccache`)
- **Access verification**: bf-3ucuqi (verified Garage deployment on apexalgo-iad)
- **Plan reference**: `/home/coding/pdftract/docs/plan/plan.md` (sccache integration sections)

---

**Documentation complete**: All configuration parameters for sccache S3 integration are captured here. Once the Garage infrastructure blocker is resolved and the `sccache` bucket is created, future beads can reference this document to configure sccache with the correct endpoint, credentials, and S3-compatible settings.
