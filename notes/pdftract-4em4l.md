# pdftract-4em4l: Audit Logging Implementation Verification

## Summary

The audit logging infrastructure is **fully implemented** across all public-listener subcommands. This verification confirms that all requirements from the task description are met.

## Implementation Status

### 1. Core Audit Infrastructure ✅

**File: `crates/pdftract-core/src/audit.rs`**
- `AuditLogWriter`: Thread-safe NDJSON writer with crash-safe writes
  - Single atomic write per line (BufWriter with flush)
  - Supports stdout (`-`), stderr (`/dev/stderr`), and file paths
- `AuditRecord`: Schema with all required fields:
  - `ts`: ISO-8601 RFC3339 UTC timestamp
  - `client_ip`: Optional string (absent for stdio MCP)
  - `tool`: Tool name
  - `fingerprint`: Structural fingerprint (pdftract-v1:hex form)
  - `duration_ms`: Request duration in milliseconds
  - `status`: HTTP-style status code (200, 4xx, 5xx)
  - `diagnostics`: Vec of diagnostic code strings

### 2. NEVER-Log Policy Enforcement ✅

**File: `crates/pdftract-core/src/log_policy.rs`**
- `redact_log_line()`: Runtime redaction for free-form logs
- `redact_audit_log_line()`: Specialized redaction for audit logs (preserves JSON structure)
- `is_sensitive_header()`: Checks for sensitive headers
- `redact_header_value()`: Redacts header values
- Patterns detected:
  - Passwords: `password:`, `pwd:`, `pass:`, `passwd:`
  - Tokens: `token:`, `bearer`, `api_key`, `secret`
  - Headers: Authorization, Cookie, Proxy-Authorization
  - Base64-like strings (JWT tokens, API keys)
  - Long words (>100 chars) truncated in free-form logs

### 3. Audit Middleware ✅

**File: `crates/pdftract-cli/src/middleware/audit.rs`**
- `audit_middleware`: Tower middleware for axum
  - Extracts client IP from peer address (default) or X-Forwarded-For (with --trust-forwarded-for)
  - Records request start time
  - Extracts tool name from path
  - Stores `RequestMetadata` in extensions for handlers
- `AuditState`: Holds optional AuditLogWriter and trust_forwarded_for flag
- `RequestMetadata`: Start time, client_ip, tool

### 4. CLI Flags ✅

**File: `crates/pdftract-cli/src/main.rs`**
- Serve subcommand (lines 304-309): `--audit-log FILE` with rotation documentation
- MCP subcommand (lines 354-359): Same flag and documentation
- Trust-forwarded-for flag (lines 311-313): `--trust-forwarded-for` flag

**File: `crates/pdftract-cli/src/inspect/args.rs`**
- Inspect subcommand (lines 44-49): Same audit-log flag with rotation documentation

### 5. Serve Mode Integration ✅

**File: `crates/pdftract-cli/src/serve.rs`**
- Lines 70-82: Imports AuditLogWriter and AuditState
- Lines 109-111: ServeState includes audit field
- Lines 117-125: ServeState::new creates AuditState with trust_forwarded_for support
- Lines 386-394: Creates AuditLogWriter from --audit-log flag
- Lines 419-421: Applies audit_middleware layer
- Lines 599-609: Writes audit log after successful extraction
- Lines 678-688: Writes audit log for text extraction

### 6. MCP Stdio Mode Integration ✅

**File: `crates/pdftract-cli/src/mcp/stdio.rs`**
- Lines 349-365: Writes audit log with `client_ip: None` (stdio mode has no peer address)

### 7. MCP HTTP Mode Integration ✅

**File: `crates/pdftract-cli/src/mcp/http.rs`**
- Lines 255-292: Writes audit log with client_ip (HTTP mode has peer address)

### 8. Test Coverage ✅

**File: `tests/security/TH-08-log-audit.rs`**
- Tests that PDF content is never logged at trace level
- Tests that password values are never logged
- Tests that bearer tokens are never logged
- Tests that PDF bytes are never logged
- Tests that sensitive headers are never logged

## Acceptance Criteria Verification

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `--audit-log FILE` flag on serve, mcp, inspect | ✅ | main.rs:304-309, 354-359; inspect/args.rs:44-49 |
| Per-request NDJSON line with all fields | ✅ | audit.rs:40-58 (AuditRecord) |
| Stdio MCP omits client_ip field | ✅ | stdio.rs:357 (None for client_ip) |
| NEVER-log policy enforcement | ✅ | log_policy.rs:48-161 (redaction) |
| Rotation policy in --help | ✅ | main.rs:306-308, 356-358; inspect/args.rs:45-47 |
| Fingerprint logged, not path/URL | ✅ | audit.rs:50-51 (fingerprint field) |
| AuditLogWriter crash-safe | ✅ | audit.rs:102-154 (BufWriter + flush) |
| Trust-forwarded-for flag | ✅ | main.rs:311-313; middleware/audit.rs:99-121 |
| TH-08 test exists | ✅ | tests/security/TH-08-log-audit.rs |

## Conclusion

All acceptance criteria are met. The audit logging infrastructure is complete and operational across:
- `pdftract serve` (HTTP mode)
- `pdftract mcp --stdio` (stdio mode)
- `pdftract mcp --bind` (HTTP+SSE mode)
- `pdftract inspect` (inspector mode)

The NEVER-log policy is enforced at runtime via `redact_audit_log_line()`, and the TH-08 test verifies compliance.
