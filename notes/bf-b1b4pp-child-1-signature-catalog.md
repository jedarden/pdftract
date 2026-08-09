# Test Function Signature Catalog
**Bead:** bf-b1b4pp-child-1  
**Date:** 2026-08-09  
**Scope:** Integration test suite signature analysis

## Executive Summary

Scanned all integration test files in `tests/` directory. Found **3 helper functions** that are named like test functions but intentionally lack the `#[test]` attribute because they accept parameters. These are **not bugs**—they are helper functions called by actual test functions. No compilation errors exist.

## Catalog by Category

### 1. Helper Functions with Parameters (3 total)

These functions are named like tests (prefix `test_` or contain `fixture`) but lack `#[test]` because they require parameters. They are **correctly implemented** as helper functions.

#### 1.1 `test_fixture(fixture: &Fixture)` 
- **File:** `tests/json_schema.rs:101`
- **Current signature:** `fn test_fixture(fixture: &Fixture)`
- **Category:** Helper function
- **Usage:** Called by `test_all_fixtures_schema_compliance()`, `test_simple_invoice()`, `test_sample()`, and other individual fixture test functions
- **Purpose:** Validates a single fixture against JSON schema
- **Status:** ✅ CORRECT - Helper function should not have `#[test]` attribute

#### 1.2 `test_fixture_pair(name: &str, expected_match: bool)`
- **File:** `tests/fingerprint_reproducibility.rs:189`  
- **Current signature:** `fn test_fixture_pair(name: &str, expected_match: bool)`
- **Category:** Helper function
- **Usage:** Called by `test_acrobat_resave_fixture()`, `test_qpdf_resave_fixture()`, `test_pdftk_resave_fixture()`, `test_linearization_toggle_fixture()`, `test_metadata_only_fixture()`, `test_content_edit_one_glyph_fixture()`, `test_content_edit_one_paragraph_fixture()`, `test_byte_identical_fixture()`
- **Purpose:** Tests a single fixture pair with expected match/differ outcome
- **Status:** ✅ CORRECT - Helper function should not have `#[test]` attribute

#### 1.3 `run_fixture(fixture: &Fixture)`
- **File:** `tests/stream_decoder_fixtures.rs:161`
- **Current signature:** `fn run_fixture(fixture: &Fixture)`  
- **Category:** Helper function
- **Usage:** Called by `test_stream_decoder_fixtures()` which iterates over all fixtures
- **Purpose:** Executes a single stream decoder fixture test
- **Status:** ✅ CORRECT - Helper function should not have `#[test]` attribute

### 2. Data Helper Functions (Correct - No Issues)

These functions return test data and correctly lack `#[test]` attributes:

- `fn fixtures_dir() -> PathBuf` (fingerprint_reproducibility.rs:17)
- `fn fixture_pairs() -> Vec<FixturePair>` (fingerprint_reproducibility.rs:28)
- `fn all_fixtures() -> Vec<Fixture>` (object_parser.rs:31)

### 3. Regular Test Functions (All Correct - No Issues)

Found **195 functions** correctly marked with `#[test]` attribute and proper signatures (no parameters). All test functions follow the standard pattern:
```rust
#[test]
fn test_<description>() {
    // test body
}
```

Sample of correctly implemented test functions:
- `test_fingerprint_fixture_pairs()` (fingerprint_reproducibility.rs:67)
- `test_inv3_reproducibility_100_invocations()` (fingerprint_reproducibility.rs:102)
- `test_all_fixtures_schema_compliance()` (json_schema.rs:163)
- `test_simple_invoice()` (json_schema.rs:175)
- `test_bomb_limit_simple()` (test_bomb_limit.rs:5)
- `test_debug_fingerprints()` (test_fingerprint_debug.rs:5)
- + 189 more

### 4. Async Test Functions (0 found)

No async test functions (`async fn test_`) found in the test suite. All tests use synchronous functions.

## Analysis Results

### No Signature Issues Found

✅ **All 195 test functions** have correct signatures  
✅ **3 helper functions** correctly lack `#[test]` attributes (they have parameters)  
✅ **All data helper functions** correctly return test data without `#[test]`  
✅ **No compilation errors** - `cargo check --tests` passes cleanly  
✅ **No functions marked with `#[test]` have parameters** (would be invalid)

### Key Insight

The 3 helper functions named like test functions (`test_fixture`, `test_fixture_pair`, `run_fixture`) are **not bugs**. They are helper functions that:
1. Accept parameters (fixture data or configuration)
2. Are called by actual `#[test]` functions
3. Correctly lack the `#[test]` attribute because test functions must accept zero parameters
4. Follow the common pattern of extracting reusable test logic into parameterized helpers

## File Scan Coverage

Scanned **44 integration test files**:
- `tests/list_pdf_fixtures.rs`
- `tests/debug_fingerprint_contents.rs`
- `tests/debug_content_hash_integration.rs`
- `tests/debug_lzw.rs`
- `tests/test_import_path.rs`
- `tests/debug_missing_mediabox.rs`
- `tests/debug_span_access.rs`
- `tests/test_assertion_methods.rs`
- `tests/lib.rs`
- `tests/debug_page_count.rs`
- `tests/debug_content_streams.rs`
- `tests/debug_parse_simple.rs`
- `tests/test_parse_fixture.rs`
- `tests/debug_content_fingerprint.rs`
- `tests/debug_fingerprint_content_streams.rs`
- `tests/fixture_discovery.rs`
- `tests/stream_decoder_fixtures.rs`
- `tests/debug_content_fingerprint_fixtures.rs`
- `tests/debug_fingerprint_content_edit.rs`
- `tests/fingerprint_fixtures.rs`
- `tests/debug_fingerprint_issue.rs`
- `tests/forms_integration.rs`
- `tests/debug_fingerprint_fixture_content.rs`
- `tests/test_cases.rs`
- `tests/test_extract_content_stream_bytes.rs`
- `tests/json_schema.rs`
- `tests/debug_fingerprint_content.rs`
- `tests/smoke_test.rs`
- `tests/gen_lexer_golden.rs`
- `tests/debug_a85_filter.rs`
- `tests/log_secret_fuzz.rs`
- `tests/test_page_access.rs`
- `tests/fingerprint_test_single_one.rs`
- `tests/debug_content_edit_fingerprint.rs`
- `tests/debug_parse.rs`
- `tests/test_helpers.rs`
- `tests/mod.rs`
- `tests/encryption_errors.rs`
- `tests/encryption_fixtures.rs`
- `tests/debug_fixtures.rs`
- `tests/object_parser.rs`
- `tests/debug_content_hash.rs`
- `tests/debug_fingerprint_content_hash.rs`
- `tests/debug_filter_array.rs`
- `tests/debug_content_edit_pages.rs`
- `tests/encryption_fixtures_usage_example.rs`
- `tests/fingerprint_reproducibility.rs`
- `tests/test_glob_discovery.rs`
- `tests/debug_content_stream_hash.rs`
- `tests/verify_encryption_fixtures.rs`

## Conclusion

**No test function signature issues were found.** All test functions have correct signatures, and helper functions are properly implemented. The catalog identifies 3 helper functions that could be confused with test functions but are correctly implemented as parameterized helpers.

### Next Steps (if any)

Since no signature issues exist, no fixes are needed. The catalog can be used as:
1. Documentation of helper function patterns in the test suite
2. Reference for understanding test organization
3. Baseline for future test additions to maintain consistency

---

**Verification:** Created catalog by scanning all test files with `grep` for patterns like `^fn test_`, `^async fn test_`, `#\[test\]` markers, and manual inspection of identified functions. `cargo check --tests` passes with no errors.
