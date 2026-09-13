# Sealed Secrets for pdftract CI — REMOVED 2026-09-13

The `forgejo-ci-token` SealedSecret (argo-workflows namespace) that lived in
this directory was deleted and must **not** be re-created or applied.

## Why it was removed

- The underlying 2026-08-07 Forgejo PATs were exposed in legacy pdftract bead
  notes and are being revoked — see P0 bead `pdftract-3eea9d13` ("Rotate
  Forgejo credentials exposed in historical pdftract bead notes").
- declarative-config commit `f9e86590` ("drop orphaned forgejo-ci-token
  SealedSecrets") removed its two copies of the same ciphertext
  (`k8s/iad-ci/sealed-secrets/`, `k8s/iad-ci/external-secrets/`) for that
  reason. The file here was the identical ciphertext (verified by
  fingerprint) and was the last stray copy.
- No WorkflowTemplate references `forgejo-ci-token` in argo-workflows; the
  Secret it materialized was unused.

## How CI actually authenticates to git.ardenone.com

The `rust-verify` template (and other iad-ci templates) inject
`FORGEJO_TOKEN` from Secret `forgejo-webhook-token` (argo-workflows
namespace), an ExternalSecret pulling from OpenBao
`secret/rs-manager/iad-ci/forgejo/ci-token`. Rotation happens at the OpenBao
path and External Secrets Operator syncs it — credentials must not be sealed
into git.
