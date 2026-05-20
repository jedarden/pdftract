# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest minor release | Yes |
| Previous minor release | Yes (security fixes only) |
| Older releases | No |

Security updates are published for the latest minor release and the previous minor release. Older releases do not receive security updates — users must upgrade to a supported version.

## Reporting a Vulnerability

**Do NOT open a public issue or pull request for security vulnerabilities.**

### Private Disclosure

Please report vulnerabilities privately using one of the following methods:

1. **Email (preferred):** [security@jedarden.com](mailto:security@jedarden.com)
   - PGP-encrypted emails are strongly encouraged
   - PGP key fingerprint: `PLACEHOLDER — see docs/security/pgp-public-key.asc for instructions`
   - Public key available at: [`docs/security/pgp-public-key.asc`](docs/security/pgp-public-key.asc)

2. **GitHub Private Vulnerability Reporting:**
   - Use the [Security tab](https://github.com/jedarden/pdftract/security/advisories) on this repository
   - This provides a private discussion forum coordinated with GitHub's security team

## Disclosure Window

Our disclosure timeline is designed to balance rapid fixes with thorough, safe deployment:

| Stage | Timeline |
|-------|----------|
| **Acknowledgment** | Within 48 hours of receipt |
| **Initial Triage** | Within 5 business days (confirmed/disputed/need-more-info) |
| **Fix & CVE Publication** | Within 90 days of confirmation |
| **Public Disclosure** | Simultaneous with fix release |

- **Extension policy:** The 90-day window may be extended by mutual agreement for complex issues requiring coordinated fixes across multiple releases or downstream ecosystems.
- **Embargoed coordination:** For major vulnerabilities, we coordinate disclosure with downstream packagers (Homebrew, Linux distributions) prior to public announcement.

## CVE Assignment

CVEs are assigned via [GitHub Security Advisories](https://github.com/jedarden/pdftract/security/advisories). GitHub is a CVE CNA (CVE Numbering Authority), which allows us to request CVEs directly without going through the MITRE backlog.

- Researchers are credited in the advisory unless they request anonymity.
- Vulnerability severity is assessed using the [CVSS v3.1](https://www.first.org/cvss/calculator/3.1) standard.

## Scope

### In Scope

The following classes of security issues are within scope for responsible disclosure:

- **Code execution** from crafted PDF documents (e.g., parser bugs enabling arbitrary code execution)
- **Path traversal** vulnerabilities (e.g., extracting embedded files that escape intended directories)
- **Server-Side Request Forgery (SSRF)** in the HTTP service mode
- **Authentication bypass** in the MCP integration or other authenticated interfaces
- **Signature verification bypass** in release artifact verification
- **Supply-chain attacks** affecting the build pipeline or dependency integrity

### Out of Scope

The following are explicitly out of scope (covered by existing mitigations or accepted risk):

- **Denial-of-Service via valid PDFs** — Large or complex PDFs that exhaust CPU/memory are resource management issues, not vulnerabilities. Use resource limits.
- **Missing security headers** on demo/deployment sites — These are deployment concerns, not code vulnerabilities.
- **Vulnerabilities in dependencies** that have already been disclosed and patched upstream — Upgrade to the fixed dependency version.
- **Information leakage via debug output** in development/debug builds — Only release builds are security-supported.

## Safe Harbor

We commit to working with security researchers who act in good faith. This policy is adapted from the [Disclose.io Safe Harbor](https://disclose.io/safe-harbor) template:

> If you act in good faith and accordance with this policy:
>
> - We will **not** pursue legal action against you
> - We will **not** notify law enforcement
> - We will **not** target your research for DMCA or CFAA claims
>
> We consider "good faith" to mean:
> - You give us reasonable time to address the vulnerability before public disclosure
> - You do not exploit the vulnerability beyond what is necessary to demonstrate it
> - You do not destroy data or degrade service for other users
> - You report the vulnerability through one of the private channels listed above
>
> If at any point you have concerns about whether your research qualifies as good faith, please contact us at [security@jedarden.com](mailto:security@jedarden.com) before proceeding.
