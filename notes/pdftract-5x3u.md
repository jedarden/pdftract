# pdftract-5x3u: pdftract-crates-publish WorkflowTemplate

## Summary

Created the `pdftract-crates-publish` WorkflowTemplate for publishing pdftract crates to crates.io with proper ordering and index propagation waits.

## File Changed

- `jedarden/declarative-config/k8s/iad-ci/argo-workflows/pdftract-crates-publish.yaml` (new file)

## Implementation Details

### DAG Structure
The template implements a strict ordering to handle crates.io's index propagation requirement:

1. `publish-core` → Publish pdftract-core to crates.io
2. `wait-index-core` → Poll crates.io sparse index until core version appears (5-min timeout, 10s interval)
3. `publish-cli` → Publish pdftract-cli (depends on pdftract-core)
4. `wait-index-cli` → Poll crates.io sparse index until cli version appears
5. `publish-libpdftract` → (optional, conditional) Publish pdftract-libpdftract after cli

### Key Features

- **Index Propagation Wait**: Uses `https://index.crates.io/<path>` polling to verify the published version appears in the sparse index before proceeding to dependent crates
- **Idempotent Publish**: Treats "already uploaded" errors as success, allowing safe re-runs after partial failure
- **Dry-run Mode**: `--dry-run` parameter for testing without actual publishes
- **Version Validation**: Extracts and compares Cargo.toml version against git tag before publishing
- **ESO Secret Integration**: Uses `crates-io-token-pdftract` ExternalSecret (synced from OpenBao)

### Bug Fix

Fixed typo on line 181: `2>/grep` → `2>/dev/null` in the version extraction logic for `pdftract-libpdftract` (future-proofing).

### Container Configuration

- Base image: `rust:1.78-slim` (matches MSRV)
- Resources: 1-2 CPU, 2-4Gi memory
- Active deadline: 600s for publish, 360s for index wait

### Acceptance Criteria Status

| Criterion | Status |
|-----------|--------|
| WorkflowTemplate at correct path | PASS - `k8s/iad-ci/argo-workflows/pdftract-crates-publish.yaml` |
| DAG nodes in correct order | PASS - publish-core → wait-index-core → publish-cli → wait-index-cli |
| Index propagation wait (5-min timeout) | PASS - 300s timeout, 10s poll interval |
| Duplicate publish idempotent | PASS - "already uploaded" treated as success |
| Partial failure recovery documented | PASS - Recovery command in template comments |

## Testing Notes

- Manual testing requires a fresh git tag and crates.io API token
- The ExternalSecret `crates-io-token-pdftract` must exist in the iad-ci cluster (manifest exists at `k8s/iad-ci/argo-workflows/crates-io-token-pdftract-externalsecret.yml`)
- The `pdftract-libpdftract` crate does not exist yet in the workspace; the conditional publish step is future-proofing

## References

- Plan section: Release Engineering / Argo WorkflowTemplates, line 3391
- ADR-009: No GitHub Actions
- crates.io sparse index documentation: https://doc.rust-lang.org/cargo/reference/registry-index.html
