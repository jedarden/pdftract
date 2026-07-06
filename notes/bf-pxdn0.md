# Bead bf-pxdn0: Add SSRF_BLOCKED helper function signature

## Summary

Added a standalone helper function `is_ssrf_blocked()` to the `mcp_helpers` module in the TH-05 SSRF test file. The function accepts a `JsonRpcError` reference and returns a boolean.

## Implementation

**File Modified:** `/home/coding/pdftract/crates/pdftract-core/tests/TH-05-ssrf-block.rs`

**Function Added:**
```rust
pub fn is_ssrf_blocked(error: &JsonRpcError) -> bool {
    // Stub implementation - returns false for now
    // TODO: Implement proper SSRF_BLOCKED detection logic
    false
}
```

**Location:** Added after the `is_ssrf_blocked_error()` function (around line 754) in the `mcp_helpers` module.

## Acceptance Criteria Status

- ✅ **PASS**: Function exists with correct signature accepting `JsonRpcError` reference
- ✅ **PASS**: Function returns `bool` type
- ✅ **PASS**: Function compiles successfully (verified with `cargo check --tests -p pdftract-core`)
- ✅ **PASS**: Function is a stub that returns `false`

## Notes

- The function is added as a standalone helper in the `mcp_helpers` module (line 754-770)
- It follows the same pattern as other helper functions in the module (e.g., `is_ssrf_blocked_error()`)
- The function is well-documented with comprehensive doc comments including usage examples
- This provides a clean API for checking SSRF errors separate from the method on `JsonRpcError` itself
- The implementation is intentionally left as a stub (returns `false`) as specified in the acceptance criteria

## Testing

The code compiles successfully without any errors. The function signature matches the requirements exactly and can be called with a `JsonRpcError` reference:

```rust
let error = JsonRpcError { /* ... */ };
if is_ssrf_blocked(&error) {
    // Handle SSRF blocked case
}
```
