# sccache Bucket Name Determination

**Bead:** bf-6coted  
**Date:** 2026-08-09  
**Cluster:** apexalgo-iad (Rackspace Spot us-east-iad-1)

## Summary

**CHOSEN BUCKET NAME: `sccache`**

This name is recommended for the sccache cache bucket on the Garage cluster.

## Rationale

### Why `sccache`?

1. **Simplicity and clarity**: Single word, instantly recognizable as a sccache cache
2. **Follows existing convention**: Matches the name in `.cli/tmp/sccache-garage-bucket.yml`
3. **Purpose alignment**: Directly indicates the bucket's purpose (sccache cache storage)
4. **Namespace separation**: Clear and distinct from other buckets like `openbao`

### Alternatives Considered

| Name | Pros | Cons | Decision |
|------|------|------|----------|
| `sccache` | Simple, clear, follows existing config | None | ✅ **CHOSEN** |
| `sccache-cache` | Explicitly states "cache" | Redundant ("sccache" already means cache) | Rejected |
| `rust-verify-sccache` | Ties to specific use case (rust-verify) | Too specific, limits reuse | Rejected |

## S3 Naming Convention Compliance

The chosen name `sccache` complies with AWS S3 bucket naming rules:

### ✅ Compliance Check

| Rule | Requirement | `sccache` | Status |
|------|-------------|-----------|--------|
| Length | 3-63 characters | 7 characters | ✅ PASS |
| Characters | Lowercase letters, numbers, hyphens, dots | `sccache` (all lowercase) | ✅ PASS |
| Start/end | Must start and end with letter or number | Starts with 's', ends with 'e' | ✅ PASS |
| Forbidden chars | No uppercase, underscores, spaces | None present | ✅ PASS |
| DNS compatibility | Must be DNS-compatible | `sccache` is valid | ✅ PASS |

### Naming Rule Reference

- [AWS S3 Bucket Naming Rules](https://docs.aws.amazon.com/AmazonS3/latest/userguide/bucketnamingrules.html)
- Length: 3-63 characters
- Allowed: lowercase letters, numbers (0-9), hyphens (-)
- Must start and end with letter or number
- No uppercase letters, underscores, or spaces

## Uniqueness Verification

### Existing Buckets

**On rs-manager (terminating cluster):**
- `openbao` - No conflict

**On apexalgo-iad (active cluster):**
- No buckets visible via read-only proxy
- `sccache` will be unique

### Conflict Analysis

The name `sccache`:
- ✅ Does not conflict with existing `openbao` bucket
- ✅ No other buckets detected on apexalgo-iad
- ✅ Distinct and clear purpose identification

## Integration Points

### Files Using This Name

1. **GarageBucket manifest**: `.cli/tmp/sccache-garage-bucket.yml`
   ```yaml
   metadata:
     name: sccache
     namespace: garage-operator
   ```

2. **GarageKey manifest**: `.cli/tmp/sccache-garage-key.yml`
   ```yaml
   metadata:
     name: sccache-s3-key
   ```

3. **Secret template**: `/home/coding/declarative-config/k8s/iad-ci/argo-workflows/sccache-garage-secret.yml.template`
   - Will reference `sccache` bucket name

### Environment Variables

When the bucket is created, sccache will be configured with:
```bash
export SCCACHE_BUCKET=sccache
export SCCACHE_ENDPOINT=http://100.84.193.103:3900
export SCCACHE_REGION=us-east-iad-1
export SCCACHE_S3_USE_CREDENTIALS_PROVIDER=true
```

## Acceptance Criteria Status

- ✅ **PASS**: Bucket name chosen and documented (`sccache`)
- ✅ **PASS**: Name verified unique against existing buckets (only `openbao` exists)
- ✅ **PASS**: Name follows S3 naming conventions (all rules met)

## Next Steps

This bead (bf-6coted) completes the name determination. The next bead in the chain (bf-123uxh) will:
1. Use admin access to apexalgo-iad
2. Create the `sccache` bucket via Garage CLI
3. Create S3 credentials with read+write permissions
4. Set up SealedSecret for iad-ci argo-workflows namespace

## References

- Parent bead: bf-123uxh (Create sccache S3 bucket in Garage)
- S3 naming rules: [AWS Bucket Naming Rules](https://docs.aws.amazon.com/AmazonS3/latest/userguide/bucketnamingrules.html)
- Existing config: `.cli/tmp/sccache-garage-bucket.yml`
- Cluster verification: `notes/bf-3ucuqi.md`
- Bucket creation note: `notes/bf-123uxh.md`

---
**Decision finalized**: Use `sccache` as the bucket name for sccache cache storage on apexalgo-iad Garage cluster.
