# Nightly run evidence

The `iad-ci` cluster's `argo-workflows-ns-iad-ci` ArgoCD Application deploys
`pdftract-nightly-fuzz` and `pdftract-nightly-supply-chain`. Each CronWorkflow
runs an `onExit` handler before pod GC. It writes one archive to the private
`needle-ci-artifacts` Garage bucket:

```text
s3://needle-ci-artifacts/pdftract/nightly/<schedule>/<workflow-name>/evidence.tgz
```

The explicit S3 artifact uses the existing write-only
`needle-ci-local-artifact-publisher` Secret. A Garage bucket lifecycle rule
deletes objects under this prefix after 14 days, including when a schedule
stops. Argo ArtifactGC is set to `Never`, so the Workflow's TTL does not
remove the archive.
The collector's temporary output is capped at 1 GiB for fuzz and 128 MiB for
supply-chain, preserving space in the shared 20 GiB bucket. An oversized run
fails its exit handler visibly instead of silently dropping files.

`manifest.json` records the Workflow name, schedule, final phase, source commit
(or `null` when clone logs were unavailable), task pod
and container exit codes, and SHA-256/size references for task logs. Fuzz
archives additionally contain each generated `coverage.profdata` and every
`crash-*`, `leak-*`, and `timeout-*` file. An empty `crash_set` is an explicit
empty result; check that `coverage` is nonempty before treating it as a
completed fuzz coverage run. Task logs are limited to 16 MiB per container;
each log reference records whether that limit was reached. Logs are redacted
for common credential forms and the supply-chain clone token before upload.
No Kubernetes pod spec,
Secret value, or command environment is archived.

## Retrieve an archive after pod or Workflow deletion

Use a Kubernetes context allowed to create, exec into, and delete an ephemeral
Pod in `argo-workflows`. The reader Pod references the existing read-only
Garage Secret. The observer proxy on codinghome cannot create Pods, so it
cannot perform this retrieval itself. The script deletes the Pod on exit and
never prints or stores the Secret value locally.

```bash
mkdir -p nightly-evidence
.ci/scripts/retrieve-nightly-evidence.sh \
  pdftract-nightly-supply-chain \
  pdftract-nightly-supply-chain-<run-suffix> \
  nightly-evidence/supply-chain.tgz
mkdir -p nightly-evidence/supply-chain
tar -xzf nightly-evidence/supply-chain.tgz -C nightly-evidence/supply-chain
jq '{workflow, source_revision, final_phase, pods, task_logs, coverage, crash_set, missing_logs}' \
  nightly-evidence/supply-chain/manifest.json
```

Replace the schedule and Workflow name with `pdftract-nightly-fuzz` for fuzz
evidence. Extract its archive the same way; `coverage/<target>/coverage.profdata`
and `crashes/<target>/<artifact>` are retrievable paths. The script performs an
S3 `HeadObject`, downloads the archive, and checks its tar integrity. Use the
recorded SHA-256 values to check individual extracted files. For example:

```bash
python3 - nightly-evidence/supply-chain <<'PY'
import hashlib, json, pathlib, sys
root = pathlib.Path(sys.argv[1])
manifest = json.loads((root / 'manifest.json').read_text())
for category in ('task_logs', 'coverage', 'crash_set'):
    for artifact in manifest[category]:
        data = (root / artifact['path']).read_bytes()
        assert len(data) == artifact['bytes']
        assert hashlib.sha256(data).hexdigest() == artifact['sha256']
print('archive contents verified')
PY
```

The source manifests live at `.ci/argo-workflows/pdftract-nightly-fuzz.yaml`,
`.ci/argo-workflows/pdftract-nightly-supply-chain.yaml`, and
`.ci/argo-workflows/pdftract-nightly-evidence-configmap.yaml`. Their deployment
copies are under `jedarden/declarative-config/k8s/iad-ci/argo-workflows/`.
Run `.ci/scripts/check-argo-workflow-drift.sh` after promotion.
