# Verification Note: pdftract-i9rk

## Bead
CONTRIBUTING.md — Argo-CI caveat for forks, local validation checklist, PR template requirements

## Summary
Created and updated contributor documentation to ensure first-time contributors can submit properly-formatted PRs without surprises.

## Files Created/Modified

### Created:
1. **CODE_OF_CONDUCT.md** — Contributor Covenant v2.1 (5,519 bytes)
2. **.github/PULL_REQUEST_TEMPLATE.md** — PR template with required fields (1,988 bytes)
3. **.github/ISSUE_TEMPLATE/bug_report.md** — Bug report template (1,532 bytes)
4. **.github/ISSUE_TEMPLATE/feature_request.md** — Feature request template (1,373 bytes)
5. **.github/ISSUE_TEMPLATE/performance_regression.md** — Performance regression template (1,974 bytes)

### Modified:
1. **CONTRIBUTING.md** — Completely restructured with all required sections (11,294 bytes)
2. **README.md** — Added Contributing section with link to CONTRIBUTING.md

## Acceptance Criteria Status

### PASS
- [x] CONTRIBUTING.md exists at repo root
- [x] All nine sections from bead description are present:
  1. Project licensing (dual MIT OR Apache-2.0)
  2. Code of conduct (link to CODE_OF_CONDUCT.md)
  3. Reporting security issues (link to SECURITY.md)
  4. Development setup (with OCR dependencies for Phase 5 features)
  5. Local validation expected before opening a PR (6 commands matching pdftract-ci)
  6. CI on forks (the Argo-CI caveat)
  7. PR template requirements
  8. Commit message style (Conventional Commits)
  9. Issue triage
- [x] The Argo-CI caveat is in a clearly visible Markdown blockquote (`> **IMPORTANT:**`)
- [x] Local-validation commands exactly match the pdftract-ci workflow steps
- [x] A first-time contributor can read CONTRIBUTING.md and submit a properly-formatted PR without surprises
- [x] Linked from README (Contributing section added)
- [x] Linked from .github/ISSUE_TEMPLATE/ (all templates have footer links)
- [x] Linked from PR template (footer link added)

### Key Features Implemented
- DCO sign-off requirement clearly documented with `git commit -s` example
- 48-hour maintainer-triggered CI response window documented
- Argo-CI caveat explains why external forks cannot self-trigger CI
- Local validation checklist matches CI workflow: test, clippy, bloat, audit, deny, fmt
- Conventional Commits format documented with examples
- All issue templates include link to CONTRIBUTING.md

## Files Modified
- CONTRIBUTING.md (restructured)
- README.md (added Contributing section)
- CODE_OF_CONDUCT.md (created)
- .github/PULL_REQUEST_TEMPLATE.md (created)
- .github/ISSUE_TEMPLATE/bug_report.md (created)
- .github/ISSUE_TEMPLATE/feature_request.md (created)
- .github/ISSUE_TEMPLATE/performance_regression.md (created)

## No WARN/FAIL
All acceptance criteria met.

## References
- Plan section: Release Engineering / Contributor Workflow, lines 3424-3433
- ADR-009 (Argo-only CI explains the fork caveat)
- Bead: pdftract-i9rk
