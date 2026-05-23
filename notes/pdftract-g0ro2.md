# Verification Note: pdftract-g0ro2 (HTTP+SSE transport)

## Summary

Implemented the HTTP+SSE transport for the MCP server per bead pdftract-g0ro2. All acceptance criteria PASS.

## Files Modified

- `crates/pdftract-cli/src/mcp/http.rs` - HTTP+SSE server implementation (538 lines)
- `crates/pdftract-cli/tests/mcp-http.rs` - Integration tests (471 lines)
- `crates/pdftract-cli/src/mcp/mod.rs` - Module exports
- `crates/pdftract-cli/src/mcp/server.rs` - Server entry point
- `crates/pdftract-cli/Cargo.toml` - Dependencies (all already present)
- `crates/pdftract-cli/src/main.rs` - CLI wiring for `pdftract mcp --bind ADDR`

## Implementation Details

### Routes Implemented
- **POST /**: JSON-RPC requests (single or batch)
- **GET /sse**: Server-Sent Events for notifications
- **GET /health**: Health check (auth-exempt)

### Key Features
- Reuses axum/tokio/tower-http from Phase 6.4 (no new deps)
- Bearer token auth (from sibling bead 6.7.7)
- Request body limit (256 MB default, configurable via --max-upload-mb)
- SSE keepalive every 30 seconds
- Broadcast channel for fan-out notifications
- Backpressure handling (drops lagged clients with WARN log)
- 100-client SSE limit (MAX_SSE_CLIENTS)
- Custom 413 Payload Too Large JSON response
- Batch request support per JSON-RPC 2.0 spec

## Acceptance Criteria Results

| Criterion | Status | Test |
|-----------|--------|------|
| POST / tools/list returns tool catalog | PASS | test_post_tools_list |
| GET /sse opens stream with keepalive | PASS | test_get_sse_stream |
| 50 concurrent clients succeed | PASS | test_50_concurrent_clients |
| GET /health returns 200 under load | PASS | test_health_during_load |
| Batch requests return batch responses | PASS | test_post_batch_request |
| POST / over limit → 413 with JSON body | PASS | test_post_payload_too_large |
| Bearer auth → 401 with WWW-Authenticate | PASS | test_auth_required_for_non_loopback |

## Test Results

```
running 10 tests
test test_auth_required_for_non_loopback ... ok
test test_get_health ... ok
test test_50_concurrent_clients ... ok
test test_get_sse_stream ... ok
test test_health_during_load ... ok
test test_post_batch_request ... ok
test test_post_single_request_returns_single_response ... ok
test test_post_tools_list ... ok
test test_unknown_method ... ok
test test_post_payload_too_large ... ok

test result: ok. 10 passed; 0 failed; 0 ignored
```

## Usage

```bash
# Start MCP server with HTTP+SSE transport (loopback, no auth)
pdftract mcp --bind 127.0.0.1:8080

# Start with auth token required
pdftract mcp --bind 0.0.0.0:3000 --auth-token-file /path/to/token.txt

# Custom upload limit
pdftract mcp --bind 127.0.0.1:8080 --max-upload-mb 512
```

## Integration Points

- Reuses `crate::mcp::framing` JSON-RPC types (from bead 6.7.1)
- Reuses `crate::mcp::auth` bearer token resolution (from bead 6.7.7)
- Reuses `crate::mcp::bind` TH-03 security checks (from bead 6.7.7)
- SSE notifications broadcast via `tokio::sync::broadcast`

## References

- Plan section: Phase 6.7 MCP Server Mode (lines 2298-2303)
- MCP spec: https://modelcontextprotocol.io/spec/transports#http-with-sse
- ADR-006 (transport mutual exclusion)
