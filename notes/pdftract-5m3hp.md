# pdftract-5m3hp: TH-03 test: unauthenticated mcp --bind on public address aborts with exit 78

## Summary
Implemented comprehensive security test suite for TH-03 (plan line 874) requiring authentication on non-loopback MCP server binds.

## Work Completed

### Test File Created
**File:** `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs` (529 lines)

### Test Coverage (11 scenarios)

#### Security Enforcement Tests
1. **test_case_1_ipv4_all_without_token** - Verifies exit code 78 when binding to 0.0.0.0:18125 without token
2. **test_case_2_ipv6_all_without_token** - Verifies exit code 78 when binding to [::]:0 without token
3. **test_case_5_ipv4_all_with_env_token** - Verifies successful bind with PDFTRACT_MCP_TOKEN environment variable
4. **test_case_6_ipv4_all_with_token_file** - Verifies successful bind with --auth-token-file argument

#### Loopback Exemption Tests
5. **test_case_3_ipv4_loopback_without_token** - Verifies 127.0.0.1:18123 bind succeeds without token
6. **test_case_4_ipv6_loopback_without_token** - Verifies [::1]:18124 bind succeeds without token
7. **test_case_7_localhost_without_token** - Verifies localhost:18126 bind succeeds without token (multi-address resolution)

#### Atomic Failure Verification
8. **test_atomic_failure_no_listener_during_failure** - Verifies process exits BEFORE binding listener (no connection window)

#### Exit Code Specificity Tests
9. **test_exit_code_is_78_not_any_nonzero** - Verifies exit code is specifically 78 (EX_CONFIG), not just any non-zero

#### Concurrency Tests
10. **test_parallel_bind_attempts_all_fail** - Verifies multiple parallel insecure binds all fail with exit 78

#### Edge Case Tests
11. **test_case_8_mixed_hostname_resolution** - Documented as #[ignore] (requires /etc/hosts control)

### Helper Functions Implemented
- `spawn_mcp_process()` - Spawns pdftract mcp with configurable bind address and auth tokens
- `wait_with_timeout()` - Waits for process completion with timeout and auto-kill
- `try_connect_to_bound_port()` - Verifies listener actually accepts connections
- `extract_bound_port()` - Parses bound port from server output

### Configuration Constants
- `EXIT_CONFIG_ERROR = 78` - Matches sysexits.h EX_CONFIG

## Code Quality
- All code formatted with rustfmt
- Proper error handling with timeout protection
- Generic type bounds for thread safety: `R: std::io::Read + Send + 'static`
- Comprehensive documentation for each test case

## Known Issue
**Pre-existing codebase compilation errors prevent test execution:**
- Missing `column` field in `SpanJson` initializers (19 instances)
- `CCITTFaxDecoder::decode()` API signature mismatch (6 instances)
- `ParsedCCITTParams` struct missing Result methods (6 instances)

These errors exist on the main branch before these changes (verified via git stash). The test file is syntactically correct and properly formatted.

## Acceptance Criteria Status

### PASS
- [x] Test file created at `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs`
- [x] All 11 TH-03 security scenarios implemented
- [x] Exit code 78 verification for non-loopback binds without token
- [x] Loopback address exemption tests (IPv4, IPv6, localhost)
- [x] Token authentication tests (env var, file)
- [x] Atomic failure verification (no listener during failure)
- [x] Proper rustfmt formatting
- [x] Comprehensive documentation and inline comments

### WARN
- [ ] **Tests cannot execute** due to pre-existing codebase compilation errors (unrelated to TH-03 work)
  - Error count: 31 compilation errors in existing code
  - Errors are in: `pdftract-cli/src/inspect/render/`, `pdftract-core/src/parser/stream.rs`, `pdftract-core/src/schema/mod.rs`
  - These errors exist on main branch before TH-03 changes

### FAIL
- None

## References
- Plan line 874: TH-03 MCP server requires authentication on non-loopback binds
- Existing implementation: `crates/pdftract-cli/src/mcp/bind.rs::check_bind_security()`
- Exit code constant: `crates/pdftract-cli/src/mcp/bind.rs::EXIT_CONFIG_ERROR`

## Commits
- (To be created)
