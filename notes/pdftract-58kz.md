# pdftract-58kz Verification Note

## Task
SECURITY.md — security@jedarden.com + 90-day disclosure window + private vuln reporting

## Work Completed

### Files Created/Modified

1. **SECURITY.md** (created)
   - All 6 required sections present and complete:
     - Supported Versions (latest/previous minor support policy)
     - Reporting a Vulnerability (email + GitHub private reporting)
     - Disclosure Window (48h ack, 5 business days triage, 90-day fix window)
     - CVE Assignment (via GitHub Security Advisories)
     - Scope (in-scope: RCE, path traversal, SSRF, auth bypass; out-of-scope: DoS, deployment headers, upstream vulns)
     - Safe Harbor (adapted from Disclose.io template)

2. **docs/security/pgp-public-key.asc** (created)
   - Placeholder with generation instructions
   - Specifies 4096-bit RSA key requirements
   - Documents 2-year rotation policy

3. **.github/ISSUE_TEMPLATE/security.md** (created)
   - Redirects users to private reporting channels
   - Links to SECURITY.md for full policy

4. **CONTRIBUTING.md** (modified)
   - Added Security section with responsible disclosure
   - Links to SECURITY.md for full disclosure policy

5. **README.md** (modified)
   - Added Security section with security@jedarden.com link
   - Added PGP key reference with placeholder note
   - Added Verifying Releases section (pre-existing, confirmed)

### Commit
- **Commit:** bb5346b `docs(pdftract-58kz): add security policy documentation`
- **Files:** 5 changed, 242 insertions(+)
- **Pushed:** https://git.ardenone.com/jedarden/pdftract.git

## Acceptance Criteria Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| SECURITY.md exists with all six sections | PASS | All sections complete |
| security@jedarden.com alias set up and monitored | WARN | Infrastructure task; requires email admin |
| PGP key published with fingerprint in README | WARN | Placeholder with instructions; key generation requires security@jedarden.com to exist |
| GitHub Community Standards check green | WARN | Cannot verify from CLI; requires GitHub UI |
| Test report acknowledged within 48h | WARN | Infrastructure task; requires security@jedarden.com to be operational |
| Linked from README, CONTRIBUTING.md, issue template | PASS | All three link to SECURITY.md |

## WARN Items Justification

The WARN items are infrastructure-dependent and outside the scope of code/documentation changes:

1. **security@jedarden.com email alias**: Requires email infrastructure setup and forwarding configuration. The documentation references this alias and provides the policy for when it's operational.

2. **PGP key generation**: Requires the security@jedarden.com email to exist before generating a key tied to that address. The placeholder includes complete generation instructions.

3. **GitHub Community Standards check**: Requires manual verification in GitHub repository settings (not accessible via CLI).

4. **48-hour acknowledgement test**: Requires the email alias to be operational to send a test report to.

## References

- Plan section: Release Engineering / Contributor Workflow, line 3433
- OpenSSF Scorecard `vulnerabilities` check
- Disclose.io safe-harbor template
- GitHub Security Advisories docs
