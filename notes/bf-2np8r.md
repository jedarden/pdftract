# bf-2np8r: Add common test fixtures and helpers for encryption tests

## Summary

Successfully implemented common test fixtures and helper functions for encryption testing in the pdftract workspace.

## Implementation

### Files Created

1. **`tests/encryption_fixtures.rs`** (461 lines)
   - Comprehensive module providing shared utilities for encryption tests
   - Path resolution functions: `workspace_root()`, `pdftract_bin()`, `encrypted_fixture()`, `fixture()`
   - Test execution helpers: `run_pdftract_extract()`, `run_pdftract_extract_with_stdin_password()`
   - Assertion helpers: `assert_encryption_diagnostic()`, `assert_encryption_exit_code()`, `assert_success_exit_code()`, `assert_failure_exit_code()`, `assert_output_contains()`
   - Fixture validation: `assert_fixture_exists()`, `assert_valid_pdf_structure()`, `assert_encrypted_fixtures_exist()`
   - Mock data builders (with `decrypt` feature): `make_dict()`, `make_trailer()`, `make_rc4_encryption_dict()`, `make_aes128_encryption_dict()`, `make_aes256_encryption_dict()`, `make_unsupported_encryption_dict()`
   - Test constants: `EXPECTED_ENCRYPTION_EXIT_CODE`, `ENCRYPTED_FIXTURES`, `TEST_PASSWORD`, `WRONG_PASSWORD`
   - Includes module tests verifying all functionality

2. **`tests/lib.rs`** (7 lines)
   - Test support library for pdftract integration tests
   - Exports `encryption_fixtures` module for use across workspace

3. **`tests/mod.rs`** (7 lines)
   - Test module aggregation
   - Includes `encryption_fixtures` as public module

4. **`tests/ENCRYPTION_FIXTURES.md`** (161 lines)
   - Comprehensive documentation for the fixtures module
   - Usage examples and API reference
   - Maintenance guidelines

5. **`tests/verify_encryption_fixtures.rs`** (137 lines)
   - Verification tests for the fixtures module
   - Tests all helper functions and mock builders
   - Validates fixture path resolution

6. **`tests/encryption_fixtures_usage_example.rs`** (81 lines)
   - Example usage demonstrating fixture module
   - Reference implementation for other test files

### Acceptance Criteria Status

- ✅ **Common test fixtures added**: Created comprehensive fixture list with 5 encrypted PDFs
- ✅ **Helper functions added**: 20+ helper functions for path resolution, test execution, assertions, and fixture validation
- ✅ **Code compiles successfully**: Verified with `cargo check --tests` - no compilation errors
- ✅ **Functions are usable across multiple encryption tests**: Module is publicly exported and can be imported with `use pdftract_tests::encryption_fixtures::*`

## Test Coverage

### Verification Tests Created

1. Path resolution tests (5 tests)
2. Constant validation tests (4 tests)
3. Mock builder tests (4 tests, feature-gated on `decrypt`)
4. Fixture validation tests (3 tests)
5. Integration tests demonstrating usage

### Encrypted Fixtures Available

- `livecycle.pdf` - Adobe LiveCycle unsupported encryption
- `EC-04-rc4-encrypted.pdf` - RC4-40 encryption (V=1, R=2)
- `EC-05-aes128-encrypted.pdf` - AES-128 encryption (V=4, R=4)
- `EC-06-aes256-encrypted.pdf` - AES-256 encryption (V=5, R=6)
- `EC-empty-password.pdf` - Empty password PDF

## Integration Notes

The existing encryption test at `crates/pdftract-cli/tests/test_encryption_errors.rs` currently duplicates some of this functionality with its own implementations of:
- `workspace_root()`
- `encrypted_fixture_dir()`
- `encrypted_fixture()`
- `pdftract_bin()`
- `ENCRYPTION_EXIT_CODE` constant
- `ENCRYPTED_FIXTURES` constant

Future work could refactor that file to use the common fixtures from `pdftract_tests::encryption_fixtures` instead, eliminating duplication.

## References

- Parent bead: bf-2nl4x
- Plan line 258, Failure Mode Taxonomy: ENCRYPTION_UNSUPPORTED
- Related files:
  - `tests/encryption_fixtures.rs`
  - `tests/lib.rs`
  - `tests/ENCRYPTION_FIXTURES.md`
  - `tests/verify_encryption_fixtures.rs`
  - `tests/encryption_fixtures_usage_example.rs`

## Git Status

Files are currently untracked (newly created):
- `tests/encryption_fixtures.rs`
- `tests/lib.rs`
- `tests/ENCRYPTION_FIXTURES.md`
- `tests/verify_encryption_fixtures.rs`
- `tests/encryption_fixtures_usage_example.rs`
- `tests/mod.rs` (modified)

All files compile successfully and are ready for commit.
