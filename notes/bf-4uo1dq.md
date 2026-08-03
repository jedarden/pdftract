# bf-4uo1dq: Require conformance-run link before closing generated-SDK beads

## Summary

Per ADR-001 in pdftract-ruby/docs/plan/plan.md, this bead updates the acceptance-criteria convention for subprocess-generated SDK completion beads to require attaching a link to a passing containerized conformance run before closure, not structural code presence alone.

## Context

The Ruby SDK bead (pdftract-45vo7) was closed on structural grounds only ("all 9 contract methods exposed on the Pdftract module"). The SDK turned out to be **100% non-functional at runtime**:
- LoadError on require (wrong `require_relative` paths in template)
- Entire API accidentally private (stray `private` keyword in template)
- BytesSource a stub (NotImplementedError)

These bugs lived in the Tera templates, affecting **all 8 subprocess-based SDKs** generated from the same pipeline.

## Audit Findings: All 8 SDK Beads Closed Without Conformance

| Bead | SDK | Language | Close Reason Pattern | Runtime Evidence |
|------|-----|-----------|---------------------|------------------|
| pdftract-45vo7 | Ruby | Ruby | "All 9 contract methods exposed" | None - language not installed |
| pdftract-2m3gl | PHP | PHP | "All methods implemented" + WARN | ⚠️ "PHP not installed locally, cannot run vendor/bin/phpunit" |
| pdftract-1w22d | .NET | C# | "All 9 contract methods" + WARN | ⚠️ "needs .NET SDK machine for build/test verification" |
| pdftract-5lvpu | Swift | Swift | "9 contract methods generated" + WARN | ⚠️ "swift test cannot run locally - Swift not installed" |
| pdftract-2v2d0 | Node.js | TypeScript | "All 9 contract methods implemented" + WARN | ⚠️ "npm toolchain unavailable for testing" |
| pdftract-2pyln | Go | Go | "All 9 contract methods exposed" + WARN | ⚠️ "pkg.go.dev page awaits separate publish workflow bead" |
| pdftract-32qkr | Java | Java/Kotlin | "All 9 contract methods exposed" + WARN | ⚠️ "GitHub repo needs to be created" |
| (duplicate) | - | - | - | - |

**Pattern**: All 8 beads closed with ✅ for structural requirements (methods exposed, classes defined) but ⚠️ for runtime conformance, citing "language not installed locally" or "toolchain unavailable."

## Root Cause

The existing acceptance criteria already **named** a runtime check (e.g., `bundle exec rake test:conformance` must "100% pass"), but nothing enforced it. Beads were closed without the conformance suite ever actually executing, as the language toolchains weren't available on the build server.

## ADR-001 Decision (from pdftract-ruby/docs/plan/plan.md)

A generated-SDK bead may not be closed as complete, and a `<lang>-sdk-publish` Argo Workflow may not proceed past its conformance step, without a **containerized runtime execution** of the shared conformance suite against that language's official Docker image.

Concretely:
- Run conformance inside `ruby:3.2-slim`, `node:22-slim`, `golang:1.22`, etc. on iad-ci
- Attach pass/fail output (Argo Workflow log or link) to the bead as acceptance evidence
- Treat closing without this evidence as a hygiene defect (same class as closing with evidence attached but red)

This generalizes as a parameterized `sdk-conformance-verify` WorkflowTemplate in `jedaren/declarative-config` (k8s/iad-ci/argo-workflows/).

## Implementation: New Acceptance-Criteria Convention

### For SDK Completion Beads

All beads implementing/completing a subprocess-generated SDK (Ruby, PHP, .NET, Swift, Node, Go, Java) MUST include the following **hard gate** in their acceptance criteria:

```
### ✅ HARD GATE: Containerized Conformance Execution
- [ ] Conformance suite runs inside <official-language-image>:<version> on iad-ci
- [ ] 100% of tests pass
- [ ] Argo Workflow run link or log output attached to this bead
- [ ] Bead CANNOT close without this evidence attached
```

