# Sealed Secrets for pdftract CI

This directory contains SealedSecret manifests for the pdftract CI pipeline on iad-ci.

## SealedSecret Details

### forgejo-ci-token.yaml

- **Purpose**: Forgejo API token for CI authentication
- **Namespace**: argo-workflows
- **Sealed with**: kubeseal v0.27.1
- **Controller**: sealed-secrets-controller (via rs-manager cluster)
- **Date sealed**: 2026-08-06
- **Token scope**: CI/CD automation for pdftract repository

## Usage

SealedSecrets are automatically decrypted by the sealed-secrets controller in the cluster.
Apply the manifest to create the actual Secret:

```bash
kubectl --kubeconfig ~/.kube/iad-ci.kubeconfig apply -f .ci/sealed-secrets/forgejo-ci-token.yaml
```

## Regenerating

If a secret needs to be re-sealed:

1. Create a standard Kubernetes Secret manifest
2. Use kubeseal with the appropriate cluster context:

```bash
kubeseal --kubeconfig ~/.kube/rs-manager.kubeconfig \
  --controller-name sealed-secrets-controller \
  --controller-namespace sealed-secrets \
  --format yaml < secret.yaml > sealed-secret.yaml
```

Note: The sealed-secrets controller runs on rs-manager cluster. Secrets are sealed there
and can be applied to any cluster with the same certificate.

## Security

SealedSecrets can be safely committed to git as they are encrypted with the cluster's
public key. Only the sealed-secrets controller can decrypt them.
