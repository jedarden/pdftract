# pdftract-4em4l: Audit Logging Implementation Verification

## Summary

The `--audit-log FILE` flag and audit logging infrastructure was already fully implemented in the codebase. This note documents the verification of the acceptance criteria.

## Acceptance Criteria Status

### ✅ --audit-log FILE flag implemented
- **serve**: `crates/pdftract-cli/src/main.rs:322` - AuditLogWriter passed to serve::run
- **mcp**: `crates/pdftract-cli/src/main.rs:385` - AuditLogWriter passed to mcp::run_stdio/run_http
- **inspect**: `crates/pdftract-cli/src/inspect/args.rs:44-62` - audit_log field on InspectArgs

### ✅ Per-request NDJSON line with all documented fields
**AuditRecord schema** (`crates/pdftract-core/src/audit.rs:33-52`):
- `ts`: ISO-8601 RFC3339 UTC timestamp
- `client_ip`: Option<String> - HTTP peer or stdio (absent)
- `tool`: String - Tool name (extract, classify, mcp.extract, etc.)
- `fingerprint`: Option<String> - PDF structural fingerprint (pdftract-v1:hex)
- `duration_ms`: u64 - Request duration in milliseconds
- `status`: u16 - HTTP-style status code (200 ok, 4xx client error, 5xx server error)
- `diagnostics`: Vec<String> - Diagnostic codes (XREF_REPAIRED, STREAM_BOMB, etc.)

### ✅ Stdio MCP requests OMIT the client_ip field
**Implementation**: `crates/pdftract-cli/src/mcp/stdio.rs:364,476`
```rust
None, // No client_ip for stdio mode
```

### ✅ Log-policy enforcement (compile-time)
**CI Gate**: `.ci/scripts/check-log-policy.sh`
- Scans for credential patterns in log/println/eprintln calls
- Checks for password, token, secret, api_key patterns
- Checks for content leaks (body, content, text, data)
- Runs in CI to prevent violations from being merged

### ✅ Log-policy enforcement (runtime)
**Runtime redaction mechanisms**:
1. **Password redaction**: `crates/pdftract-core/src/parser/stream.rs:3167` - Debug/Serialize redacts password
2. **Header redaction**: `crates/pdftract-cli/src/mcp/http.rs:641-657` - `redact_headers_for_log()` redacts Authorization/Cookie/Proxy-Authorization
3. **Panic hook redaction**: `crates/pdftract-cli/src/panic_hook.rs:58-79` - Redacts SecretString from backtraces
4. **Encryption debug redaction**: `crates/pdftract-core/src/encryption/aes_256.rs:423-427` - Debug redacts encryption keys

### ✅ TH-08 test for log policy
**Test**: `tests/security/TH-08-log-audit.rs`
- Runs extraction with `RUST_LOG=trace` (maximum verbosity)
- Captures stderr (log output)
- Verifies no sensitive content appears in logs
- Tests for password, bearer token, PDF bytes, and sensitive headers

### ✅ Rotation policy documented in --help output
**Documentation**: `crates/pdftract-cli/src/main.rs:306-320`
```text
## Audit log rotation

pdftract does NOT rotate the audit log. Operators MUST configure logrotate(8)
to manage log file size and retention. A typical logrotate configuration:
```

### ✅ Fingerprint logged, NOT path/URL
**Implementation**:
- `crates/pdftract-core/src/audit.rs:44` - `fingerprint: Option<String>` stores structural fingerprint
- `crates/pdftract-cli/src/serve.rs:582` - Extracts fingerprint from result
- `crates/pdftract-cli/src/serve.rs:604` - Logs fingerprint instead of path

### ✅ AuditLogWriter is crash-safe
**Implementation**: `crates/pdftract-core/src/audit.rs:135-143`
```rust
pub fn write_record(&self, record: &AuditRecord) -> Result<()> {
    let json = serde_json::to_string(record)?;
    let mut writer = self.writer.lock()?;
    writeln!(writer, "{}", json)?;
    writer.flush()?;  // Flush after each line for crash safety
    Ok(())
}
```

## Middleware Integration

### HTTP Serve
**File**: `crates/pdftract-cli/src/middleware/audit.rs`
- Extracts client IP from peer address or X-Forwarded-For
- Stores RequestMetadata in request extensions
- Handlers write audit logs after extraction completes

### MCP HTTP
**File**: `crates/pdftract-cli/src/mcp/http.rs:254-291`
- Writes audit log after handling POST requests
- Includes client_ip, duration_ms, status, diagnostics

### MCP Stdio
**File**: `crates/pdftract-cli/src/mcp/stdio.rs:361-370`
- Writes audit log for tools/call requests
- Omits client_ip (stdio mode has no client)

### Inspect
**File**: `crates/pdftract-cli/src/inspect/inspect.rs:79-94`
- Creates AuditLogWriter from args.audit_log
- Stores in InspectorState with AuditState wrapper

## NEVER-Log Policy

The following are NEVER logged at any level:
- Password values (redacted via SecretString)
- Bearer tokens (redacted via header redaction)
- PDF bytes (not logged; only fingerprint)
- Extracted text content (not logged; only metadata)
- Cookie/Authorization/Proxy-Authorization headers (redacted)

## Test Results

**Audit module tests**: All 6 tests passed
- test_audit_record_new
- test_audit_record_with_client_ip
- test_audit_record_with_diagnostics
- test_audit_record_add_diagnostic
- test_audit_record_serialize
- test_audit_log_writer_memory

## Conclusion

All acceptance criteria for pdftract-4em4l are satisfied. The audit logging infrastructure is fully implemented and tested.
