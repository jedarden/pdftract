# Test Signature Audit Issues

Generated: Sun Aug  9 07:46:37 AM EDT 2026

## Summary

**Files scanned:** 226
**Total test functions found:** 1,160
**Issues found:** 16 (partial scan - full results pending from Explore agent)


## Issues by Category

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/cjk_encoding.rs:59
- **Function:** test_cjk_fixture
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/document_model.rs:121
- **Function:** test_fixture
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/encoding_recovery.rs:109
- **Function:** test_encoding_fixture
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/memory_guard_tests.rs:16
- **Function:** test_large_vector_allocation_fails_gracefully
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/memory_guard_tests.rs:33
- **Function:** test_oversized_decompression_fails_gracefully
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/memory_guard_tests.rs:57
- **Function:** test_hashmap_under_memory_pressure
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/memory_guard_tests.rs:76
- **Function:** test_try_reserve_propagates_failure
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/memory_guard_tests.rs:102
- **Function:** test_string_try_reserve_fails_gracefully
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/memory_guard_tests.rs:116
- **Function:** test_box_allocation_under_limit
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/memory_guard_tests.rs:130
- **Function:** test_multiple_allocations_under_tight_budget
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/memory_guard_tests.rs:148
- **Function:** test_vec_resize_fails_gracefully
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/memory_guard_tests.rs:162
- **Function:** test_string_from_large_bytes_fails_gracefully
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/memory_guard_tests.rs:175
- **Function:** test_nested_allocations_under_limit
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/object_parser.rs:124
- **Function:** test_fixture
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./crates/pdftract-core/tests/test_page_access.rs:20
- **Function:** test_fixture_path
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./tests/fingerprint_reproducibility.rs:189
- **Function:** test_fixture_pair
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

### MISSING_TEST_ATTRIBUTE
- **File:** ./tests/json_schema.rs:101
- **Function:** test_fixture
- **Issue:** function named 'test_*' without #[test] or #[tokio::test] attribute

## Test Functions with Parameters


## Test Functions by Issue Type

**Files scanned:** 226
**Total test functions found:** 1160
### ASYNC_TEST_FUNCTIONS (with #[tokio::test])

- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_bandwidth_limited_extraction` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_no_range_support` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_416_retry_without_range` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_linearized_pdf` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_connection_drop` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_basic_auth` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_unauthorized` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_forbidden` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_custom_headers` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_cache_behavior` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_block_boundary_crossing` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_mock_server_tests.rs** - `async fn test_read_beyond_eof` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/test_416_debug.rs** - `async fn test_416_retry_debug` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_tls_tests.rs** - `async fn test_tls_self_signed_cert_rejected` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_tls_tests.rs** - `async fn test_tls_expired_cert_rejected` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_tls_tests.rs** - `async fn test_tls_wrong_host_rejected` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_tls_tests.rs** - `async fn test_tls_error_exit_code` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_tls_tests.rs** - `async fn test_tls_valid_cert_works` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_tls_tests.rs** - `async fn test_tls_connection_timeout` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_tls_tests.rs** - `async fn test_inv8_no_panic_on_tls_errors` - has #[tokio::test] ✓
- **./crates/pdftract-core/tests/remote_tls_tests.rs** - `async fn test_http_no_tls_validation` - has #[tokio::test] ✓
- **./tests/remote/integration.rs** - `async fn test_range_support_page_5_of_100` - has #[tokio::test] ✓
- **./tests/remote/integration.rs** - `async fn test_no_range_fallback` - has #[tokio::test] ✓
- **./tests/remote/integration.rs** - `async fn test_416_range_not_satisfiable` - has #[tokio::test] ✓
- **./tests/remote/integration.rs** - `async fn test_linearized_hint_stream_prefetch` - has #[tokio::test] ✓
- **./tests/remote/integration.rs** - `async fn test_connection_drop_interrupted` - has #[tokio::test] ✓
- **./tests/remote/integration.rs** - `async fn test_http_source_basic_creation` - has #[tokio::test] ✓
- **./tests/remote/integration.rs** - `async fn test_http_source_read_trait` - has #[tokio::test] ✓
- **./tests/remote/integration.rs** - `async fn test_http_source_seek_trait` - has #[tokio::test] ✓


## Test Functions with Parameters (Suspicious)


## Analysis: memory_guard_tests.rs

mod memory_guard;
use std::io::Cursor;
fn test_large_vector_allocation_fails_gracefully() {
fn test_oversized_decompression_fails_gracefully() {

The memory_guard_tests.rs functions might be helper functions called by other tests, not meant to run standalone.
## Detailed Analysis of Problematic Functions

### memory_guard_tests.rs - Module Structure
```rust
//! Allocation-sensitive tests using the memory-guard helper.
//!
//! These tests verify that code fails gracefully under memory pressure.
//! All tests are tagged to skip on Windows (which doesn't support
//! per-process memory limits).
//!
//! See `memory_guard.rs` for the helper implementation and usage convention.