The `<official-language-image>` examples:
- Ruby: `ruby:3.2-slim`
- Node.js: `node:22-slim` (LTS)
- Go: `golang:1.22`
- Java: ` eclipse-temurin:17-jdk`
- PHP: `php:8.3-cli`
- .NET: `mcr.microsoft.com/dotnet/sdk:8.0`
- Swift: `swift:5.9`

### For SDK Publish WorkflowTemplates

All `<lang>-sdk-publish` WorkflowTemplates in `jedarden/declarative-config/k8s/iad-ci/argo-workflows/` MUST include:

```yaml
- name: conformance
  template: sdk-conformance-verify
  arguments:
    parameters:
      - name: language
        value: "{{inputs.parameters.language}}"
      - name: version
        value: "{{inputs.parameters.version}}"
  # Workflow aborts if conformance fails
```

Where `sdk-conformance-verify` is a reusable, parameterized template that:
1. Clones the SDK repo
2. Runs the language's native test command (e.g., `bundle exec rake test:conformance`, `npm test -- conformance`)
3. Fails the workflow on any non-zero exit code
4. Produces logs that can be linked from the bead

## Action Items

1. ✅ **Document findings**: This note records the audit of all 8 SDK beads
2. ✅ **Annotate pdftract-45vo7**: Added hygiene defect comment (poster child for the bug class)
3. ✅ **Annotate other 6 SDK beads**: Added comments to pdftract-5lvpu, pdftract-2m3gl, pdftract-1w22d, pdftract-32qkr, pdftract-2pyln, pdftract-2v2d0 noting they closed without conformance
4. ⏳ **Wire sdk-conformance-verify WorkflowTemplate**: Parameterized by language image (tracked as bead bf-5rxgxg)
5. ✅ **Plan.md already updated**: SDK Acceptance Criteria section at line 3681 already contains the hard-gate language

## References

- ADR-001: Containerized conformance execution as hard gate (pdftract-ruby/docs/plan/plan.md, line 71)
- Plan section: SDK Architecture / SDK Acceptance Criteria (pdftract/docs/plan/plan.md, line ~3581)
- Related bead: bf-5rxgxg (Wire shared sdk-conformance-verify Argo WorkflowTemplate)
- Ruby audit findings: pdftract-ruby/docs/notes/ dated 2026-07-20

## Acceptance Criteria Status

### ✅ PASS
- All 7 SDK completion beads audited for closure pattern
- ADR-001 decision documented and understood
- New acceptance-criteria convention defined
- Action items completed: all 7 beads annotated with hygiene defect comments
- plan.md SDK Acceptance Criteria section confirmed already updated (line 3681)

### ⏳ PENDING (blocks bead closure)
- [x] Add comments to pdftract-45vo7 explicitly noting the closure was incorrect per ADR-001 ✅ DONE
- [x] Add comments to the other 6 SDK beads (pdftract-2m3gl, pdftract-1w22d, pdftract-5lvpu, pdftract-2v2d0, pdftract-2pyln, pdftract-32qkr) noting they closed without conformance ✅ DONE
- [x] Update plan.md SDK Acceptance Criteria section with the hard-gate language ✅ ALREADY PRESENT (line 3681)
- [ ] Commit this verification note

### ⚠️ WARN
- This audit focused on the 8 subprocess-based SDKs; the in-process SDKs (Python via PyO3, C via FFI) may have similar issues but were not audited here
- The sdk-conformance-verify WorkflowTemplate (bf-5rxgxg) is currently blocked; implementing it is a prerequisite for enforcing the new convention

## Test Hygiene Note

Per ADR-001, the goal is to prevent a repeat of the Ruby SDK situation where structural completeness ("all 9 methods exposed") masked complete runtime failure. The containerized conformance execution requirement ensures that **every SDK bead that closes has actually been run** and verified to work, not just reviewed statically.

This aligns with the test-hygiene principles documented elsewhere: never close a bead claiming "tests pass" when the tests were never actually executed.
