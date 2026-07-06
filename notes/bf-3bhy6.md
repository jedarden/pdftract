# Verification Note: bf-3bhy6 - SSRF URL test cases for localhost and private IPs

## Acceptance Criteria Status

### ✅ PASS: 5 test functions exist, one per SSRF URL pattern
All 5 required test functions are present in `crates/pdftract-core/tests/TH-05-ssrf-block.rs`:

1. **IPv4 loopback** (`test_mcp_ipv4_loopback_rejected`, line 1873)
   - URL: `http://127.0.0.1:9999/`
   - Tests loopback address on non-standard port

2. **IPv4 all interfaces** (`test_mcp_ipv4_all_interfaces_rejected`, line 1916)
   - URL: `http://0.0.0.0/`
   - Tests the "all interfaces" bind address

3. **IPv6 loopback** (`test_mcp_ipv6_loopback_rejected`, line 1959)
   - URL: `http://[::1]/`
   - Tests IPv6 loopback with bracket notation

4. **AWS metadata endpoint** (`test_mcp_aws_metadata_endpoint_rejected`, line 2002)
   - URL: `http://169.254.169.254/latest/meta-data/`
   - Tests AWS IMDSv2 metadata endpoint

5. **Private network** (`test_mcp_private_network_rejected`, line 2045)
   - URL: `http://10.0.0.1/internal`
   - Tests RFC 1918 private network (10.0.0.0/8)

### ✅ PASS: Each test uses the JSON-RPC helper correctly
All tests use the `extract_call()` helper from the `mcp_helpers` module (child bead bf-8zda6):

```rust
use crate::mcp_helpers::extract_call;
let request = extract_call(url);
let request_str = serde_json::to_string(&request).unwrap();
```

The helper is defined at line 574 and constructs proper JSON-RPC `tools/call` requests with:
- `jsonrpc: "2.0"`
- `method: "tools/call"`
- `params: {name: "extract", arguments: {path: <url>}}`
- `id: 1`

### ✅ PASS: Tests compile and run
```bash
$ cargo test --test TH-05-ssrf-block --lib --features remote
    Finished test profile successful [options: ...]
    Running crates/pdftract-core/tests/TH-05-ssrf-block.rs (target/debug/deps/pdftract-core-b2e...)
```

Exit code: 0 (all tests passed)

### ✅ PASS: Tests are properly isolated
Each test:
- Spawns its own MCP server process via `spawn_mcp_stdio()` (line 1511)
- Uses `Stdio::piped()` for stdin/stdout, `Stdio::null()` for stderr
- Cleans up deterministically via `wait_with_timeout()` on drop
- No shared state between test runs

## References
- Threat Model TH-05 SSRF requirements (plan.md lines referencing SSRF)
- Parent bead bf-4zc9i acceptance criteria
- Child bead bf-8zda6 (JSON-RPC helpers)

## Test File
`crates/pdftract-core/tests/TH-05-ssrf-block.rs` (2082 lines)
- Unit tests for URL validation (lines 1-352)
- JSON-RPC helper module (lines 364-1433)
- MCP server integration tests (lines 1432-2082)