mod memory_guard;

use std::io::Cursor;

/// Test that large vector allocations fail gracefully under memory limits.
#[cfg_attr(not(target_os = "windows"), test)]
#[ignore = "memory limit tests interfere with each other when run in the same process"]
fn test_large_vector_allocation_fails_gracefully() {
    use memory_guard::assert_fails_under_memory_limit;

    // Try to allocate 1 GB under a 100 MB limit
    assert_fails_under_memory_limit(100 * 1024 * 1024, || {
```

### cjk_encoding.rs - test_cjk_fixture

/// Test a single CJK fixture.
fn test_cjk_fixture(fixture: &CjkFixture) -> Result<String, Box<dyn std::error::Error>> {
    let pdf_path = Path::new(&fixture.pdf_path);

    // Open the PDF
    let mut extractor =
        PdfExtractor::open(pdf_path).map_err(|e| format!("Failed to open PDF: {}", e))?;

## TH- (Threat Model) Test Files

Found TH- test files:
- ./crates/pdftract-cli/tests/TH-02-path-traversal.rs: 10 #[test], 0
0 #[tokio::test]
- ./crates/pdftract-cli/tests/TH-05-ssrf-block.rs: 7 #[test], 0
0 #[tokio::test]
- ./crates/pdftract-cli/tests/TH-08-log-audit.rs: 6 #[test], 0
0 #[tokio::test]
- ./crates/pdftract-cli/tests/TH-09-inspector-xss.rs: 5 #[test], 0
0 #[tokio::test]
- ./crates/pdftract-core/tests/TH-01-stream-bomb.rs: 5 #[test], 0
0 #[tokio::test]
- ./crates/pdftract-core/tests/TH-03-mcp-no-auth.rs: 11 #[test], 0
0 #[tokio::test]
- ./crates/pdftract-core/tests/TH-04-js-presence.rs: 4 #[test], 0
0 #[tokio::test]
- ./crates/pdftract-core/tests/TH-05-ssrf-block.rs: 66 #[test], 0
0 #[tokio::test]
- ./crates/pdftract-core/tests/TH-07-ps-leak.rs: 7 #[test], 0
0 #[tokio::test]
- ./crates/pdftract-core/tests/TH-10-cache-poison.rs: 10 #[test], 0
0 #[tokio::test]

Checking TH- files for signature issues...


## Summary of Issue Types

### 1. Missing Test Attributes (16 cases)
Functions named with `test_*` prefix but missing `#[test]` or `#[tokio::test]` attributes. These may be:
- **Helper functions** called by other tests (legitimate)
- **Forgotten attributes** that should be tests (needs fixing)

**Files affected:**
- `crates/pdftract-core/tests/memory_guard_tests.rs` (11 functions)
- `crates/pdftract-core/tests/cjk_encoding.rs` (1 function)
- `crates/pdftract-core/tests/document_model.rs` (1 function)
- `crates/pdftract-core/tests/encoding_recovery.rs` (1 function)
- `crates/pdftract-core/tests/object_parser.rs` (1 function)
- `crates/pdftract-core/tests/test_page_access.rs` (1 function)
- `tests/fingerprint_reproducibility.rs` (1 function)
- `tests/json_schema.rs` (1 function)

### 2. Async Test Functions (All Proper)
All async test functions found correctly use `#[tokio::test]` attribute. No issues found.

**Files with async tests:**
- `crates/pdftract-core/tests/remote_mock_server_tests.rs` (12 async tests)
- `crates/pdftract-core/tests/remote_tls_tests.rs` (10 async tests)
- `crates/pdftract-core/tests/test_416_debug.rs` (1 async test)
- `tests/remote/integration.rs` (7 async tests)

### 3. Test Function Parameters
Most test functions have no parameters (as expected). Some helper functions may have parameters for setup/fixture generation.

## Recommendations

1. **Review memory_guard_tests.rs**: The 11 `test_*` functions without attributes may be intended as test helpers. If they should run as tests, add `#[test]` attributes.

2. **Check fixture helper functions**: Functions like `test_fixture`, `test_cjk_fixture` appear to be helpers called by other tests. Ensure they are intentionally not marked as tests.

3. **Maintain async test pattern**: Continue using `#[tokio::test]` for all async test functions - current practice is correct.

4. **CI validation**: Consider adding a CI check that prevents new `test_*` functions from being added without the appropriate `#[test]` or `#[tokio::test]` attribute.

## Audit Methodology

This audit scanned:
- **226 test files** across the entire project
- **1,160 test functions** with `#[test]` or `#[tokio::test]` attributes
- All files matching: `*/tests/*.rs`, `*/examples/test*.rs`, `*/test_*.rs`

Issues were identified by:
1. Parsing function names and attributes
2. Checking for `test_*` naming without corresponding test attributes
3. Verifying async functions have `#[tokio::test]`
4. Checking parameter patterns in test functions
