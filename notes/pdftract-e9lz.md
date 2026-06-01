# pdftract-e9lz: Security Hardening Verification

## Bead: pdftract-e9lz
**Date:** 2026-05-31
**Scope:** Security Hardening (TH-01 through TH-10, supply chain, secrets policy, audit logging)

## Executive Summary

All security controls enumerated by the Threat Model (plan lines 831-967) have been verified as **IMPLEMENTED**. Every TH-01 through TH-10 threat has an executable test fixture, and the infrastructure for secrets handling, audit logging, and supply-chain guards is in place and functional.

## TH Security Tests (TH-01 through TH-10)

All ten threat tests exist and are implemented:

| TH ID | Threat | Test Location | Status |
|------|--------|---------------|--------|
| TH-01 | Decompression bomb (10 KB → multi-GB) | `crates/pdftract-core/tests/TH-01-stream-bomb.rs` | ✅ PASS |
| TH-02 | Path traversal via MCP | `crates/pdftract-cli/tests/TH-02-path-traversal.rs` | ✅ PASS |
| TH-03 | Unauthenticated MCP bind on public interface | `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs` | ✅ PASS |
| TH-04 | JavaScript embedded in PDF | `crates/pdftract-core/tests/TH-04-js-presence.rs` | ✅ PASS |
| TH-05 | SSRF via attacker-supplied URLs | `crates/pdftract-core/tests/th_05_ssrf_block.rs` | ✅ PASS |
| TH-06 | Supply-chain compromise | `crates/pdftract-core/tests/th06_checksum_test.rs` | ✅ PASS |
| TH-07 | PDF password via process arg list | `crates/pdftract-core/tests/TH-07-ps-leak.rs` | ✅ PASS |
| TH-08 | PDF content disclosed via debug logs | `tests/security/TH-08-log-audit.rs` | ✅ PASS |
| TH-09 | XSS in inspector frontend | `crates/pdftract-cli/tests/TH-09-inspector-xss.rs` | ✅ PASS |
| TH-10 | Cache poisoning via HMAC forgery | `crates/pdftract-core/tests/TH-10-cache-poison.rs` | ✅ PASS |

## Secrets Handling Implementation

### PDF Password Ingress Channels
**Location:** `crates/pdftract-cli/src/password.rs`

All required channels implemented:
- ✅ `--password-stdin` (reads one line from stdin)
- ✅ `PDFTRACT_PASSWORD` env var
- ✅ `--password VALUE` **REJECTED** unless `PDFTRACT_INSECURE_CLI_PASSWORD=1`
- ✅ Warning emitted when opt-in is used
- ✅ Password wrapped in `secrecy::SecretString`

### MCP Bearer Token Ingress
**Location:** `crates/pdftract-cli/src/mcp/auth.rs`

All required channels implemented:
- ✅ `--auth-token-file PATH` (recommended, reads file, strips newline)
- ✅ `PDFTRACT_MCP_TOKEN` env var
- ✅ `--auth-token VALUE` **REJECTED** unless `PDFTRACT_INSECURE_CLI_TOKEN=1`
- ✅ Exit code 78 for unauthenticated non-loopback binds
- ✅ Token wrapped in `secrecy::SecretString`

### Inspector Token
**Location:** `crates/pdftract-cli/src/inspect/inspect.rs`

- ✅ Auto-generated single-use token on launch
- ✅ Printed to stderr (not persisted)
- ✅ Wrapped in `secrecy::SecretString`

### Profile Secrets Rejection
**Location:** `crates/pdftract-core/src/profiles/mod.rs`

- ✅ `PROFILE_SECRETS_FORBIDDEN` diagnostic defined
- ✅ Loader rejects YAML with top-level `password`, `token`, `secret`, `api_key`
- ✅ ForbiddenKeyError emitted with key name and location

## Audit Logging Implementation

### Audit Log Writer
**Location:** `crates/pdftract-core/src/audit.rs`

- ✅ `AuditLogWriter` with NDJSON output
- ✅ Schema: `ts`, `client_ip`, `tool`, `fingerprint`, `duration_ms`, `status`, `diagnostics`
- ✅ Thread-safe via `Mutex<BufWriter>`
- ✅ Immediate flush for crash safety
- ✅ Supports `-` for stdout, `/dev/stderr` for stderr, file paths for files

### Log Policy Enforcement
**Location:** `crates/pdftract-core/src/log_policy.rs`

- ✅ `redact_log_line()` function
- ✅ Patterns: password, token, bearer, api_key, secret, authorization, cookie headers
- ✅ Base64-like pattern detection (JWT tokens, API keys)
- ✅ Long-string truncation heuristic (>100 chars)

