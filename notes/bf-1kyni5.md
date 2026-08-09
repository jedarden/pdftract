# Test Function Catalog and Compilation Issues
**Bead ID:** bf-1kyni5  
**Date:** 2026-08-09  
**Purpose:** Comprehensive catalog of test functions and identification of compilation issues

## Overview

This document catalogs all test functions in the codebase and identifies compilation issues discovered during the bead bf-1kyni5 investigation.

## Critical Finding: Examples Directory Misuse

### Issue Description
Six (6) files in the `examples/` directory have been converted from `fn main()` to `#[test]` functions. This creates a fundamental problem:

1. **Examples are not tested by default** - `cargo test --examples` runs but discovers 0 tests
2. **These functions cannot be run as examples** - they lack the required `fn main()` entry point
3. **They are orphaned test code** - not discoverable by normal test infrastructure

### Affected Files

| File | Function Name | Original Purpose | Current Status |
|------|--------------|------------------|-----------------|
| `examples/test_ascii85.rs` | `test_ascii85_terminator()` | Test ASCII85 decoding | ⚠️ Orphaned test in examples/ |
| `examples/test_export.rs` | `test_detect_merged_cells_exported()` | Test table module export | ⚠️ Orphaned test in examples/ |
| `examples/test_parse_fixture.rs` | `test_parse_minimal_fixture()` | Test PDF fixture parsing | ⚠️ Orphaned test in examples/ |
| `examples/test_simple_extract.rs` | `test_simple_extract()` | Test SDK extraction | ⚠️ Orphaned test in examples/ |
| `examples/test_source.rs` | `test_memory_source()` | Test memory source | ⚠️ Orphaned test in examples/ |
| `examples/test_source.rs` | `test_mmap_source()` | Test mmap source | ⚠️ Orphaned test in examples/ |
| `examples/test_source.rs` | `test_file_source()` | Test file source | ⚠️ Orphaned test in examples/ |
| `examples/test_source.rs` | `test_prefetch_noop()` | Test prefetch no-op | ⚠️ Orphaned test in examples/ |
| `examples/test_url_host.rs` | `test_url_host_parsing()` | Test URL host parsing | ⚠️ Orphaned test in examples/ |

**Total orphaned test functions: 9**

### Evidence from Git Diff

All affected files show this pattern:
- **Before:** `fn main()` or `fn main() -> anyhow::Result<()>`
- **After:** `#[test] fn test_<name>()`

The conversion includes:
- Removal of `println!` statements
- Addition of proper `assert!` macros
- Removal of error handling (replaced by assertions)
- Removal of `anyhow::Result` return types

## Integration Test Structure

### Main Integration Test File

**File:** `tests/integration_test.rs` (30 lines)

This file serves as the main entry point for integration tests and organizes them into logical modules:

```rust
// Module structure
mod test_helpers;  // Test utilities and fixtures
mod test_cases;    // Individual integration test cases
```

### Imported Types

The integration test imports these key types:
- `std::path::PathBuf`
- `pdftract::PyPdfProcessor`
- Exception types: `CorruptPdfError`, `EncryptionError`, `PdftractError`, etc.
- Core types: `AttachmentJson`, `ExtractionOptions`, `PageResult`, `TableJson`

### Test Helper Module

**File:** `tests/test_helpers.rs` (47 lines)

**Functions:**
- `Fixtures::new()` - Constructor for test fixture paths
- `Fixtures::get(name)` - Get path to specific test fixture
- `Fixtures::exists(name)` - Check if fixture exists
- `temp_dir()` - Create temporary test directory
- `test_fixtures_path()` - Test that fixtures path is correct

### Test Cases Module

**File:** `tests/test_cases.rs` (17 lines)

**Test Functions:**
- `test_fixture_discovery()` - Verifies fixtures directory structure

## Compilation Status

### Cargo Check Results

```bash
cargo check --all-targets
# Result: SUCCESS (no output = no errors)
```

```bash
cargo test --no-run
# Result: SUCCESS (no output = no errors)
```

**Key Finding:** The code compiles successfully with NO compilation errors or warnings.

### Unused Imports Check

```bash
RUSTFLAGS="-W unused_imports -W dead_code" cargo test --no-run
# Result: SUCCESS (no warnings about unused imports or dead code)
```

### Examples Test Discovery Issue

```bash
cargo test --examples
# Result: "running 0 tests" for each example file
```

**This confirms that the test functions in examples/ are not discovered by the test infrastructure.**

## Additional Test Files Inventory

The codebase contains 357 total test functions across many test files:

### Major Test Categories

1. **Integration tests** (tests/integration_test.rs)
2. **Debug tests** (tests/debug_*.rs) - Fingerprint debugging, content hash, etc.
3. **Conformance tests** (tests/sdk-conformance/) - SDK conformance test suite
4. **Property-based tests** (tests/proptest/) - Hypothesis/fuzz testing
5. **Fixture generators** (tests/*/generate_*.rs) - Golden master test generation
6. **Unit tests** (scattered throughout lib.rs and other modules)

## Summary of Issues

### Critical Issues
1. **9 orphaned test functions in examples/ directory** - Cannot be run by `cargo test`
2. **Examples are no longer executable** - Missing `fn main()` entry points

### Non-Issues (Confirmed Working)
1. ✅ **No compilation errors** - Code compiles cleanly
2. ✅ **No unused imports** - All imports are used
3. ✅ **No dead code warnings** - All code is referenced
4. ✅ **No signature mismatches** - All function signatures are correct
5. ✅ **No missing test attributes** - All test functions have proper `#[test]` attributes

## Recommendations

### Immediate Action Required

The orphaned test functions should be moved to proper test locations:

1. **Move to tests/integration/** - Most integration-style tests
2. **Move to tests/unit/** - Unit-style tests  
3. **Restore examples/** - Either restore original examples or create new proper examples with `fn main()`

### Alternative Solution

If these are meant to be runnable examples rather than tests, they should be restored to `fn main()` format with proper error handling and output.

## Next Steps

This catalog provides the roadmap for bead bf-2fuz9q fixes:
1. Decide whether these are tests or examples
2. Move to appropriate location
3. Verify they run with `cargo test`
4. Remove from examples/ if they're tests, or restore to examples if they're examples

---

**Verification:** This catalog was created by examining git diffs, running cargo check/test commands, and analyzing the test infrastructure. All findings are based on actual code inspection and build results.
