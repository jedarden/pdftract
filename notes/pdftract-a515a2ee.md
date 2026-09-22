# sdk-conformance-verify negative-gate proof — consolidated verification note

**Parent bead:** pdftract-a515a2ee ("Prove failing conformance command fails
sdk-conformance-verify workflow") · grandparent pdftract-854dc50c
**Written by:** pdftract-4ebbfa96 (proof-mapping child), 2026-09-22
**Evidence chain:** pdftract-e2ba7365 (derive submission spec) →
pdftract-cb28b9c8 (submit run) → pdftract-da7bf8c2 (capture Failed-phase
evidence) → pdftract-4ebbfa96 (this note: consolidation + AC mapping)

## 1. Verdict

**The negative gate works.** A standalone `sdk-conformance-verify` run whose
conformance command deliberately exits non-zero lands the workflow in
**phase `Failed`**, with the failing node `run-conformance` surfacing the
non-zero exit verbatim, **attempts == 1** (no retry anywhere), and **no
`continueOn` / `retryStrategy` at any level** — nothing softened or swallowed
the failure. All three parent acceptance criteria PASS.

| Workflow | `sdk-conformance-verify-manual-m2nvb` |
|---|---|
| Namespace | `argo-workflows` (iad-ci) |
| Template | `sdk-conformance-verify` (WorkflowTemplate, live gen=2, uid `ea2af6e9-d124-4a28-8f5c-a775a0efd8ea`, ArgoCD instance `argo-workflows-ns-iad-ci`) |
| Submitted by | pdftract-da7bf8c2 (sanctioned write path: `kubectl create` of an Argo Workflow via `~/.kube/iad-ci.kubeconfig`) |
| Parameters (stored verbatim) | `runtime-image=alpine:3.19` · `sdk-repo=https://github.com/octocat/Hello-World.git` · `sdk-ref=master` · `conformance-command=echo CONFORMANCE-DELIBERATE-FAIL-pdftract-da7bf8c2; exit 3` · `working-dir=.` |
| Window | started 2026-09-22T14:01:00Z → finished **2026-09-22T14:03:12Z** (~2m12s; 7200s backstop nowhere near) |
| **End state** | **`phase: Failed`** · `message: child 'sdk-conformance-verify-manual-m2nvb-2154854314' failed` |

## 2. Run timeline (two submissions, one demonstration)

1. **2026-09-22T07:20:41Z** — pdftract-cb28b9c8 submitted
   `sdk-conformance-verify-manual-q2fml` (identical parameters, marker tag
   `pdftract-cb28b9c8`) but did not poll (out of that child's scope).
   Attempts 1–2 of pdftract-da7bf8c2 found it **already reaped** by the
   template's `ttlStrategy: secondsAfterFailure=7200` before any node evidence
   could be captured (`NotFound` on single-GET at 13:54Z; re-confirmed
   `NotFound` at 14:19Z this dispatch). No workflow-level claim rested on
   q2fml.
2. **2026-09-22T14:01:00Z** — pdftract-da7bf8c2 submitted
   `sdk-conformance-verify-manual-m2nvb`, arguments verbatim-identical to the
   q2fml spec derived by pdftract-e2ba7365 (marker swapped to its own bead id,
   explicitly permitted by that spec). Captured **~19s after terminal phase**
   (14:03:31Z), ~2h59m inside the reap window; full live YAML (15767 bytes)
   preserved and committed.

The evidence below was **re-verified live by this dispatch at
2026-09-22T14:19–14:23Z** via single-GETs against
`http://traefik-iad-ci:8001` (read-only), and cross-checked field-by-field
against the committed artifact — the two agree exactly.

## 3. Node-level evidence (complete tree, quoted)

Complete node tree — 5 nodes, **each template step executed exactly once**:

| Node id (suffix) | Type | Template | Phase | Window (UTC) | Message |
|---|---|---|---|---|---|
| `…m2nvb` (root) | Steps | sdk-conformance-verify | **Failed** | 14:01:00 → 14:03:12 | `child 'sdk-conformance-verify-manual-m2nvb-2154854314' failed` |
| `…-1918951762` | Pod | clone-sdk | Succeeded | 14:01:00 → 14:02:30 | (none) |
| `…-999261260` | StepGroup [0] | — | Succeeded | 14:01:00 → 14:02:40 | (none) |
| `…-1066224641` | StepGroup [1] | — | **Failed** | 14:02:40 → 14:03:12 | `child '…-2154854314' failed` |
| `…-2154854314` | Pod | **run-conformance** | **Failed** | 14:02:40 → 14:03:01 | **`main: Error (exit code 3)`** |

The failing node message is `main: Error (exit code 3)` — the deliberate
non-zero exit surfaced **verbatim**, and is distinguishable from the
template's own exit-1 guards (missing parameter, `:latest` rejection,
working-dir missing) precisely because 3 can only come from the conformance
command. `clone-sdk` Succeeded first, which isolates the failure to the
conformance-command step: the run failed for exactly the reason it was
designed to fail.

## 4. Nothing softened the failure — positive proof of absence

- **attempts == 1:** the node tree above contains no `Retry`-type node and no
  node with attempts > 1; every one of the 5 nodes ran once. Re-confirmed
  live 14:19Z.
- **`continueOn`: absent everywhere.** Recursive scan of the live workflow
  spec (every nesting level, parsed JSON): **0 occurrences**.
  `status.storedTemplates` (3 stored templates): contains no
  `continueOn`. The committed artifact: 0 grep matches. The
  WorkflowTemplate itself: verified absent by pdftract-e2ba7365 (grep 0
  matches + recursive structural walk of the live template spec, 12/12
  validation checks PASS).
- **`retryStrategy`: absent everywhere.** Same three checks, same result: 0
  occurrences at every level.
- **Failure propagated correctly up the tree:** Pod
  (`run-conformance`, exit 3) → StepGroup [1] Failed → root Steps Failed →
  `status.phase: Failed` + `status.message` names the failing child. No
  mechanism anywhere in the chain swallowed the exit code.
- **Not Succeeded, not stuck:** phase reached `Failed` terminally in ~2m12s;
  `finishedAt=2026-09-22T14:03:12Z`. Post-capture re-confirmation GET at
  ~14:12Z and this dispatch's GETs at 14:19–14:23Z all read `Failed`.

## 5. Explicit mapping to parent acceptance criteria (pdftract-a515a2ee)

### AC-1 — Failing conformance command ends phase Failed → **PASS**
`sdk-conformance-verify-manual-m2nvb`, run with
`conformance-command=…; exit 3`, ended `status.phase=Failed` terminally at
2026-09-22T14:03:12Z (~2m12s total, clone included). Not `Succeeded`, not
stuck `Running` anywhere near the 7200s `activeDeadlineSeconds` backstop.
Evidence: live single-GET (re-confirmed 14:23Z) + committed artifact
`notes/pdftract-da7bf8c2-sdk-conformance-verify-manual-m2nvb.yaml` (commit
`05eff553`, HEAD of origin/main).

### AC-2 — Node-level unsoftened failure evidence → **PASS**
Quoted above (§3, §4): `run-conformance` Pod node
`sdk-conformance-verify-manual-m2nvb-2154854314`, `phase: Failed`,
`message: main: Error (exit code 3)` (non-zero exit surfaced verbatim);
attempts == 1 (5-node tree, no retry nodes); `continueOn`/`retryStrategy`
absent from the live workflow spec, from `status.storedTemplates`, from the
committed artifact, and from the WorkflowTemplate itself.

### AC-3 — Handling if the workflow unexpectedly Succeeds → **PASS (condition not triggered)**
The run did **not** Succeed — the exit-code-swallowing defect this criterion
guards against was not observed, so the manifest-fix / record-FAIL branch
never fired and no declarative-config change is required. The criterion's
intent — "never close claiming the gate works without this run demonstrating
it" — is satisfied: this run demonstrates the gate works. Had it Succeeded,
the full node status was already captured (committed artifact) and the defect
path was documented by pdftract-e2ba7365 §7.

**WARN/infra items (none block any AC):**
- *q2fml evidence window missed (infra, superseded):* the first submission was
  reaped by `ttlStrategy secondsAfterFailure=7200` before evidence capture;
  failure-count inflation on the parent chain came from this, not from any
  workflow defect. Superseded by m2nvb, which carries the demonstration.
- *Live-object TTL:* m2nvb itself is reaped ~2026-09-22T16:03Z. The durable
  evidence is the committed artifact at
  `notes/pdftract-da7bf8c2-sdk-conformance-verify-manual-m2nvb.yaml` (15767
  bytes, commit `05eff553`), which this dispatch parsed field-by-field and
  found identical to the live object on every quoted field.

## 6. Provenance

- **Committed artifact:** `notes/pdftract-da7bf8c2-sdk-conformance-verify-manual-m2nvb.yaml`
  — full live workflow JSON→YAML captured 14:03:31Z, commit `05eff553`
  (`test(pdftract-da7bf8c2): preserve Failed-phase node evidence…`), pushed;
  `git rev-list origin/main..HEAD` empty at write time.
- **Children's close evidence:** pdftract-e2ba7365 notes (submission spec +
  12/12 validation + template unsoftenability), pdftract-cb28b9c8 notes
  (submission + reuse check), pdftract-da7bf8c2 notes (capture + AC
  PASS/FAIL) — all closed.
- **Verification commands this dispatch** (all read-only; exit codes recorded
  in the closing bead's `verified:` block): live workflow single-GET + jq
  phase/message/node-tree extraction; recursive `continueOn`/`retryStrategy`
  scan of live spec JSON; PyYAML parse of the committed artifact's node tree;
  `git rev-list origin/main..HEAD` (empty).
- **Cluster writes in the whole chain:** exactly one Workflow create (m2nvb,
  by pdftract-da7bf8c2, via the one sanctioned path). This dispatch wrote
  nothing to the cluster and no repo code — the note file and the closing
  bead's notes field are its evidence.
