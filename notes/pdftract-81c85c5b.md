# pdftract-81c85c5b — pdftract-ci verify reaches Succeeded end to end

Status: IN PROGRESS (dispatch 1, 2026-10-26)

## Failures found and fixed (chronological)

### 1. setup: "artifact storage is not configured" (exit 64) — FIXED
- Evidence: `pdftract-main-verify-t65xr` (commit abcabed6) and
  `pdftract-main-verify-42xzg` (commit cc21f932) — both `setup` nodes:
  `wait: Error (exit code 64): artifact storage is not configured; see the
  docs ... configure-artifact-repository/`.
- Root cause: the iad-ci Argo controller configmap
  (`argo-workflows-iad-ci-workflow-controller-configmap`) defines **no default
  `artifactRepository`**, while `setup` declares an output artifact
  (`workflow-metadata`). The first artifact save failed the wait container.
  Sibling templates (`pdftract-build-binaries`, `pdftract-release-cascade`,
  `pdftract-github-release`) all carry a workflow-level
  `spec.artifactRepositoryRef` pointing at the Garage-backed
  `ci-release-artifact-repositories` configmap; `pdftract-ci` was missing it.
- Fix: added `artifactRepositoryRef: {configMap: ci-release-artifact-repositories, key: release-v1}`
  to BOTH copies (pdftract `.ci/argo-workflows/pdftract-ci.yaml`,
  declarative-config `k8s/iad-ci/argo-workflows/pdftract-ci.yaml`).
- Proof: `pdftract-main-verify-2glcd` (commit 1e3f7698, old code, fixed
  template — submitted after ArgoCD sync): **setup Succeeded** — first green
  setup since the 2026-10-06/07 template changes.

### 2. workflow-source-drift: exit 128 — root cause under investigation
- Evidence: `pdftract-main-verify-2glcd` `workflow-source-drift` node:
  `main: Error (exit code 128)`; downstream steps all `Omitted`; workflow
  Failed. Pod logs lost to podGC OnPodCompletion/5s (short-lived pod, not
  scraped by the log shipper).
- Pre-existing drift that WOULD have failed this gate regardless (found and
  fixed preemptively): the `repo-ci-semaphores/pdftract` `synchronization`
  block (b1a6954e8) existed only in the deployed copies — back-ported to
  `.ci/argo-workflows/pdftract-ci.yaml` and `.ci/argo-workflows/pdftract-nightly-fuzz.yaml`.
  `check-argo-workflow-drift.sh --live` passes locally against both copies
  and both live CronWorkflows.
- Remaining suspicion: the gate's anonymous partial clone of declarative-config
  from inside iad-ci (setup authenticates the same host and works). Debug
  probe workflow `pdftract-drift-debug-5nh45` submitted to isolate anonymous
  vs authed clone from the cluster. Result: PENDING (queued behind node capacity).

## Artifacts
- declarative-config commit `efae00dc5` (artifactRepositoryRef; pushed, ArgoCD-synced, verified on cluster)
- pdftract commit `cbe6296a` (template reconciliation; pushed; triggered release `pdftract-main-release-bmrzd`)

---
Split (2026-10-26 ~16:55Z, attempt 4): auto-split into a 5-child sequential chain; this bead converted to umbrella (labels umbrella, auto-split-parent; blocked-by the last child). Children, each owning one stage group of the pdftract-main-verify-* DAG:
- pdftract-810a6a81: Verify setup and workflow-source-drift green on a fresh pdftract main verify run
- pdftract-90fed7da: Fix sdk-conformance, mcp-client-smoke and diagnostics-conformance nodes to green
- pdftract-4a6dbfe2: Fix build-matrix, test-matrix and fixture-acceptance nodes to green
- pdftract-fcef0209: Fix quality-matrix, wasm32-check and wer-gate nodes to green
- pdftract-a151afe9: Green the pdftract-ci artifact tail and land the first full Succeeded run
Chain: 810a6a81 <- 90fed7da <- 4a6dbfe2 <- fcef0209 <- a151afe9 <- this bead. Each child closes only with gate-visible evidence (in-run node phases); a notes-field update before close is mandatory for no-code-change children.
Evidence snapshot at split time: pdftract-main-verify-k5c4x (commit 10c0a4af, created 16:12:47Z) - post-pending-forgejo Succeeded, setup Succeeded, workflow-source-drift Succeeded (the declarative-config f5bcdcf9c / pdftract cec8c320 credential-helper fix works in-run); sdk-conformance (10-language fan-out), build-matrix (5 targets), test-matrix (glibc/musl) still Running at ~16:45Z; tail nodes not yet instantiated (Argo lazy node creation).
