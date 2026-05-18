# pdftract-59zz: MCP Bearer Token Ingress Channels and TH-03 Enforcement

## Summary

Implemented MCP bearer-token ingress channels and TH-03 startup abort enforcement. The implementation was already present in the codebase (`crates/pdftract-cli/src/mcp/`) and verified to be working correctly.

## Verification

### PASS: --auth-token-file PATH (RECOMMENDED)
```bash
$ echo "file-token-32-bytes-long-security" > /tmp/token.txt
$ timeout 0.1 ./target/debug/pdftract mcp --bind 127.0.0.1:9999 --auth-token-file /tmp/token.txt
Bearer token provided via secure channel
Bind address: 127.0.0.1:9999
Starting MCP server on 127.0.0.1:9999...
```

### PASS: PDFTRACT_MCP_TOKEN env var
```bash
$ PDFTRACT_MCP_TOKEN="env-token-32-bytes-long-security" timeout 0.1 ./target/debug/pdftract mcp --bind 127.0.0.1:9999
Bearer token provided via secure channel
Bind address: 127.0.0.1:9999
Starting MCP server on 127.0.0.1:9999...
```

### PASS: --auth-token VALUE rejected (exit 64) unless PDFTRACT_INSECURE_CLI_TOKEN=1
```bash
$ ./target/debug/pdftract mcp --bind 127.0.0.1:8080 --auth-token "test-token"
Error: The --auth-token VALUE flag is REJECTED for security reasons.
...
Exit code: 64
```

With insecure flag:
```bash
$ PDFTRACT_INSECURE_CLI_TOKEN=1 timeout 0.1 ./target/debug/pdftract mcp --bind 127.0.0.1:9999 --auth-token "test-token"
WARNING: Using --auth-token VALUE is INSECURE. The token is visible in process listings.
...
Bearer token provided via secure channel
```

### PASS: TH-03 - mcp --bind ADDR with non-loopback ADDR and no token: aborts with exit 78
```bash
$ ./target/debug/pdftract mcp --bind 0.0.0.0:9999
Error: ERROR: pdftract mcp --bind 0.0.0.0:9999 requires --auth-token-file PATH or PDFTRACT_MCP_TOKEN env (loopback addresses 127.0.0.1 / ::1 exempt). Refusing to bind to 0.0.0.0:9999 without authentication.
Exit code: 78
```

### PASS: TH-03 - mcp --bind ADDR with loopback ADDR and no token: succeeds
```bash
$ timeout 0.1 ./target/debug/pdftract mcp --bind 127.0.0.1:9999
No bearer token (loopback-only mode)
Bind address: 127.0.0.1:9999
Starting MCP server on 127.0.0.1:9999...
```

### PASS: TH-03 - IPv6 loopback exemption
```bash
$ timeout 0.1 ./target/debug/pdftract mcp --bind "[::1]:9999"
No bearer token (loopback-only mode)
Bind address: [::1]:9999
Starting MCP server on [::1]:9999...
```

### PASS: mcp --bind ADDR with token: succeeds regardless of address
```bash
$ PDFTRACT_MCP_TOKEN="test-token-32-bytes-long-security" timeout 0.1 ./target/debug/pdftract mcp --bind 0.0.0.0:9999
Bearer token provided via secure channel
Bind address: 0.0.0.0:9999
Starting MCP server on 0.0.0.0:9999...
```

### PASS: Token length warning
Tokens shorter than 32 bytes emit a warning:
```
WARNING: Token length is 10 bytes, which is below the recommended minimum of 32 bytes. Consider using a longer token for better security.
```

## Files Modified

- `crates/pdftract-cli/Cargo.toml` - Added `walkdir = "2"` dependency (was missing)
- `crates/pdftract-cli/src/mcp/auth.rs` - Fixed `mut` warnings (unnecessary mut on temp_file)
- `crates/pdftract-cli/src/mcp/server.rs` - Fixed unused `Context` import

## Files Reviewed (Already Implemented)

- `crates/pdftract-cli/src/mcp/auth.rs` - `resolve_token()` function with priority order
- `crates/pdftract-cli/src/mcp/bind.rs` - `check_bind_security()` function with TH-03 enforcement
- `crates/pdftract-cli/src/mcp/server.rs` - `run()` function using both auth and bind checks
- `crates/pdftract-cli/src/main.rs` - CLI arguments for `--auth-token-file` and `--auth-token`
- `crates/pdftract-cli/src/mcp/mod.rs` - Module exports

## WARN Items

- The TH-03 test (`tests/security/TH-03-mcp-no-auth.rs`) is a separate bead as noted in the task description
- Inspector token implementation (Phase 7.9) is a separate parallel implementation

## References

- Plan lines 874 (TH-03 mitigation)
- Plan lines 915-921 (Secrets Handling: MCP bearer token)
- Plan lines 922-924 (Inspector token same channels)