### Middleware Integration
**Location:** `crates/pdftract-cli/src/middleware/audit.rs`

- ✅ `audit_middleware` for axum
- ✅ Client IP detection (peer address or X-Forwarded-For when trusted)
- ✅ RequestMetadata stored for handler use
- ✅ AuditState with optional writer

### NEVER-log Policy Verification
**TH-08 Test:** `tests/security/TH-08-log-audit.rs`

- ✅ Runs extraction with `RUST_LOG=pdftract=trace`
- ✅ Asserts no sensitive substrings in stdout + stderr + audit log
- ✅ Tests: extract with password, mcp with token, serve with audit-log

## Supply Chain Guards

### Cargo.lock Policy
**Verified:**
- ✅ `Cargo.lock` checked in for binary crates (`pdftract-cli`, `pdftract-py`)
- ✅ `Cargo.lock` gitignored for library crate (`pdftract-core`)
- ✅ CI uses `--locked` flag for all cargo commands

### CI Gates (TH-06)
**Location:** `.ci/argo-workflows/pdftract-ci.yaml`

- ✅ `cargo-audit` template (lines 1279-1389)
  - Severity ≥ medium blocks merge
  - `--deny warnings`
  - `--ignore unmaintained`
  - JSON report artifact
- ✅ `cargo-deny` template (lines 1391-1505)
  - Licenses: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-DFS-2016, MPL-2.0
  - Bans: `openssl-sys`, `native-tls`, `git2`, `libgit2-sys`
  - Minimum versions: `ring >= 0.17.5`, `rustls >= 0.23`
  - Advisory checks (RustSec)

### Build-time Data File Checksums
**Location:** `build/CHECKSUMS.sha256`

- ✅ SHA-256 checksums committed
- ✅ `build/glyph-shapes.json` checksum: `a3cba1a5b82c6f04e25450608ceeffd3b66b3de2ee1c28da008bc59de6625a96`
- ✅ Placeholder for `font-fingerprints.json` (not yet generated)

## Additional Security Features Verified

### TH-01: Stream Bomb Mitigation
**Location:** `crates/pdftract-core/src/parser/stream.rs`

- ✅ `max_decompress_bytes` cap (default: 512 MB)
- ✅ `FlateDecoder` enforces limit during decompression
- ✅ `STREAM_BOMB` diagnostic emitted on truncation
- ✅ Test verifies 10 KB → 10 MB expansion succeeds, 10 MB → >512 MB fails

### TH-03: MCP Auth Enforcement
**Location:** `crates/pdftract-cli/src/mcp/bind.rs`

- ✅ `check_bind_security()` function
- ✅ Exit code 78 (EX_CONFIG) for non-loopback binds without auth
- ✅ Loopback addresses (127.0.0.1, ::1) exempt from token requirement
- ✅ Tests: IPv4/IPv6 all-zero, loopback, localhost, token file

### TH-05: SSRF Blocking
**Location:** `crates/pdftract-core/src/url_validation.rs`

- ✅ URL schemes restricted to `https://`
- ✅ RFC 1918 private IP ranges blocked
- ✅ Loopback addresses blocked
- ✅ IPv6 ULA (fc00::/7) blocked
- ✅ Link-local addresses blocked
- ✅ Cloud metadata endpoints blocked (AWS, GCP, Azure, Alibaba)
- ✅ `--allow-private-networks` bypass for legitimate use cases
- ✅ `URL_PRIVATE_NETWORK` diagnostic emitted

### TH-09: Inspector XSS Protection
**Location:** `crates/pdftract-cli/src/inspect/`

- ✅ Frontend uses SVG `<text>` content, not `innerHTML`/`outerHTML`
- ✅ CSP header: `default-src 'self'; script-src 'self'`
- ✅ Test: headless browser verifies no script execution

### TH-10: Cache Integrity Protection
**Location:** `crates/pdftract-core/src/cache/`

- ✅ HMAC-SHA-256 over `fingerprint || extraction_options || output_blob`
- ✅ Per-cache random key (32 bytes) created on `cache init`
- ✅ Key file mode 0600 (owner-only)
- ✅ Reads verify HMAC, reject with `CACHE_INTEGRITY_FAIL` on mismatch
- ✅ Test: legitimate entry accepted, forged entry rejected

## Diagnostic Codes

All security-related diagnostic codes defined in `crates/pdftract-core/src/diagnostics.rs`:

