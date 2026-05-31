# pdftract-e9lz: Security Hardening Epic - Survey Results

## Overview
Survey completed 2026-05-31. This epic implements security controls TH-01 through TH-10, supply chain guards, secrets handling, and audit logging.

## Already Implemented

### TH-01: Decompression Bomb Mitigation ✅
**Status**: Already implemented in `crates/pdftract-core/src/parser/stream.rs`
- `DEFAULT_MAX_DECOMPRESS_BYTES` constant (512 MB default)
- `StreamBomb` diagnostic emission
- Bomb limit enforcement in all stream decoders (FlateDecode, LZWDecode, ASCII85Decode, etc.)
- Chunk-by-chunk limit checking during decode
- Tests exist in stream.rs module

### TH-06: Supply Chain CI Gates ✅
**Status**: Partially implemented
- **cargo audit**: Argo Workflow `.ci/argo-workflows/pdftract-nightly-supply-chain.yaml` exists
- **cargo deny**: Workflow exists but **cargo-deny.toml config file missing**
- **Cargo.lock**: Exists at root (`./Cargo.lock`) for binary crate pdftract-cli

### TH-07: CLI Password Leak Prevention ✅
**Status**: Already implemented in `crates/pdftract-cli/src/password.rs`
- `--password-stdin` flag reads one line from stdin
- `PDFTRACT_PASSWORD` env var support
- `--password VALUE` rejected unless `PDFTRACT_INSECURE_CLI_PASSWORD=1`
- Uses `secrecy::SecretString` wrapper
- Comprehensive unit tests

### TH-08: Log Audit ✅
**Status**: Already implemented
- **Audit logging**: `crates/pdftract-core/src/audit.rs` implements NDJSON audit log writer
- **Test**: `tests/security/TH-08-log-audit.rs` exists
- **Schema**: ts/client_ip/tool/fingerprint/duration_ms/status/diagnostics fields
- **Log policy**: `crates/pdftract-core/src/log_policy.rs` enforces no-secrets logging

### Secrets Handling Infrastructure ✅
**Status**: Already implemented
- **secrecy crate**: Used throughout for secret wrapping
- **Password handling**: `crates/pdftract-cli/src/password.rs`
- **MCP token handling**: `crates/pdftract-cli/src/mcp/auth.rs` with:
  - `--auth-token-file PATH` (recommended)
  - `PDFTRACT_MCP_TOKEN` env var
  - `--auth-token VALUE` rejected unless `PDFTRACT_INSECURE_CLI_TOKEN=1`
  - Uses `secrecy::SecretString`

### Audit Logging Subsystem ✅
**Status**: Already implemented
- **Writer**: `crates/pdftract-core/src/audit.rs`
- **Middleware**: `crates/pdftract-cli/src/middleware/audit.rs`
- **Integration**: Used in serve.rs, mcp modules

## Still Missing / Needs Verification

### TH-02: Path Traversal Prevention ❓
**Status**: Needs verification
- INV-10 requirement: MCP MUST NOT accept file-path parameters
- Need to verify MCP tool signatures don't include path parameters
- Test `TH-02-path-traversal.rs` doesn't exist yet

### TH-03: MCP Authentication Enforcement ❓
**Status**: Needs verification
- Requirement: `mcp --bind` MUST require `--auth-token` unless bind resolves to 127.0.0.1/::1
- Startup must abort with exit code 78 if unauthenticated public bind
- Test `TH-03-mcp-no-auth.rs` doesn't exist yet
- Need to verify implementation in `crates/pdftract-cli/src/mcp/` modules

### TH-04: JavaScript Presence Detection ❓
**Status**: Partially implemented
- **Catalog parsing**: `crates/pdftract-core/src/parser/catalog.rs` extracts `/OpenAction` and `/AA` entries
- **Missing**: JAVASCRIPT_PRESENT diagnostic emission
- **Missing**: `metadata.javascript_actions[]` in JSON output
- Test `TH-04-js-presence.rs` doesn't exist yet

