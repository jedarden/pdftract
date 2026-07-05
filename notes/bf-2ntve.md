# bf-2ntve: Encryption Test Module Location Research

## Summary

Identified the appropriate test module locations for encryption-related error tests in the pdftract CLI.

## Findings

### Existing Test Structure

The pdftract project uses a dual-level test structure:

1. **Workspace-level integration tests**: `/home/coding/pdftract/tests/`
   - Contains general integration tests shared across the workspace
   - Example: `tests/encryption_errors.rs` (7,018 bytes)

2. **CLI-specific integration tests**: `/home/coding/pdftract/crates/pdftract-cli/tests/`
   - Contains tests specific to the pdftract CLI binary
   - More extensive test fixtures and CLI-specific behavior

### Encryption Test Files Already Present

#### 1. CLI-specific encryption tests

**File**: `/home/coding/pdftract/crates/pdftract-cli/tests/test_encryption_errors.rs` (12,690 bytes)
- **Purpose**: Comprehensive encryption error testing for pdftract CLI
- **Structure**: Well-organized module with:
  - `unsupported_handlers` module - Tests for Adobe LiveCycle unsupported handler
  - `supported_encryption` module - Tests for RC4/AES decryption with passwords
  - `password_errors` module - Tests for wrong/missing password handling
  - `fixture_validation` module - Validates encrypted PDF fixtures
  - `integration_tests` module - Placeholder for complete workflow tests
- **Fixtures**: Uses `tests/fixtures/encrypted/` directory
- **References**: Cites plan lines 258, 732, 1132, 1149

**File**: `/home/coding/pdftract/crates/pdftract-cli/tests/test_encryption_unsupported.rs` (2,828 bytes)
- **Purpose**: Focused test for ENCRYPTION_UNSUPPORTED exit code
- **Tests**: Two specific tests for livecycle.pdf behavior
- **Functionality**: Verifies exit code 3 and error message content

#### 2. Workspace-level encryption tests

**File**: `/home/coding/pdftract/tests/encryption_errors.rs` (7,018 bytes)
- **Purpose**: Encryption error integration tests
- **Feature flag**: `#![cfg(feature = "decrypt")]`
- **Tests**: 
  - `test_encryption_unsupported_livecycle()` - Verifies ENCRYPTION_UNSUPPORTED diagnostic
  - `test_exit_code_3_no_password()` - Exit code verification
  - `test_wrong_password_encryption_unsupported()` - Wrong password handling
  - `test_encryption_error_consistency()` - Parameterized test across fixtures

## Target File Paths for New Encryption Tests

### For CLI-specific encryption tests
```
/home/coding/pdftract/crates/pdftract-cli/tests/test_encryption_errors.rs
```
**Rationale**: This is the primary location for CLI-specific encryption error tests. It already contains:
- Comprehensive module structure
- Fixture validation infrastructure
- Password handling tests
- Exit code verification

### For general integration tests
```
/home/coding/pdftract/tests/encryption_errors.rs
```
**Rationale**: This location is appropriate for:
- Cross-cutting integration tests
- Tests that don't require CLI-specific behavior
- Tests gated by feature flags (e.g., `decrypt` feature)

## Module Structure Pattern

The test structure follows these conventions:

1. **CLI tests**: `crates/pdftract-cli/tests/test_<feature>_<subcategory>.rs`
   - Examples: `test_encryption_errors.rs`, `test_encryption_unsupported.rs`
   - Uses direct binary execution via `Command::new(pdftract_bin())`

2. **Workspace tests**: `tests/<feature>_<subcategory>.rs`
   - Examples: `encryption_errors.rs`
   - May use feature gates: `#![cfg(feature = "decrypt")]`

## Encrypted Fixtures Location

```
/home/coding/pdftract/tests/fixtures/encrypted/
```

**Fixture files**:
- `EC-04-rc4-encrypted.pdf` - RC4 encryption
- `EC-05-aes128-encrypted.pdf` - AES-128 encryption
- `EC-06-aes256-encrypted.pdf` - AES-256 encryption
- `EC-empty-password.pdf` - Empty password
- `livecycle.pdf` - Unsupported Adobe.APS handler

## References

- Plan line 258: Failure Mode Taxonomy - ENCRYPTION_UNSUPPORTED
- Plan line 732: Input | Encryption-unsupported diagnostic and CLI exit code 3
- Plan line 1132: RC4 and AES-128/256 decryption implementation
- Plan line 1149: Encrypted file with unknown handler error handling
- Parent bead: bf-56jda

## Recommendation

For new encryption-related error tests, the appropriate location is:

**Primary location**: `/home/coding/pdftract/crates/pdftract-cli/tests/test_encryption_errors.rs`

This file already contains:
- Module structure for different encryption scenarios
- Fixture validation infrastructure
- Exit code verification patterns
- Password handling tests (some marked as `[ignore]` pending implementation)

**Secondary location** (for feature-flagged or general integration tests):
`/home/coding/pdftract/tests/encryption_errors.rs`

## Acceptance Criteria Status

✅ **Target test file path identified**: 
- Primary: `crates/pdftract-cli/tests/test_encryption_errors.rs`
- Secondary: `tests/encryption_errors.rs`

✅ **Module structure understood**:
- CLI tests use `test_<feature>_<subcategory>.rs` naming
- Modular structure with sub-modules for different scenarios
- Fixture-based testing with validation

✅ **Path documented in verification note**: This file documents all findings
