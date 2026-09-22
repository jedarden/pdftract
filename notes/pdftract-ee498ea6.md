# pdftract-ee498ea6 — standalone sdk-conformance-verify invocation shape

Split child of parent pdftract-854dc50c. Scope: add the standalone ad hoc invocation
shape for the shared `sdk-conformance-verify` WorkflowTemplate — no language-specific
publish wiring.

## What was added

`docs/notes/sdk-conformance-runner.md` gains a **"Standalone Ad Hoc Verification
(sdk-conformance-verify WorkflowTemplate)"** section (inserted between "CI
Integration" and "README Integration", +128 lines, commit `1f79177a`):

- **Parameter reference** for the template — `runtime-image`, `sdk-repo`,
  `conformance-command` (all required, no defaults), `sdk-ref` (default `main`),
  `working-dir` (default `.`) — transcribed from the committed declarative-config
  manifest (`k8s/iad-ci/argo-workflows/sdk-conformance-verify-workflowtemplate.yml`)
  and cross-checked against the **live iad-ci object** via read-only single GET on
  2026-09-22 (entrypoint `sdk-conformance-verify`, 5 inputs, TTL 1800s/7200s,
  podGC OnPodCompletion, workflow deadline 7200s).
- **Both submit forms**: `argo submit -n argo-workflows --from
  workflowtemplate/sdk-conformance-verify -p ...`, and the repository-standard bare
  `kind: Workflow` + `workflowTemplateRef` kubectl heredoc
  (`--kubeconfig=/home/coding/.kube/iad-ci.kubeconfig`, `generateName:
  sdk-conformance-verify-manual-`, namespace `argo-workflows`) — the same shape the
  repo documents for every other manual run, and the exact shape the API server
  accepted for today's two live runs.
- **Result handling**: phase semantics (no retryStrategy / no continueOn — the
  command's exit code is the verdict), podGC/TTL reaping windows and where to pull
  logs before they expire, the 7200s deadline backstop, and single-GET polling under
  iad-ci controller starvation (the s7mbl/656vw lesson from sibling
  pdftract-71694636).
- **Worked examples**: the live-proven plumbing smoke (`alpine:3.19` +
  `octocat/Hello-World` + `test -f README` — `sdk-conformance-verify-manual-qhtmj`
  Succeeded, `sdk-conformance-verify-manual-m2nvb` Failed on deliberate exit 3, both
  2026-09-22), a real Ruby gate (`ruby:3.2-slim` + `gem install rake` +
  `rake conformance` — task verified in `pdftract-ruby/Rakefile` line 12), and the
  monorepo-clone variant (`sdk-repo` = `https://git.ardenone.com/jedarden/pdftract.git`,
  `working-dir: pdftract-ruby`) for verifying against the shared suite rather than a
  possibly-lagging SDK repo.
- **Caller-side caveats**: the vacuous-skip hazard (Ruby runner exits 0 when
  `CONFORMANCE_SUITE` is absent — cannot fail the gate) and anonymous-https-only
  cloning. References section gains the plan line 3732 entry and the template
  manifest path.

Scope guard: no `kind: Workflow` manifest was committed (ArgoCD would instantiate
it — the repo convention is WorkflowTemplates in-tree, invocations documented), and
no `<lang>`-publish wiring was added, per the bead scope.

## Verification (from clean git-archive HEAD extraction, exit 0)

Commands and results (run against `/var/tmp/ee498ea6-UI1I`, extraction of commit
`1f79177a`):

- Fence balance: 8 balanced code blocks — PASS
- `yaml.safe_load` of the publish-flow yaml block — PASS
- `yaml.safe_load` of the heredoc's inner Workflow: apiVersion
  `argoproj.io/v1alpha1`, kind `Workflow`, ns `argo-workflows`, generateName
  `sdk-conformance-verify-manual-`, `workflowTemplateRef: sdk-conformance-verify`,
  all five parameters — PASS
- Param cross-check: every required template parameter supplied, no unknown
  parameters, against the declarative-config manifest — PASS
- Live-object cross-check (read-only `kubectl --server=http://traefik-iad-ci:8001
  get workflowtemplate sdk-conformance-verify -n argo-workflows -o json`): live
  entrypoint inputs match the documented table exactly — PASS
- Fact checks: `git remote -v` origin = the documented monorepo URL;
  `pdftract-ruby/Rakefile` defines the `conformance` task with rake dev-dependency;
  `CONFORMANCE_SUITE` default string matches the doc — PASS

No Rust/Go/Node code was touched, so the language-default build/test DoD is not
applicable to this docs-only dispatch (same reasoning accepted for sibling
pdftract-fbe933c5). No new workflow was submitted: the invocation shape this bead
documents was live-proven end-to-end today by siblings pdftract-fbe933c5 (positive
leg) and pdftract-da7bf8c2 (negative leg) with byte-identical plumbing.

## Acceptance criteria

1. Operator can submit as an independent verification workflow via
   `workflowTemplateRef` (or `argo submit --from`) — **PASS** (both forms
   documented; shape accepted live by the API server today, qhtmj/m2nvb).
2. Invocation supplies required image + runtime parameters and forwards them —
   **PASS** (all three required params supplied in the example; entrypoint forwards
   each via `{{inputs.parameters.*}}`; omission fails fast — documented).
3. Suitable for manual verification, no publish-workflow dependency — **PASS**
   (standalone `kind: Workflow`; independence stated; no publish wiring added).
4. Follows existing repository conventions — **PASS** (repo-standard kubectl heredoc
   + `workflowTemplateRef` shape, iad-ci kubeconfig path and namespace as documented
   for every other manual run; example values fact-checked against vendored SDKs).

Commits: `1f79177a` (docs), this note. Evidence runs live-proven by siblings:
`notes/pdftract-fbe933c5-sdk-conformance-verify-manual-qhtmj.yaml`,
`notes/pdftract-da7bf8c2-sdk-conformance-verify-manual-m2nvb.yaml`.
