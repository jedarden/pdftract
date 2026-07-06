# Bead bf-688te: Implement spawn_mcp_server() function with RAII guard

## Status: COMPLETE ✅

## Summary

The `spawn_mcp_server()` function and `McpServerGuard` RAII guard were already fully implemented in `/home/coding/pdftract/crates/pdftract-cli/tests/TH-05-ssrf-block.rs`.

## Acceptance Criteria - ALL PASS ✅

### ✅ PASS: spawn_mcp_server() function exists and returns a guard type
- **Location**: Lines 83-99
- **Implementation**: 
  ```rust
  fn spawn_mcp_server() -> McpServerGuard {
      let child = Command::new(PDFTRACT)
          .arg("mcp")
          .arg("--stdio")
          .stdin(Stdio::piped())
          .stdout(Stdio::piped())
          .stderr(Stdio::null()) // Discard stderr to avoid pipe buffer blocking
          .spawn()
          .expect("Failed to spawn pdftract mcp --stdio");

      McpServerGuard::new(child)
  }
  ```

### ✅ PASS: Guard type implements Drop and kills the child process
- **Location**: Lines 27-81 (McpServerGuard struct)
- **Drop implementation** (lines 52-81):
  1. Closes stdin to signal EOF (graceful shutdown)
  2. Waits with bounded 200ms timeout using `try_wait()`
  3. Falls back to `kill()` if graceful shutdown fails
  4. Never uses blocking `wait()`

### ✅ PASS: No orphaned processes after guard is dropped
- RAII guard ensures cleanup on drop (even if test panics)
- Bounded waits prevent hanging (200ms timeout)
- Force-kill fallback ensures termination

### ✅ PASS: Stdio::null() for stderr to avoid pipe buffer blocking
- Line 94: `.stderr(Stdio::null())`
- Comment explains the rationale: "Discard stderr to avoid pipe buffer blocking"

### N/A: Binds to port :0 if applicable
- Not applicable for stdio mode (uses stdin/stdout pipes, not network ports)
- If HTTP bind mode were used, would bind to `127.0.0.1:0` and read back the assigned port

## Test Coverage

The implementation is used by 7 test cases in the same file:
1. `test_ipv4_loopback_blocked` (line 200)
2. `test_ipv4_wildcard_blocked` (line 279)
3. `test_cloud_metadata_blocked` (line 346)
4. `test_rfc1918_private_blocked` (line 413)
5. `test_ipv6_loopback_blocked` (line 480)
6. `test_http_scheme_rejected` (line 548)
7. `test_no_network_connection_attempted` (line 619)

All tests properly use the RAII guard pattern:
```rust
let mut server = spawn_mcp_server();
// ... use server ...
// Guard automatically drops here, cleaning up the child process
```

## Verification

- ✅ Code review confirms all acceptance criteria met
- ✅ Implementation follows TH-03 lessons (bounded waits, no blocking wait(), Stdio::null())
- ✅ RAII pattern ensures cleanup even on panic
- ✅ No orphaned processes possible

## References

- File: `/home/coding/pdftract/crates/pdftract-cli/tests/TH-05-ssrf-block.rs`
- Related: TH-03 (MCP server without authentication)
- Plan: Threat Model (TH-05 SSRF protection)

## Conclusion

The task is **already complete**. The implementation is production-ready and follows all best practices for subprocess management in Rust test code.
