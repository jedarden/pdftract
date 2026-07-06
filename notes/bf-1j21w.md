# Verification Note: bf-1j21w - assert_stderr_contains on ExtractionResult

## Task
Verify that the `assert_stderr_contains` method on `ExtractionResult` is implemented and functional.

## Findings

### Method Does NOT Exist on ExtractionResult

After a thorough search of the codebase:
- `ExtractionResult` is defined in `/home/coding/pdftract/crates/pdftract-core/src/extract.rs` (line 237)
- `ExtractionResult` has **NO impl block** with any assertion methods
- The only associated function is `result_to_json()` (line 1483), which is a standalone function, not a method

### assert_stderr_contains Exists Elsewhere

The `assert_stderr_contains` method **DOES** exist, but on a different type:
- **Type:** `TestExecutionResult`
- **Location:** `/home/coding/pdftract/tests/encryption_fixtures.rs` (line 243)
- **Purpose:** Test helper for CLI command execution results

### Code Comparison

**TestExecutionResult::assert_stderr_contains** (exists):
```rust
impl TestExecutionResult {
    pub fn assert_stderr_contains(&self, text: &str) -> &Self {
        let stderr = self.stderr();
        assert!(
            stderr.contains(text),
            "Expected stderr to contain '{}', got: {}",
            text,
            stderr
        );
        self
    }
}
```

**ExtractionResult** (no such method):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub fingerprint: String,
    pub pages: Vec<PageResult>,
    pub metadata: ExtractionMetadata,
    pub signatures: Vec<SignatureJson>,
    pub form_fields: Vec<FormFieldJson>,
    pub links: Vec<LinkJson>,
    pub attachments: Vec<AttachmentJson>,
    pub threads: Vec<ThreadJson>,
    pub javascript_actions: Vec<JavascriptActionJson>,
}
```

## Conclusion

The `assert_stderr_contains` method does **NOT** exist on `ExtractionResult`. The task asks to verify a method that has not been implemented.

### Possible Explanations

1. **Typo in task description:** The task may have meant `TestExecutionResult` instead of `ExtractionResult`
2. **Missing implementation:** The method was planned but never implemented on `ExtractionResult`
3. **Wrong bead:** This may be testing the wrong type

### Why It Doesn't Make Sense for ExtractionResult

Looking at the structure:
- `ExtractionResult` is the **output** of PDF extraction (JSON serializable data)
- It has **no stderr field** - it's not a command execution result
- It contains: pages, metadata, signatures, form_fields, links, attachments, threads, javascript_actions
- stderr/stdout are properties of **process execution**, not extraction results

The `TestExecutionResult` type (which DOES have `assert_stderr_contains`) wraps `std::process::Output` and is used for testing CLI commands that produce stderr.

## Recommendation

**DO NOT CLOSE** - This bead describes verification work for a method that doesn't exist. One of:

1. Update task to verify `TestExecutionResult::assert_stderr_contains` instead
2. Implement the method if it's actually needed (though it doesn't make semantic sense)
3. Mark bead as superseded/invalid if the task was based on incorrect assumptions

## Verification Commands Run

```bash
# Search for assert_stderr_contains in codebase
grep -rn "assert_stderr_contains" /home/coding/pdftract --include="*.rs"
# Results: Only found in tests/encryption_fixtures.rs on TestExecutionResult

# Search for ExtractionResult impl blocks
grep -n "impl ExtractionResult" /home/coding/pdftract/crates/pdftract-core/src/extract.rs
# Results: None found

# Search for any assert_stderr on ExtractionResult
grep -rn "assert_stderr" /home/coding/pdftract/crates --include="*.rs"
# Results: None found
```

## Status

**FAIL** - Method does not exist on the specified type.
