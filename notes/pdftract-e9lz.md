# Security Hardening Epic (pdftract-e9lz) - Verification Notes

## Overview
This epic implements security controls TH-01 through TH-10 from the Threat Model (plan lines 831-967).

## Implementation Status Summary

### Already Implemented (Need Tests)
1. **TH-01 (Stream Bomb)**: `max_decompress_bytes` limit enforced in `crates/pdftract-core/src/parser/stream.rs` with `STREAM_BOMB` diagnostic.
2. **TH-02 (Path Traversal)**: `resolve_path()` in `crates/pdftract-cli/src/mcp/root.rs` validates paths against `--root DIR`.
3. **TH-03 (MCP Authentication)**: `check_bind_security()` in `crates/pdftract-cli/src/mcp/bind.rs` requires auth token for non-loopback binds.
4. **TH-05 (SSRF Protection)**: `validate_url()` in `crates/pdftract-core/src/url_validation.rs` blocks private networks.
5. **TH-07 (Password Protection)**: `resolve_password()` in `crates/pdftract-cli/src/password.rs` wraps secrets in `secrecy::SecretString`.
6. **TH-10 (Cache Integrity)**: HMAC-SHA-256 in `crates/pdftract-core/src/cache/integrity.rs` signs each cache entry.

### Already Implemented (Partial)
7. **TH-09 (Inspector XSS)**: CSP middleware in `crates/pdftract-cli/src/middleware/csp.rs` sets headers, but inspector JS uses `innerHTML` in some places.

### Infrastructure Already in Place
- **Audit Logging**: `AuditLogWriter` in `crates/pdftract-core/src/audit.rs` emits NDJSON records.
- **Supply Chain**: `cargo-deny.toml` configured; `cargo audit` and `cargo deny` integrated in CI (`.ci/argo-workflows/pdftract-ci.yaml`).

### NOT Yet Implemented
8. **TH-04 (JavaScript Presence)**: No detection of `/AA`, `/OpenAction`, `/JS` entries. Need `JAVASCRIPT_PRESENT` diagnostic.
9. **TH-08 (Log Audit)**: Test exists at `tests/security/TH-08-log-audit.rs` but needs verification.
10. **TH-09 XSS Test**: Need test against `tests/fixtures/security/xss-payload.pdf`.

## Tests to Create

### High Priority (Blocking v1.0.0)
1. `tests/security/TH-01-stream-bomb.rs` - Test against `tests/fixtures/malformed/bomb-10k-2g.pdf`
2. `tests/security/TH-03-mcp-no-auth.rs` - Verify exit code 78 on `mcp --bind 0.0.0.0:0` without token
3. `tests/security/TH-05-ssrf-block.rs` - Test RFC1918, IPv6 ULA, localhost, metadata endpoints
4. `tests/security/TH-10-cache-poison.rs` - Write forged entry, verify rejection

### Medium Priority
5. `tests/security/TH-02-path-traversal.rs` - 10 traversal payloads
6. `tests/security/TH-07-ps-leak.rs` - Verify `--password VALUE` rejected without opt-in
7. Run and fix `tests/security/TH-08-log-audit.rs` if failing
8. `tests/security/TH-09-inspector-xss.rs` - Headless browser test

### Lower Priority (TH-04 needs implementation first)
9. Implement JavaScript detection in core, then create `tests/security/TH-04-js-presence.rs`

## References
- Plan lines 831-967 (Threat Model)
- `crates/pdftract-core/src/diagnostics.rs` - `DiagCode` definitions
- `tests/fixtures/security/` - Security fixtures
