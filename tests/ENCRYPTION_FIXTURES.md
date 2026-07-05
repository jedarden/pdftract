# Encryption Test Fixtures and Helpers

This directory contains common test fixtures and helper functions for encryption testing in pdftract.

## Overview

The `encryption_fixtures.rs` module provides shared utilities for encryption-related tests across the pdftract codebase.

## Features

- **Path resolution**: Functions to locate binaries and test fixtures
- **Test execution**: Helpers for running pdftract with various configurations
- **Assertion helpers**: Functions to validate test outcomes
- **Mock data builders**: Functions to create mock encryption dictionaries
- **Test constants**: Common constants for encryption testing

## Usage

### Basic Imports

```rust
use pdftract_tests::encryption_fixtures::*;
```

### Running Tests

```rust
#[test]
fn test_encryption_scenario() {
    let bin = pdftract_bin();
    let fixture = encrypted_fixture("livecycle.pdf");
    
    let output = run_pdftract_extract(&bin, &fixture, None);
    assert_encryption_exit_code(&output, &fixture);
}
```

### Available Functions

#### Path Resolution

- `workspace_root()` - Get workspace root directory
- `pdftract_bin()` - Get path to pdftract binary
- `encrypted_fixtures_dir()` - Get encrypted fixtures directory
- `encrypted_fixture(name)` - Get specific encrypted fixture
- `fixtures_dir()` - Get general fixtures directory
- `fixture(name)` - Get general fixture

#### Test Execution

- `run_pdftract_extract(bin, pdf_path, password)` - Run extract with optional password
- `run_pdftract_extract_with_stdin_password(bin, pdf_path, password)` - Run with stdin password

#### Assertions

- `assert_encryption_diagnostic(output, fixture_name)` - Check for encryption diagnostics
- `assert_encryption_exit_code(output, fixture_name)` - Verify exit code 3
- `assert_success_exit_code(output)` - Verify success
- `assert_failure_exit_code(output)` - Verify failure
- `assert_output_contains(output, message)` - Check output contains message

#### Fixture Validation

- `assert_fixture_exists(fixture_path)` - Verify fixture exists
- `assert_valid_pdf_structure(fixture_path)` - Verify PDF structure
- `assert_encrypted_fixtures_exist()` - Verify all fixtures exist

#### Mock Data Builders (with `decrypt` feature)

- `make_dict(entries)` - Create PdfDict from entries
- `make_trailer(encrypt_dict, id)` - Create trailer with encryption
- `make_rc4_encryption_dict()` - Create RC4-40 encryption dict
- `make_aes128_encryption_dict()` - Create AES-128 dict
- `make_aes256_encryption_dict()` - Create AES-256 dict
- `make_unsupported_encryption_dict(filter_name)` - Create unsupported dict

## Constants

- `EXPECTED_ENCRYPTION_EXIT_CODE` - Expected exit code (3)
- `ENCRYPTED_FIXTURES` - List of encrypted fixture names
- `TEST_PASSWORD` - Test password for fixtures
- `WRONG_PASSWORD` - Wrong password for error testing

## Encrypted Fixtures

The following encrypted PDF fixtures are available:

- `livecycle.pdf` - Adobe LiveCycle unsupported encryption
- `EC-04-rc4-encrypted.pdf` - RC4-40 encryption (V=1, R=2)
- `EC-05-aes128-encrypted.pdf` - AES-128 encryption (V=4, R=4)
- `EC-06-aes256-encrypted.pdf` - AES-256 encryption (V=5, R=6)
- `EC-empty-password.pdf` - Empty password PDF

## Examples

### Test Basic Encryption Detection

```rust
#[test]
fn test_livecycle_unsupported() {
    let bin = pdftract_bin();
    let fixture = encrypted_fixture("livecycle.pdf");
    
    let output = run_pdftract_extract(&bin, &fixture, None);
    assert_encryption_exit_code(&output, &fixture);
    assert_encryption_diagnostic(&output, &fixture);
}
```

### Test Password Handling

```rust
#[test]
fn test_password_handling() {
    let bin = pdftract_bin();
    let fixture = encrypted_fixture("EC-04-rc4-encrypted.pdf");
    
    // Test with correct password
    let output = run_pdftract_extract(&bin, &fixture, Some(TEST_PASSWORD));
    assert_success_exit_code(&output);
    
    // Test with wrong password
    let output = run_pdftract_extract(&bin, &fixture, Some(WRONG_PASSWORD));
    assert_encryption_exit_code(&output, &fixture);
}
```

### Mock Data for Core Tests

```rust
#[cfg(feature = "decrypt")]
#[test]
fn test_encryption_detection() {
    let encrypt_dict = make_rc4_encryption_dict();
    let trailer = make_trailer(encrypt_dict, Some(vec![0u8; 16]));
    
    // Test detection logic...
}
```

## Integration

The fixtures module is automatically available to all integration tests in the pdftract workspace. Simply import it:

```rust
use pdftract_tests::encryption_fixtures::*;
```

## Maintenance

When adding new encrypted fixtures:
1. Add the PDF to `tests/fixtures/encrypted/`
2. Add the filename to `ENCRYPTED_FIXTURES` constant
3. Document any special characteristics

## Related Files

- `tests/encryption_errors.rs` - Main encryption error tests
- `crates/pdftract-cli/tests/test_encryption_errors.rs` - CLI-specific tests
- `crates/pdftract-core/tests/encryption_integration_tests.rs` - Core encryption tests