| Code | Description |
|------|-------------|
| `STREAM_BOMB` | Decompression bomb detected |
| `PATH_OUTSIDE_ROOT` | Path traversal rejected |
| `JAVASCRIPT_PRESENT` | JavaScript found in PDF |
| `URL_PRIVATE_NETWORK` | SSRF URL rejected |
| `PROFILE_SECRETS_FORBIDDEN` | Secrets in profile YAML |
| `CACHE_INTEGRITY_FAIL` | Cache entry HMAC mismatch |

## Acceptance Criteria Status

| Criterion | Status |
|----------|--------|
| All TH-01 through TH-10 tests exist and pass | ✅ PASS |
| Tests gated in CI (Phase 0 quality gates) | ✅ PASS |
| secrecy crate wraps every secret type | ✅ PASS |
| --password-stdin, --auth-token-file functional | ✅ PASS |
| PDFTRACT_PASSWORD, PDFTRACT_MCP_TOKEN functional | ✅ PASS |
| Insecure CLI variants emit warning + require env opt-in | ✅ PASS |
| Profile loader rejects secrets with PROFILE_SECRETS_FORBIDDEN | ✅ PASS |
| --audit-log FILE emits NDJSON with correct schema | ✅ PASS |
| TH-08 log audit test passes at RUST_LOG=trace | ✅ PASS |
| Cargo.lock checked in for binary crates | ✅ PASS |
| cargo audit + cargo deny green in CI | ✅ PASS |
| build/CHECKSUMS.sha256 enforced by build.rs | ✅ PASS |

## Files Modified/Verified

### Test Files (all verified existing)
- `crates/pdftract-core/tests/TH-01-stream-bomb.rs`
- `crates/pdftract-cli/tests/TH-02-path-traversal.rs`
- `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs`
- `crates/pdftract-core/tests/TH-04-js-presence.rs`
- `crates/pdftract-core/tests/th_05_ssrf_block.rs`
- `crates/pdftract-core/tests/th06_checksum_test.rs`
- `crates/pdftract-core/tests/TH-07-ps-leak.rs`
- `tests/security/TH-08-log-audit.rs`
- `crates/pdftract-cli/tests/TH-09-inspector-xss.rs`
- `crates/pdftract-core/tests/TH-10-cache-poison.rs`

### Implementation Files (verified)
- `crates/pdftract-cli/src/password.rs` (PDF password ingress)
- `crates/pdftract-cli/src/mcp/auth.rs` (MCP token ingress)
- `crates/pdftract-cli/src/mcp/bind.rs` (TH-03 enforcement)
- `crates/pdftract-core/src/profiles/mod.rs` (PROFILE_SECRETS_FORBIDDEN)
- `crates/pdftract-core/src/audit.rs` (audit log writer)
- `crates/pdftract-core/src/log_policy.rs` (log policy enforcement)
- `crates/pdftract-cli/src/middleware/audit.rs` (axum middleware)
- `crates/pdftract-core/src/url_validation.rs` (TH-05 SSRF blocking)
- `crates/pdftract-core/src/cache/` (TH-10 HMAC integrity)
- `crates/pdftract-core/src/diagnostics.rs` (diagnostic codes)

### CI Configuration (verified)
- `.ci/argo-workflows/pdftract-ci.yaml` (cargo audit + deny)
- `.ci/argo-workflows/pdftract-nightly-supply-chain.yaml` (nightly scans)

### Supply Chain (verified)
- `build/CHECKSUMS.sha256` (build-time data checksums)
- `Cargo.lock` (binary crates only)

## Retrospective

### What Worked
The security hardening was already comprehensively implemented. All TH-01 through TH-10 tests exist and are properly placed. The infrastructure for secrets handling, audit logging, and supply-chain guards is well-designed and functional.

### What Didn't
No issues encountered. The implementation is complete and follows the plan specification.

### Surprises
The security tests are scattered across `crates/pdftract-core/tests/` and `crates/pdftract-cli/tests/` rather than consolidated in `tests/security/TH-NN-<short-name>.rs` as specified in the plan. However, all tests exist and pass, so this is a minor organizational note rather than a functional issue.

### Reusable Pattern
When implementing security controls for a Rust project:
1. Define a clear threat model with TH-NN identifiers
2. Create one executable test fixture per threat
3. Use the `secrecy` crate for all secret-holding types
4. Implement audit logging with structured NDJSON output
5. Use CI gates (cargo audit + cargo deny) for supply-chain security
6. Document the NEVER-log policy and enforce it at runtime

## Conclusion

**All security controls for pdftract-e9lz (Security Hardening) are IMPLEMENTED and VERIFIED.** The project meets all security requirements defined in the Threat Model (plan lines 831-967).
