# SDK Conformance Test Runner Pattern

This document describes the conformance test runner pattern that every SDK implements for pdftract.

## Overview

The conformance test suite is the SDK API contract. Every SDK must implement a test runner that:

1. Loads the shared `tests/sdk-conformance/cases.json` file
2. Iterates through test cases
3. Invokes the SDK's native methods with the case's options
4. Compares the result against expected values with tolerances
5. Reports per-case pass/fail/skip/error status
6. Emits a machine-readable JSON summary (`conformance-report.json`)

## Conformance Report Schema

See `tests/sdk-conformance/report-schema.json` for the full JSON schema.

Key fields:
- `sdk`: SDK name (e.g., "pdftract-py", "pdftract-node")
- `sdk_version`: SDK version that produced the report
- `suite_version`: Version of the conformance suite run
- `results`: Array of per-case results with `id`, `status`, `actual`, `expected`, `error`, `reason`, `duration_ms`
- `summary`: Aggregate counts for `total`, `passed`, `failed`, `skipped`, `errors`
- `environment`: OS, arch, binary version, runtime version

## Per-Language Runners

| SDK | Path | Test Framework | CLI Command |
|-----|------|----------------|-------------|
| Rust | `crates/pdftract-cli/tests/conformance.rs` | cargo test | `cargo test --test conformance` |
| Python | `tests/conformance/test_conformance.py` | pytest | `pytest tests/conformance/test_conformance.py -v` |
| Node.js | `tests/conformance/conformance.test.ts` | vitest | `vitest test/conformance/conformance.test.ts` |
| Go | `tests/conformance/conformance_test.go` | go test | `go test -v ./conformance_test.go` |
| Java | `tests/conformance/ConformanceTest.java` | JUnit 5 | `mvn test -Dtest=ConformanceTest` |
| .NET | `tests/conformance/ConformanceTests.cs` | xUnit | `dotnet test --filter ConformanceTests` |
| C | `tests/conformance/conformance.c` | standalone binary | `./conformance [suite-path] [output-path]` |
| Ruby | `tests/conformance/conformance_test.rb` | minitest | `ruby test/conformance/conformance_test.rb` |
| PHP | `tests/conformance/ConformanceTest.php` | PHPUnit | `./vendor/bin/phpunit tests/ConformanceTest.php` |
| Swift | `tests/conformance/ConformanceTests.swift` | XCTest | `swift test --filter ConformanceTests` |

## Shared Comparison Logic

All runners implement the same comparison logic with tolerances:

### Numeric Comparison with Tolerance

```pseudocode
function compare_with_tolerance(actual, expected, tolerance):
    if tolerance is null:
        return abs(actual - expected) < EPSILON

    if tolerance.abs exists:
        if abs(actual - expected) <= tolerance.abs:
            return true

    if tolerance.rel exists:
        diff = abs(actual - expected)
        avg = (actual + expected) / 2.0
        if avg > 0.0 and diff / avg <= tolerance.rel:
            return true

    return false
```

### Wildcard Path Matching

Tolerances use JSONPath-like wildcard syntax:
- `pages[*].blocks[*].bbox` matches all bbox values
- `pages[0].spans[*].confidence` matches all confidence values in page 0

### Expected Value Constraints

The expected object supports special constraint fields:

| Field | Type | Description |
|-------|------|-------------|
| `min` | number | Minimum numeric value |
| `max` | number | Maximum numeric value |
| `value` | number | Exact value (with tolerance) |
| `min_length` | number | Minimum string/array length |
| `contains` | array | String must contain all substrings |
| `min` | number | Minimum array length |
| `max` | number | Maximum array length |

## Test Case Execution Flow

1. Load test case from suite
2. Check `min_schema_version` - skip if SDK schema is too old
3. Resolve fixture path (handle remote URLs)
4. Execute SDK method with options
5. Compare result against expected with tolerances
6. Record result with timing
7. Emit final report

## Exit Codes

- `0`: All tests passed (or all failures were skips)
- `1`: One or more tests failed or errored

## CI Integration

The per-SDK Argo publish workflow MUST run the conformance runner BEFORE publishing. A failed runner aborts the publish step.

Example Argo step:

```yaml
- name: conformance
  template: conformance-runner
  arguments:
    parameters:
    - name: sdk
      value: pdftract-py

- name: publish
  template: publish-to-pypi
  dependencies:
  - conformance
  when: "{{steps.conformance.exitCode}}"
```

## Standalone Ad Hoc Verification (sdk-conformance-verify WorkflowTemplate)

The publish-flow wiring above is per-SDK and aspirational until each publish workflow
lands. For manual verification of an SDK checkout — e.g. producing the
passing-conformance-run link that generated-SDK beads MUST attach before closure
(plan line 3732) — submit the shared `sdk-conformance-verify` WorkflowTemplate
directly on iad-ci. The template is defined in
`jedarden/declarative-config → k8s/iad-ci/argo-workflows/sdk-conformance-verify-workflowtemplate.yml`
and synced by ArgoCD into namespace `argo-workflows`. A standalone run clones an SDK
repository at a requested ref and executes its conformance command inside a
caller-selected runtime image. It depends on nothing else — no publish workflow and no
language-specific wiring — which makes it the shape to use for ad hoc and manual
verification.

### Parameters (verified against the live template, 2026-09-22)

| Parameter | Required | Default | Meaning |
|-----------|----------|---------|---------|
| `runtime-image` | yes | — | Pinned language runtime image the suite runs in (e.g. `ruby:3.2-slim`, `node:22-slim`). Must carry an explicit tag; `:latest` is banned org-wide and rejected by the template's clone guard. |
| `sdk-repo` | yes | — | Git URL of the SDK repository to clone (anonymous https only). |
| `sdk-ref` | no | `main` | Branch or tag to check out; other refs fall back to a full clone + detached checkout. |
| `conformance-command` | yes | — | Shell command string, run from `working-dir`. MUST exit non-zero when any case fails — its exit code is the whole gate. |
| `working-dir` | no | `.` | Directory under the checkout the command runs from. |