### TH-05: SSRF Protection ❓
**Status**: Needs verification
- Requirement: URL schemes restricted to `https://`
- localhost/RFC1918/IPv6 ULA/link-local/loopback refused unless `--allow-private-networks`
- Refusal emits `URL_PRIVATE_NETWORK` diagnostic
- Need to verify ureq-based remote fetcher implementation
- Test `TH-05-ssrf-block.rs` doesn't exist yet

### TH-09: Inspector XSS Protection ❓
**Status**: Needs verification
- Requirement: Inspector never uses innerHTML/outerHTML with extraction output
- CSP header: `default-src 'self'; script-src 'self'`
- Test `TH-09-inspector-xss.rs` doesn't exist yet
- Fixture `xss-payload.pdf` exists in `tests/fixtures/security/`

### TH-10: Cache Integrity Verification ❌
**Status**: Not implemented
- Requirement: HMAC-SHA-256 over `fingerprint || extraction_options || output_blob`
- Per-cache random key created on cache init
- Reads verify HMAC; mismatch = miss with `CACHE_INTEGRITY_FAIL` diagnostic
- Test `TH-10-cache-poison.rs` doesn't exist yet

### Build Checksums ❌
**Status**: Not implemented
- **Missing**: `build/CHECKSUMS.sha256` file
- **Missing**: build.rs verification of font-fingerprints.json and glyph-shapes.json checksums
- Files exist: `build/font-fingerprints.json`, `build/glyph-shapes.json`

### cargo-deny Configuration ❌
**Status**: Not implemented
- **Missing**: `cargo-deny.toml` at root
- Need to configure:
  - License allowlist (MIT, Apache-2.0, BSD-2/3, ISC, Zlib, Unicode-DFS-2016, MPL-2.0)
  - Bans: openssl-sys, native-tls, git2, libgit2-sys
  - Minimum versions: ring >= 0.17.5, rustls >= 0.23

## Acceptance Criteria Status

| Criterion | Status |
|-----------|--------|
| All TH-01 through TH-10 tests exist and pass | ❌ 5 tests missing |
| secrecy crate wraps every secret type | ✅ |
| --password-stdin, --auth-token-file functional | ✅ |
| Profile loader rejects YAML with credentials | ❓ Needs verification |
| --audit-log FILE emits NDJSON | ✅ |
| TH-08 log audit test passes | ✅ |
| Cargo.lock checked in | ✅ |
| cargo audit + cargo deny green | ❌ cargo-deny.toml missing |
| build/CHECKSUMS.sha256 enforced | ❌ |

## Priority Implementation Order

1. **cargo-deny.toml** - TH-06 acceptance criterion
2. **build/CHECKSUMS.sha256** - Build integrity gate
3. **TH-03 MCP auth enforcement** - Critical security gate
4. **TH-04 JavaScript detection** - Malware detection
5. **TH-05 SSRF protection** - Network security
6. **TH-10 Cache integrity** - Cache poisoning defense
7. **TH-02 Path traversal test** - Verify design invariant
8. **TH-09 Inspector XSS test** - Verify CSP/no-innerHTML

## Files Referenced

- `crates/pdftract-core/src/parser/stream.rs` - Bomb protection
- `crates/pdftract-cli/src/password.rs` - Password ingress
- `crates/pdftract-cli/src/mcp/auth.rs` - Token ingress
- `crates/pdftract-core/src/audit.rs` - Audit log writer
- `crates/pdftract-core/src/log_policy.rs` - Log policy enforcement
- `.ci/argo-workflows/pdftract-nightly-supply-chain.yaml` - Supply chain scan
- `tests/security/TH-08-log-audit.rs` - Log audit test
- `tests/fixtures/security/` - Security test fixtures

## Next Steps

1. Create `cargo-deny.toml` with license/ban/advisory configs
2. Generate `build/CHECKSUMS.sha256` for font-fingerprints.json and glyph-shapes.json
3. Verify/complete TH-03 MCP authentication enforcement
4. Verify/complete TH-05 SSRF protection
5. Implement TH-04 JavaScript diagnostic emission
6. Implement TH-10 cache integrity verification
7. Create missing TH-NN test files
