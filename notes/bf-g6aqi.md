# Bead bf-g6aqi: Locate and examine assert_exit_code method implementation

## Summary

Located and examined the `assert_exit_code` method in the pdftract codebase.

## Findings

### 1. Method EXISTS on `TestExecutionResult` struct

**Location:** `/home/coding/pdftract/tests/encryption_fixtures.rs` (lines 267-279)

**Method signature:**
```rust
pub fn assert_exit_code(&self, expected: i32) -> &Self
```

**Implementation:**
```rust
pub fn assert_exit_code(&self, expected: i32) -> &Self {
    let actual = self.exit_code();
    let context = self.fixture_name.as_deref().unwrap_or("command");
    assert_eq!(
        actual,
        Some(expected),
        "Expected {} to exit with code {}, got {:?}",
        context,
        expected,
        actual
    );
    self
}
```

**Behavior:**
- Asserts that the exit code matches an expected value
- Returns `&Self` for method chaining
- Provides clear error messages with context (fixture name or "command")
- Handles cases where process was terminated by signal (returns `None` for exit code)

**Usage pattern:**
```rust
result.assert_exit_code(3);  // Exits with code 3 for encryption errors
result.assert_exit_code(0);  // Exits with code 0 for success
```

### 2. Method MISSING from `ExtractionResult` struct

**Location:** `/home/coding/pdftract/crates/pdftract-core/src/extract.rs` (line 237)

**Struct definition:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ExtractionResult {
    pub fingerprint: String,
    pub pages: Vec<PageResult>,
    pub metadata: ExtractionMetadata,
    pub signatures: Vec<SignatureJson>,
    pub form_fields: Vec<FormFieldJson>,
    pub links: Vec<LinkJson>,
    pub attachments: Vec<AttachmentJson>,
    pub threads: Vec<ThreadJson>,
    #[serde(default)]
    pub javascript_actions: Vec<JavascriptActionJson>,
}
```

**Status:** NO impl block exists for `ExtractionResult` - the `assert_exit_code` method is NOT implemented on this struct.

### 3. Other `ExtractionResult` in CER diff tool

**Location:** `/home/coding/pdftract/crates/pdftract-cer-diff/src/main.rs` (lines 13-16)

**Purpose:** Simple struct for JSON deserialization in CER diff tool

**Status:** No impl block, no `assert_exit_code` method (not needed for this use case)

## Test Coverage

The `TestExecutionResult::assert_exit_code` method has comprehensive test coverage:

**Passing test (line 762-773):**
- `test_execution_result_assert_exit_code` - Verifies method panics when expected code doesn't match

**Failing test (line 775-788):**
- `test_execution_result_assert_exit_code_failure` - Verifies correct panic behavior with expected message

**Method chaining test (line 979-996):**
- `test_execution_result_method_chaining` - Demonstrates fluent API usage

## Acceptance Criteria Status

- ✅ Location of assert_exit_code method identified
- ✅ Method signature documented
- ✅ Current implementation state known (exists on TestExecutionResult, missing from ExtractionResult)
- ✅ ExtractionResult structure understood

## Conclusion

The `assert_exit_code` method exists on `TestExecutionResult` (a test helper struct for CLI command execution) but does NOT exist on `ExtractionResult` (the core PDF extraction result struct). This is the expected state, as `ExtractionResult` is a data-carrying struct for PDF extraction output and does not need test assertion methods.
