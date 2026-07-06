# bf-2rwx6: Verify SSRF_BLOCKED assertion applied to all SSRF URL tests

## Task
Apply SSRF_BLOCKED assertion to all SSRF URL tests

## Summary
Verified that all 5 SSRF URL test cases in `crates/pdftract-cli/tests/TH-05-ssrf-block.rs` correctly use the `assert_ssrf_blocked_error` assertion helper.

## Verification Details

### All 5 SSRF URL tests verified:

1. **test_ipv4_loopback_blocked** (line 229)
   - Calls: `assert_ssrf_blocked_error(&response, "IPv4 loopback (127.0.0.1)")`
   - URL tested: `http://127.0.0.1:9999/doc.pdf`
   - Status: ✓ PASS

2. **test_ipv4_wildcard_blocked** (line 266)
   - Calls: `assert_ssrf_blocked_error(&response, "IPv4 wildcard (0.0.0.0)")`
   - URL tested: `http://0.0.0.0/doc.pdf`
   - Status: ✓ PASS

3. **test_cloud_metadata_blocked** (line 303)
   - Calls: `assert_ssrf_blocked_error(&response, "Cloud metadata endpoint (169.254.169.254)")`
   - URL tested: `http://169.254.169.254/latest/meta-data/`
   - Status: ✓ PASS

4. **test_rfc1918_private_blocked** (line 340)
   - Calls: `assert_ssrf_blocked_error(&response, "RFC 1918 private network (10.0.0.1)")`
   - URL tested: `http://10.0.0.1/internal/doc.pdf`
   - Status: ✓ PASS

5. **test_ipv6_loopback_blocked** (line 377)
   - Calls: `assert_ssrf_blocked_error(&response, "IPv6 loopback ([::1])")`
   - URL tested: `http://[::1]/doc.pdf`
   - Status: ✓ PASS

### Additional Verification

- No SSRF URL tests use the stub-accepting helper `assert_ssrf_blocked_or_stub`
- The stub-accepting helper exists in the file (lines 402-460) for backward compatibility during Phase 1.8 development, but none of the SSRF URL tests use it
- All tests correctly pass the response JSON string and test description to the assertion helper
- The `assert_ssrf_blocked_error` helper is implemented at lines 472-518 and performs strict SSRF_BLOCKED error checking

## Acceptance Criteria

- [x] All 5 SSRF URL tests call `assert_ssrf_blocked_error`
- [x] Each test passes response JSON and description correctly
- [x] No tests use stub-accepting helper (`assert_ssrf_blocked_or_stub`)
- [x] Verification note created at notes/bf-2rwx6.md

## Test Status: PASS

All 5 SSRF URL tests correctly use the assertion helper. The implementation properly enforces SSRF blocking verification without accepting stub responses.

## File Verified
- `/home/coding/pdftract/crates/pdftract-cli/tests/TH-05-ssrf-block.rs`
