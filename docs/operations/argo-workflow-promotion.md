# Argo WorkflowTemplate promotion

The gated Argo manifests have two tracked copies:

- Authoritative sources: `.ci/argo-workflows/pdftract-ci.yaml`,
  `.ci/argo-workflows/pdftract-nightly-fuzz.yaml`, and
  `.ci/argo-workflows/pdftract-nightly-supply-chain.yaml`, plus
  `.ci/argo-workflows/pdftract-nightly-evidence-configmap.yaml` in this repository.
- Deployed copies: `k8s/iad-ci/argo-workflows/pdftract-ci.yaml`,
  `k8s/iad-ci/argo-workflows/pdftract-nightly-fuzz.yaml`, and
  `k8s/iad-ci/argo-workflows/pdftract-nightly-supply-chain.yaml`, plus
  `k8s/iad-ci/argo-workflows/pdftract-nightly-evidence-configmap.yaml` in
  `jedarden/declarative-config`.

The in-tree copies are authoritative workflow contracts. Their deployed copies
must contain the same schema, DAG, container images, parameters, schedules,
fuzz targets, and quality-gate commands. The only permitted deployment-only
difference is a `resources:` map, because the `iad-ci` resource-limit gate can
require smaller requests than the source defaults. The drift checker strips
those maps structurally before comparing each pair; a missing or extra map is
still a failure. Resource changes remain subject to declarative-config's
`k8s/check-resource-limits.py iad-ci` check.

## Required promotion sequence

1. Edit and validate the source copy. Run the repository's definition of done
   and the drift test:

   ```sh
   scripts/definition-of-done.sh --fast
   .ci/scripts/test-argo-workflow-drift.sh
   ```

2. Commit and push the pdftract source change to Forgejo `origin/main`.

3. In a clean checkout of `jedarden/declarative-config`, update only the three
   corresponding `k8s/iad-ci/argo-workflows/` manifests from the same source
   revision. Preserve only the existing `resources:` overrides required by the
cluster; do not hand-edit tasks, gates, images, parameters, schedules, or
schema fields.

4. From the pdftract checkout, validate the two checked-out copies before
   committing the deployment change:

   ```sh
   DECLARATIVE_CONFIG_DIR=/path/to/declarative-config \
     .ci/scripts/check-argo-workflow-drift.sh
   ```

5. Commit and push the declarative-config change to its configured `origin`.
   ArgoCD application `argo-workflows-ns-iad-ci` reconciles the
   `argo-workflows` namespace; do not apply or patch the WorkflowTemplate with
   `kubectl`.

All four deployed copies are checked out from declarative-config `main` by the
`workflow-source-drift` task before the rest of `pdftract-ci` runs. With
`--live`, the same task reads the two CronWorkflows from the `iad-ci` cluster's
`argo-workflows` namespace through the `devpod-observer/kubectl-proxy`
read-only service. It requires exactly `0 3 * * *` and `0 4 * * *` in UTC,
then compares each full live `spec` with the GitOps manifest. The source to
GitOps comparison permits only `resources:` maps to differ; the live to
GitOps comparison permits no spec differences, including resource values.
An absent CronWorkflow, failed read, schedule drift, or spec drift fails CI.

From codinghome, run the live check against the read-only iad-ci proxy:

```sh
ARGO_LIVE_API_BASE=http://traefik-iad-ci:8001 \
  .ci/scripts/check-argo-workflow-drift.sh --live
```

The focused fixture test is `sh .ci/scripts/test-argo-workflow-drift.sh`.
Record the declarative-config deployment commit (`git rev-parse origin/main`)
on the owning application bead after promotion so the checked revision can be
distinguished from an unpromoted source commit.
