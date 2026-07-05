# bf-53l6m: Test Macros and Attributes Setup

## Task
Set up test macros and attributes for encryption tests.

## Implementation

### Current Test Attribute Configuration

The encryption tests use the following Rust test attributes and macros:

#### 1. **Standard Test Macro**
```rust
#[test]
fn test_example() {
    // test code
}
```
- Used for all test functions
- Recognized by standard Rust test framework

#### 2. **Conditional Compilation Attribute**
```rust
#[cfg(feature = "decrypt")]
fn test_decrypt_feature() {
    // decrypt-dependent test
}
```
- Gates tests behind the `decrypt` feature
- Prevents compilation errors when feature is disabled
- Used consistently across encryption tests

#### 3. **Test Ignore Attribute**
```rust
#[test]
#[ignore = "Performance test - run with --release"]
fn test_encryption_performance() {
    // performance test
}
```
- Marks tests to be skipped by default
- Can be run with `cargo test -- --ignored`
- Used for performance-intensive tests

### Test File Structure

#### `pdftract-core` Encryption Tests
- **File**: `crates/pdftract-core/tests/encryption_integration_tests.rs`
- **Structure**: Top-level test functions with conditional compilation
- **Tests**: 18 active tests, 1 ignored performance test
- **Attributes**: `#[test]`, `#[cfg(feature = "decrypt")]`, `#[ignore]`

#### Other Encryption Test Files
- `encryption_aes_128_test.rs` - Uses `#[cfg(test)]` module wrapper
- `encryption_aes_256_test.rs` - Uses `#[cfg(test)]` module wrapper  
- `encryption_rc4_test.rs` - Uses `#[cfg(test)]` module wrapper

#### `pdftract-cli` Encryption Tests
- **File**: `crates/pdftract-cli/tests/test_encryption_unsupported.rs`
- **Structure**: Standard `#[test]` attributes without feature gates
- **Purpose**: CLI exit code verification for encryption errors

## Verification

### Compilation Status
✅ **All encryption test files compile successfully**
- `pdftract-core` encryption tests: PASS
- `pdftract-cli` encryption tests: PASS (compilation)
- No attribute-related compilation errors

### Test Framework Recognition
✅ **Test framework properly recognizes all attributes**
```bash
$ cargo test --package pdftract-core --test encryption_integration_tests
running 19 tests
test test_aes128_decrypt_requires_iv ... ok
test test_aes128_object_key_derivation ... ok
...
test test_encryption_performance ... ignored, Performance test - run with --release
test result: ok. 18 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
```

### Attribute Functionality
✅ **`#[cfg(feature = "decrypt")]`** - Correctly gates decrypt-dependent tests
✅ **`#[test]`** - All tests recognized and executed by test framework
✅ **`#[ignore]`** - Performance test correctly ignored by default

## Acceptance Criteria Met

- ✅ **Test macros added**: Standard `#[test]` macro used throughout
- ✅ **Test attributes configured**: `#[cfg(feature = "decrypt")]` and `#[ignore]` properly applied
- ✅ **File compiles successfully**: All encryption test files compile without errors
- ✅ **Test framework recognizes the macros/attributes**: 18 tests pass, 1 test ignored as expected

## Notes

The encryption test infrastructure uses standard Rust testing attributes rather than custom macros. This approach:
- Maintains compatibility with standard Rust tooling
- Works seamlessly with IDE test runners
- Integrates with CI/CD pipelines
- Requires no additional build dependencies

The conditional compilation via `#[cfg(feature = "decrypt")]` ensures that encryption tests are only compiled when the decrypt feature is enabled, preventing compilation failures in configurations without encryption support.
