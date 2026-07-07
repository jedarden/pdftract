# TH-05 Test Files and Suites Inventory

## Overview
This document catalogs all TH-05 (SSRF protection) test files and suites in the pdftract repository.

## Test Files

### 1. Core Library Tests
**File:** `/home/coding/pdftract/crates/pdftract-core/tests/TH-05-ssrf-block.rs`
**Feature flag:** `#![cfg(feature = "remote")]`
**Purpose:** Unit-level SSRF protection validation for URL validation logic

**Test count:** 12 test functions

#### Test List (Core):
1. `test_ssrf_protection_blocks_all_dangerous_payloads()` - Validates that all SSRF payloads are rejected
2. `test_allow_private_networks_bypass()` - Tests `--allow-private-networks` flag behavior
3. `test_public_urls_are_accepted()` - Positive test for public URLs
4. `test_http_scheme_always_rejected()` - Validates http:// scheme rejection
5. `test_file_scheme_always_rejected()` - Validates file:// scheme rejection
6. `test_ftp_scheme_always_rejected()` - Validates ftp:// scheme rejection
7. `test_url_with_basic_auth_rejected()` - Tests URLs with embedded credentials
8. `test_ipv6_zone_id_detected_as_link_local()` - IPv6 link-local detection
9. `test_metadata_subdomain_detected()` - Cloud metadata subdomain blocking
10. `test_url_validation_returns_correct_diagnostic_code()` - Diagnostic code validation
11. `test_private_ipv4_boundary_addresses()` - Edge case testing for private ranges
12. `test_current_network_range_blocked()` - 0.0.0.0/8 blocking

#### SSRF Payload Categories Tested:
- Cloud metadata endpoints (AWS, GCP, Azure, Alibaba)
- RFC 1918 private IPv4 ranges (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
- Loopback addresses (127.0.0.0/8)
- Link-local addresses (169.254.0.0/16)
- IPv6 ULA (fc00::/7, fd00::/8)
- IPv6 loopback (::1)
- IPv6 link-local (fe80::/10)
- Non-https schemes (http://, ftp://, file://)

### 2. CLI Integration Tests
**File:** `/home/coding/pdftract/crates/pdftract-cli/tests/TH-05-ssrf-block.rs`
**Purpose:** Integration tests for MCP server SSRF protection via JSON-RPC

**Test count:** 7 test functions

#### Test List (CLI):
1. `test_ipv4_loopback_blocked()` - Tests 127.0.0.1 rejection via MCP
2. `test_ipv4_wildcard_blocked()` - Tests 0.0.0.0 rejection via MCP
3. `test_cloud_metadata_blocked()` - Tests AWS metadata endpoint blocking
4. `test_rfc1918_private_blocked()` - Tests 10.0.0.1 private network blocking
5. `test_ipv6_loopback_blocked()` - Tests IPv6 ::1 rejection
6. `test_http_scheme_rejected()` - Tests http:// scheme rejection
7. `test_no_network_connection_attempted()` - Verifies no network calls are made

## Test Suite Summary

| Suite Name | Location | Test Count | Feature Required |
|------------|----------|------------|------------------|
| TH-05-ssrf-block | crates/pdftract-core/tests/ | 12 | `remote` |
| TH-05-ssrf-block | crates/pdftract-cli/tests/ | 7 | none (but needs built binary) |

**Total tests:** 19 test functions

## Running the Tests

### Command for core tests:
```bash
cargo nextest run --features remote --test-threads=1 TH-05-ssrf-block
```

### Command for CLI tests:
```bash
cargo nextest run --test-threads=1 TH-05-ssrf-block
```

### Run all TH-05 tests:
```bash
cargo nextest run --features remote --test-threads=1 -- TH-05-ssrf-block
```

**Note:** Tests use `--test-threads=1` because they spawn MCP server subprocesses and need to avoid port conflicts.

## Test Categories

### Unit Tests (Core)
- Direct URL validation function calls
- No subprocess spawning
- Test the `validate_url()` function directly
- Fast, isolated tests

### Integration Tests (CLI)
- Spawn actual `pdftract mcp --stdio` subprocess
- Full JSON-RPC request/response cycle
- Test real MCP server behavior
- Slower due to subprocess overhead

## Related Files
- `/home/coding/pdftract/crates/pdftract-core/src/url_validation.rs` - URL validation implementation
- TH-05 threat model coverage in plan.md

## Notes
- Both test files require Phase 1.8 SSRF protection to be fully implemented
- CLI tests currently accept stub responses until Phase 1.8 is complete
- Core tests validate the URL validation logic independently of MCP
