# Argo WorkflowTemplate promotion

`pdftract-ci` has two tracked copies:

- Source: `.ci/argo-workflows/pdftract-ci.yaml` in this repository.
- Deployed: `k8s/iad-ci/argo-workflows/pdftract-ci.yaml` in
  `jedarden/declarative-config`.

The source copy is the workflow contract. The deployed copy must contain the
same schema, DAG, container images, parameters, and quality-gate commands.
The only permitted deployment-only difference is a `resources:` map, because
the `iad-ci` resource-limit gate can require smaller requests than the source
defaults. The drift checker strips those maps structurally before comparing the
copies; a missing or extra map is still a failure. Resource changes remain
subject to declarative-config's `k8s/check-resource-limits.py iad-ci` check.

## Required promotion sequence

1. Edit and validate the source copy. Run the repository's definition of done
   and the drift test:

   ```sh
   scripts/definition-of-done.sh --fast
   .ci/scripts/test-argo-workflow-drift.sh
   ```

2. Commit and push the pdftract source change to Forgejo `origin/main`.

3. In a clean checkout of `jedarden/declarative-config`, update only
   `k8s/iad-ci/argo-workflows/pdftract-ci.yaml` from the same source revision.
   Preserve only the existing `resources:` overrides required by the cluster;
   do not hand-edit tasks, gates, images, parameters, or schema fields.

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

The deployed copy is checked out from declarative-config `main` by the
`workflow-source-drift` task before the rest of `pdftract-ci` runs. A mismatch
fails the workflow and is also a release dependency, so a tag cannot publish
from a stale WorkflowTemplate.