Omitting any required parameter fails the run immediately
(`inputs.parameters.<name> was not supplied`) instead of running a meaningless gate.

### Submitting with the argo CLI

```bash
argo submit -n argo-workflows --from workflowtemplate/sdk-conformance-verify \
  -p runtime-image=ruby:3.2-slim \
  -p sdk-repo=https://github.com/jedarden/pdftract-ruby.git \
  -p sdk-ref=main \
  -p conformance-command="gem install --no-document rake && rake conformance"
```

### Submitting with kubectl (repository-standard)

The repo-standard manual invocation is a bare Workflow carrying a
`workflowTemplateRef` (same shape as every other manual run in this repo):

```bash
kubectl --kubeconfig=/home/coding/.kube/iad-ci.kubeconfig create -f - <<EOF
apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  generateName: sdk-conformance-verify-manual-
  namespace: argo-workflows
spec:
  arguments:
    parameters:
      - name: runtime-image
        value: ruby:3.2-slim
      - name: sdk-repo
        value: https://github.com/jedarden/pdftract-ruby.git
      - name: sdk-ref
        value: main
      - name: conformance-command
        value: "gem install --no-document rake && rake conformance"
  workflowTemplateRef:
    name: sdk-conformance-verify
EOF
```

The `arguments.parameters` block is what an operator supplies; the template entrypoint
forwards every parameter to its `clone-sdk` and `run-conformance` steps verbatim.

### Reading the result

- Phase `Succeeded` means the conformance command exited 0; `Failed` means it exited
  non-zero or a guard/backstop fired. The template has no `retryStrategy` and no
  `continueOn`, so the verdict is always the command's own exit code.
- `podGC: OnPodCompletion` deletes step pods as they finish, and `ttlStrategy` reaps
  the Workflow object 1800s after success / 7200s after failure. Pull logs from the
  Argo UI (`https://argo-ci.ardenone.com`, VPN) or `kubectl logs` while the run is
  live, and capture the run link for the bead before the TTL expires.
- A workflow-level `activeDeadlineSeconds: 7200` backstops a run that hangs before or
  during execution.
- Under iad-ci controller/API-server starvation, a freshly created run can sit
  unnoticed by the controller. Poll with single-object GETs
  (`kubectl --server=http://traefik-iad-ci:8001 get workflow <name> -n argo-workflows`)
  rather than collection LISTs, and never stack a second run while one is still
  Pending.

### Worked examples

Smoke the plumbing itself — the exact shape live-proven on 2026-09-22:
`sdk-conformance-verify-manual-qhtmj` reached `Succeeded`, and
`sdk-conformance-verify-manual-m2nvb` reached `Failed` on a deliberate `exit 3`:

```text
runtime-image:       alpine:3.19
sdk-repo:            https://github.com/octocat/Hello-World.git
sdk-ref:             master
conformance-command: test -f README
```

Real SDK gate (Ruby; other languages substitute their row from the per-language table
above):

```text
runtime-image:       ruby:3.2-slim
sdk-repo:            https://github.com/jedarden/pdftract-ruby.git
sdk-ref:             main
conformance-command: gem install --no-document rake && rake conformance
```

To verify against the shared suite in this monorepo rather than whatever the SDK repo
vendors, clone the monorepo and point `working-dir` at the SDK:

```text
sdk-repo:     https://git.ardenone.com/jedarden/pdftract.git
working-dir:  pdftract-ruby
```

Caveats that remain the caller's responsibility:

- The template clones exactly one repository. If the runner cannot find its suite
  (Ruby reads `CONFORMANCE_SUITE`, default `tests/sdk-conformance/cases.json`,
  relative to `working-dir`), it skips every case and exits 0 — a vacuous pass that
  cannot fail the gate. Make sure the suite actually reaches the clone before reading
  `Succeeded` as conformance evidence.
- Private SDK repos are out of scope for the shared template (anonymous https clones
  only); language-specific publish workflows wire their own auth when they adopt it.

## README Integration

Each SDK's README should have a "Conformance" section that links to the latest published report:

```markdown
## Conformance

This SDK passes the official pdftract conformance suite. Latest report: [conformance-pdftract-py-0.1.0.json](https://argoproj.example/artifacts/conformance-pdftract-py-0.1.0.json)
```

## Stub Implementation Notes

The current runners contain stub implementations for `executeMethod()` that return placeholder values. These must be replaced with actual SDK calls when:

1. The SDK's native methods are implemented
2. The binary interface is stable
3. The JSON output schema is finalized

Until then, the runners serve as:
- A reference implementation pattern
- A starting point for SDK development
- Documentation of expected behavior

## Adding New Test Cases

To add a new test case to the suite:

1. Add the case to `tests/sdk-conformance/cases.json`
2. Bump `version` in the suite (if cases changed)
3. Update all SDK runners to handle the new case (if needed)
4. Verify all SDKs pass the updated suite before publishing

## References

- Plan section: SDK Architecture / The Conformance Suite, line 3547
- Plan section: SDK Acceptance Criteria, line 3589
- Plan section: SDK hard gate (conformance-run link requirement), line 3732
- Shared suite: `tests/sdk-conformance/cases.json`
- Report schema: `tests/sdk-conformance/report-schema.json`
- Template manifest: `jedarden/declarative-config → k8s/iad-ci/argo-workflows/sdk-conformance-verify-workflowtemplate.yml`
