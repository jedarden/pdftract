# Verification Note: pdftract-f29c (GitHub Issue and PR Templates)

## Summary

Created GitHub issue and PR templates as specified in the bead. All templates use GitHub Issue Forms (YAML format) with required field enforcement.

## Files Created/Modified

### Created
- `.github/ISSUE_TEMPLATE/config.yml` - Disables blank issues, provides contact links for Discussions and Security
- `.github/ISSUE_TEMPLATE/documentation.yml` - New template for documentation issues
- `.github/ISSUE_TEMPLATE/bug_report.yml` - Bug report with `pdftract doctor` as required field
- `.github/ISSUE_TEMPLATE/feature_request.yml` - Feature request with use case and proposed solution
- `.github/ISSUE_TEMPLATE/performance_regression.yml` - Performance regression with baseline/comparison fields
- `.github/ISSUE_TEMPLATE/security.yml` - Security advisory redirect to SECURITY.md

### Modified
- `.github/PULL_REQUEST_TEMPLATE.md` - Updated to include local validation checkbox

### Deleted
- `.github/ISSUE_TEMPLATE/bug_report.md` - Replaced with .yml Issue Form
- `.github/ISSUE_TEMPLATE/feature_request.md` - Replaced with .yml Issue Form
- `.github/ISSUE_TEMPLATE/performance_regression.md` - Replaced with .yml Issue Form
- `.github/ISSUE_TEMPLATE/security.md` - Replaced with .yml Issue Form

## Acceptance Criteria

- [x] All five issue templates and the PR template land at the documented paths
- [x] `config.yml` disables blank issues (`blank_issues_enabled: false`)
- [x] `config.yml` lists contact links (Discussions, Security advisories)
- [x] Bug template forces `pdftract doctor` output (`required: true` enforcement)
- [x] Security template redirects to SECURITY.md (contact link in config.yml + template reference)
- [x] PR template includes linked-issue + scope-statement + test-plan + checklist sections
- [x] PR template includes local validation checkbox per CONTRIBUTING.md
- [x] All templates use YAML Issue Forms syntax with `required: true` enforcement
- [x] All templates include Code of Conduct checkbox

## Notes

- Security vulnerabilities are primarily routed via `config.yml` contact link to prevent public filing
- The security.yml template exists as a reference for anyone who navigates to it directly
- Bug report template enforces `pdftract doctor` output as required field (UI blocks submission if missing)
- All templates auto-apply appropriate labels via `labels:` field
- Documentation template was missing and has been added

## Verification

Templates are valid YAML and follow GitHub Issue Forms specification. The GitHub UI will render these as interactive forms when users create new issues.

## Compilation Note

Pre-existing compilation errors in the codebase (examples/test_lzw_api.rs, tests/ocr_integration.rs, etc.) are unrelated to these template changes. Templates are static YAML files and do not affect compilation.
