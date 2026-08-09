# Integration Test Signature and Module Structure Verification

## Task
Fix integration test function signatures, attributes, and module imports in tests/ and examples/ directories.

## Analysis
Based on the comprehensive audit in `notes/test-signature-audit.md`:

### Key Findings
- **Critical issues:** 0 - All actual test functions have correct signatures
- **Total test functions found:** 2,214 (2,146 with `#[test]`, 68 with `#[tokio::test]`)
- **All async tests:** Properly use `#[tokio::test]` attribute
- **Test signatures:** All match harness expectations

### Functions Without Test Attributes
The audit identified 16 functions with `test_*` prefix but without test attributes. All of these are **legitimate helper functions** called by other tests:

#### In tests/ directory:
1. `tests/fingerprint_reproducibility.rs::test_fixture_pair` - Helper function with parameters, called by `test_acrobat_resave_fixture()`, `test_qpdf_resave_fixture()`, etc.
2. `tests/json_schema.rs::test_fixture` - Helper function with parameters, called by `test_all_fixtures_schema_compliance()`, `test_simple_invoice()`, etc.

#### In crates/pdftract-core/tests/:
3. `cjk_encoding.rs::test_cjk_fixture` - Helper function with parameters and return type
4. `document_model.rs::test_fixture` - Helper fixture function
5. `encoding_recovery.rs::test_encoding_fixture` - Helper fixture function
6. `object_parser.rs::test_fixture` - Helper fixture function
7. `test_page_access.rs::test_fixture_path` - Helper function returning PathBuf
8. `memory_guard_tests.rs` - 11 functions with `#[cfg_attr(not(target_os = "windows"), test)]` attribute (conditional test attributes for Windows compatibility)

### Module Structure
Integration tests have proper module structure:
- Test functions use appropriate imports from `pdftract::*`
- Helper utilities are properly organized in `tests/test_helpers.rs`
- Test support library in `tests/lib.rs` provides common fixtures

### Examples/
Examples directory contains standalone Rust programs (not tests) with `fn main()` entry points - these are not integration tests and don't need test attributes.

## Verification Method
- Reviewed audit report `notes/test-signature-audit.md`
- Examined integration test files in `tests/` directory
- Confirmed helper functions have parameters (cannot be test functions)
- Verified conditional test attributes in memory_guard_tests.rs

## Conclusion
**No fixes needed.** The integration test signatures, attributes, and module structure are already correct and follow best practices:
- All test functions have correct `#[test]` or `#[tokio::test]` attributes
- All test functions have proper signatures (no parameters for unit tests)
- Helper functions are correctly named as helpers and called by actual tests
- Module imports are appropriate and organized
- Conditional test attributes are used for platform-specific tests

## Acceptance Criteria Status
1. ✅ All integration tests have correct attributes
2. ✅ All integration test signatures match harness expectations
3. ✅ Integration tests have proper module imports
4. N/A - Cannot run `cargo test --list` due to unrelated compilation errors in main code (PageExtractionError trait implementations, ResourceDict method calls)

## Note
The compilation errors preventing test runs are in the main codebase (conflicting trait implementations, method call mismatches) and are outside the scope of integration test signature fixes.
