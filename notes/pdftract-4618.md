# pdftract-4618: Adopt Contributor Covenant v2.1

## Summary

Implemented CODE_OF_CONDUCT.md adoption per bead requirements.

## Changes Made

### 1. CODE_OF_CONDUCT.md
- Updated to official Contributor Covenant v2.1 text (fetched from contributor-covenant.org)
- Substituted enforcement contact: `community@jedarden.com`
- Removed "caste, color," from pledge (not in official v2.1)
- Restored blank line after enforcement email

### 2. Issue Template Links (.github/ISSUE_TEMPLATE/)
- **bug_report.yml**: Added link to CODE_OF_CONDUCT.md in CoC checkbox description
- **feature_request.yml**: Added link to CODE_OF_CONDUCT.md in CoC checkbox description  
- **config.yml**: Added new contact link for Code of Conduct

### 3. README.md
- Added "By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md)." to Contributing section

## Acceptance Criteria

### PASS
- [x] CODE_OF_CONDUCT.md exists in repository root
- [x] File adopts Contributor Covenant v2.1 verbatim
- [x] Enforcement contact substituted with community@jedarden.com
- [x] Linked from README.md Contributing section
- [x] Linked from .github/ISSUE_TEMPLATE/bug_report.yml
- [x] Linked from .github/ISSUE_TEMPLATE/feature_request.yml
- [x] Linked from .github/ISSUE_TEMPLATE/config.yml
- [x] CONTRIBUTING.md already linked at line 35 (no change needed)

### Verification
- All links point to valid paths (relative links in docs, absolute URLs in templates)
- Covenant text matches official v2.1 from contributor-covenant.org
- Four-tier enforcement ladder present (Correction, Warning, Temporary Ban, Permanent Ban)

## Commit
- Hash: `5699998`
- Message: `docs(pdftract-4618): adopt Contributor Covenant v2.1 and link from templates`

## Next Steps
- GitHub Community Standards health check should now pass (CODE_OF_CONDUCT.md + links verified)
