# ADR-004: Keep Schema Validation in Argo CI

## Status

Accepted

## Context

GitHub Actions are disabled for pdftract. The former
`.github/workflows/schema-gen.yml` workflow only ran on GitHub-hosted runners
and was removed in commit `f3a8a7ff`. The JSON schema remains a compatibility
contract for extraction output, so removing that workflow must not remove the
validation capability.

## Decision

Schema generation and output validation are CI gates in the `pdftract-ci`
Argo WorkflowTemplate. The in-tree source is
`.ci/argo-workflows/pdftract-ci.yaml`, with the deployed copy in
`declarative-config/k8s/iad-ci/argo-workflows/pdftract-ci.yaml`.

The `quality-matrix` runs both gates:

- `schema-gen` regenerates the schema with `cargo xtask gen-schema` and fails
  when the generated `docs/schema/v1.0/pdftract.schema.json` differs from the
  committed file.
- `schema-validation` runs `ci/schema-gate.sh`, which executes the
  `json_schema` fixture test suite against the published schema.

The GitHub Actions workflow must not be recreated or used as a template.

## Consequences

Schema type changes must be followed by schema regeneration and a committed
schema update. Extraction-output regressions and schema drift fail the Argo CI
workflow before they can be accepted as a passing change. The capability is
maintained in the repository's supported CI path rather than a disabled
GitHub-hosted workflow.

## References

- `.ci/argo-workflows/pdftract-ci.yaml`
- `ci/schema-gate.sh`
- `docs/schema/v1.0/pdftract.schema.json`
- Removal commit: `f3a8a7ff`
