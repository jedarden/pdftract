# SealedSecret Sealing Details

## forgejo-ci-token.yaml

Sealed on: 2026-08-07

### Tooling
- kubeseal version: 0.27.1
- Controller: sealed-secrets controller in iad-ci cluster
- Controller namespace: sealed-secrets

### Verification
The sealed secret was created using kubeseal with the iad-ci cluster's sealed-secrets controller.
The encrypted data can only be decrypted by the sealed-secrets controller running in iad-ci.

### Secret Details
- Name: forgejo-ci-token
- Namespace: argo-workflows
- Type: Opaque
- Data: token (Forgejo CI API token for workflow authentication)
