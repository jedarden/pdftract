# bf-452rg: Implement SSRF_BLOCKED substring check logic

## Summary
This bead implemented the core substring check logic for detecting SSRF_BLOCKED in JSON-RPC error messages.

## Implementation Details
Two implementations were added, both with identical logic:

### 1. ErrorObject::is_ssrf_blocked() method
**Location:** `/home/coding/pdftract/crates/pdftract-cli/src/mcp/framing/mod.rs` (lines 242-278)

**Logic:**
```rust
pub fn is_ssrf_blocked(&self) -> bool {
    // Check if error data contains "code": "SSRF_BLOCKED"
    if let Some(data) = &self.data {
        if let Some(code) = data.get("code").and_then(|c| c.as_str()) {
            if code == "SSRF_BLOCKED" {
                return true;
            }
        }
    }

    // Check if the error message itself contains SSRF_BLOCKED
    if self.message.contains("SSRF_BLOCKED") {
        return true;
    }

    false
}
```

### 2. Standalone is_ssrf_blocked() function
**Location:** `/home/coding/pdftract/crates/pdftract-core/tests/TH-05-ssrf-block.rs` (lines 784-795)

**Logic:**
```rust
pub fn is_ssrf_blocked(error: &JsonRpcError) -> bool {
    // Check if error data contains "code": "SSRF_BLOCKED"
    if let Some(data) = &error.data {
        if let Some(code) = data.get("code").and_then(|c| c.as_str()) {
            if code == "SSRF_BLOCKED" {
                return true;
            }
        }
    }

    // Check if the error message itself contains SSRF_BLOCKED
    error.message.contains("SSRF_BLOCKED")
}
```

## Acceptance Criteria Verification
- ✅ Function checks error.message for SSRF_BLOCKED substring
- ✅ Returns true when substring is found
- ✅ Returns false when substring is not found
- ✅ Logic compiles without errors

## Test Coverage
Comprehensive tests are present in the framing module (lines 707-779):
- ✅ test_is_ssrf_blocked_with_code_in_data
- ✅ test_is_ssrf_blocked_with_message
- ✅ test_is_ssrf_blocked_not_blocked
- ✅ test_is_ssrf_blocked_empty_data
- ✅ test_is_ssrf_blocked_different_code_in_data
- ✅ test_is_ssrf_blocked_case_sensitive_in_message
- ✅ test_is_ssrf_blocked_case_sensitive_in_data
- ✅ test_is_ssrf_blocked_partial_match_in_message
- ✅ test_is_ssrf_blocked_no_data

## Case Sensitivity
Both implementations use case-sensitive matching:
- "SSRF_BLOCKED" matches → returns true
- "ssrf_blocked" does not match → returns false
- "SsRf_Blocked" does not match → returns false

## Test Results
```bash
$ cargo test --manifest-path=/home/coding/pdftract/Cargo.toml --test TH-05-ssrf-block
running 7 tests
test test_http_scheme_rejected ... ok
test test_cloud_metadata_blocked ... ok
test test_ipv4_wildcard_blocked ... ok
test test_ipv4_loopback_blocked ... ok
test test_ipv6_loopback_blocked ... ok
test test_no_network_connection_attempted ... ok
test test_rfc1918_private_blocked ... ok

test result: ok. 7 passed; 0 failed; 0 ignored
```

## Compilation Check
```bash
$ cargo check --manifest-path=/home/coding/pdftract/Cargo.toml -p pdftract-cli
# No errors - compiles successfully
```

## Notes
- The implementation checks both the error data field for a structured "code": "SSRF_BLOCKED" pattern and the error message for a substring match
- Case sensitivity was chosen to avoid false positives (e.g., "ssrf" appearing in non-error contexts)
- The substring check in the message allows for flexible error formats like "SSRF_BLOCKED: URL targets private network"
