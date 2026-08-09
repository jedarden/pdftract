# Structured Compiler Warnings for Test Files

**Generated:** 2026-08-09  
**Bead:** bf-42xx3m  
**Source:** notes/bf-5kjp4b-cargo-check-output.txt

## Summary

- **Total test file warnings:** 108
- **Test files affected:** 50
- **Warning categories:** 5
- **Warning types:** 81 unique types

## Categories Overview

| Category | Count | Percentage |
|----------|-------|------------|
| unused_imports | 56 | 51.9% |
| unused_variables | 33 | 30.6% |
| dead_code | 12 | 11.1% |
| other | 6 | 5.6% |
| unused_doc_comments | 1 | 0.9% |

---

## 1. UNUSED_IMPORTS (56 warnings, 39 types)

### Top Unused Import Types

#### 1.1 unused import: `Path` (10 occurrences)
**Files:**
- `crates/pdftract-cli/tests/cli_invocation_fixtures.rs:19`
- `crates/pdftract-cli/tests/test_legal_filing.rs:19`
- `crates/pdftract-cli/tests/test_encryption_errors.rs:19`
- `crates/pdftract-cli/tests/test_form.rs:19`
- `crates/pdftract-cli/tests/single_page_access.rs:19`
- `crates/pdftract-cli/tests/test_contract.rs:19`
- `crates/pdftract-cli/tests/test_scientific_paper.rs:19`
- `crates/pdftract-cli/tests/test_slide_deck.rs:19`
- `crates/pdftract-cli/tests/test_book_chapter.rs:19`
- `crates/pdftract-cli/tests/multi_output_validation.rs:19`

**Code Snippet:**
```rust
19 | use std::path::{Path, PathBuf};
   |                     ^^^^
```

#### 1.2 unused import: `super::*` (6 occurrences)
**Files:**
- `crates/pdftract-cli/tests/test_legal_filing.rs:584`
- `crates/pdftract-cli/tests/test_book_chapter.rs:584`
- `crates/pdftract-cli/tests/test_encryption_errors.rs:584`
- `crates/pdftract-cli/tests/test_slide_deck.rs:584`
- `crates/pdftract-cli/tests/test_scientific_paper.rs:584`
- `crates/pdftract-cli/tests/test_contract.rs:584`

**Code Snippet:**
```rust
584 |     use super::*;
    |         ^^^^^^^
```

#### 1.3 unused imports: multiple specific imports (various)
**Files:**
- `crates/pdftract-cli/tests/conformance.rs` - `PathBuf` and `Path`
- `crates/pdftract-core/tests/encryption_integration_tests.rs` - Multiple encryption-related imports
- `crates/pdftract-core/tests/test_helpers/mod.rs` - Multiple process guard imports
- `crates/pdftract-cli/tests/test_encryption_unsupported.rs` - Multiple diagnostic imports

---

## 2. UNUSED_VARIABLES (33 warnings, 24 types)

### Top Unused Variable Types

#### 2.1 variable does not need to be mutable (4 occurrences)
**Files:**
- `tests/list_pdf_fixtures.rs`
- `crates/pdftract-core/tests/conformance.rs`
- `crates/pdftract-cli/tests/fixture_discovery.rs`
- `crates/pdftract-cli/tests/multi_output_validation.rs`

**Code Snippet:**
```rust
19 |     let mut fixtures = discover_fixtures_in_dir(fixture_dir)?;
   |         ----^^^^^^^^
   |         |
   |         help: remove this `mut`
```

#### 2.2 unused variable: `fixture_dir` (4 occurrences)
**Files:**
- `crates/pdftract-cli/tests/test_slide_deck.rs`
- `crates/pdftract-cli/tests/test_scientific_paper.rs`
- `crates/pdftract-cli/tests/test_legal_filing.rs`
- `crates/pdftract-cli/tests/test_contract.rs`

**Code Snippet:**
```rust
15 | fn run_legal_filing(fixture_dir: &str) -> Result<()> {
   |                         ^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_fixture_dir`
```

#### 2.3 unused variable: `x0`, `x1` coordinates (2 occurrences each)
**Files:**
- `crates/pdftract-core/src/font/type3_rasterizer_test.rs`

**Code Snippet:**
```rust
8 | use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
   |                                                ^^^^^^^^
```

---

## 3. DEAD_CODE (12 warnings, 11 types)

### Dead Function/Method/Field Warnings

#### 3.1 function `format_validation_error` is never used
**File:** `crates/pdftract-core/tests/json_schema.rs`

#### 3.2 function `count_diagnostics` is never used  
**File:** `crates/pdftract-core/tests/xref_helpers.rs`

#### 3.3 function `assert_diagnostic_count_at_least` is never used
**File:** `crates/pdftract-core/tests/error_recovery_integration.rs`

#### 3.4 function `tesseract_available` is never used
**File:** `crates/pdftract-core/tests/ocr_integration.rs`

#### 3.5 method `with_encrypt_dict` is never used
**File:** `crates/pdftract-core/tests/encryption_integration_tests.rs`

#### 3.6 field `min_schema_version` is never read
**File:** `crates/pdftract-core/tests/conformance.rs`

#### 3.7 variant `Null` is never constructed
**File:** `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs`

#### 3.8 variant `UnsupportedPlatform` is never constructed
**File:** `crates/pdftract-core/tests/memory_guard.rs`

#### 3.9 struct `TestResult` is never constructed
**File:** `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs`

#### 3.10 field `description` is never read (2 occurrences)
**Files:**
- `crates/pdftract-core/tests/encoding_recovery.rs`
- `crates/pdftract-core/tests/error_recovery_integration.rs`

---

## 4. UNUSED_DOC_COMMENTS (1 warning, 1 type)

### Unused Documentation

#### 4.1 unused doc comment
**File:** `crates/pdftract-core/tests/encoding_recovery.rs:50`

**Code Snippet:**
```rust
50 | / /// Per-thread resolution depth counter.
51 | | ///
52 | | /// Each thread gets its own independent depth counter, allowing concurrent
53 | | /// page processing in rayon without lock contention.
   | |_----------------------------------------------------^
   |   |
   |   rustdoc does not generate documentation for macro invocations
```

---

## 5. OTHER WARNINGS (6 warnings, 6 types)

### Miscellaneous Test Warnings

#### 5.1 comparison is useless due to type limits
**File:** `crates/pdftract-cli/tests/single_page_access.rs`

#### 5.2 fields `name` and `description` are never read
**File:** `crates/pdftract-core/tests/cjk_encoding.rs`

#### 5.3 unreachable expression
**File:** `crates/pdftract-cli/tests/profiles_cmd.rs` (multiple occurrences)

---

## Top 10 Files With Most Warnings

1. **`crates/pdftract-cli/tests/test_legal_filing.rs`** - 8 warnings
2. **`crates/pdftract-cli/tests/test_contract.rs`** - 8 warnings  
3. **`crates/pdftract-cli/tests/test_scientific_paper.rs`** - 8 warnings
4. **`crates/pdftract-cli/tests/test_slide_deck.rs`** - 8 warnings
5. **`crates/pdftract-cli/tests/test_book_chapter.rs`** - 8 warnings
6. **`crates/pdftract-cli/tests/test_encryption_errors.rs`** - 7 warnings
7. **`crates/pdftract-cli/tests/conformance.rs`** - 6 warnings
8. **`crates/pdftract-core/tests/encryption_integration_tests.rs`** - 6 warnings
9. **`crates/pdftract-cli/tests/multi_output_validation.rs`** - 5 warnings
10. **`crates/pdftract-cli/tests/test_form.rs`** - 5 warnings

---

## Warning Distribution by Test Directory

| Directory | Warning Count | File Count |
|-----------|---------------|------------|
| `crates/pdftract-cli/tests/` | 52 | 20 |
| `crates/pdftract-core/tests/` | 38 | 25 |
| `tests/` | 8 | 4 |
| `crates/pdftract-core/src/` (test fixtures) | 8 | 3 |
| `crates/pdftract-libpdftract/tests/` | 2 | 1 |

---

## Key Patterns Identified

### High-Impact Patterns

1. **Template test files** (test_contract.rs, test_legal_filing.rs, etc.) show similar warnings:
   - Unused `fixture_dir` parameter
   - Unused `super::*` imports
   - Unused `Path` imports
   - Suggests these are template files that need customization

2. **Common unused imports across test files:**
   - `std::path::Path` (most common)
   - `super::*` (wildcard imports)
   - Test-specific utilities that aren't used in all test scenarios

3. **Unused variables often related to:**
   - Test fixture parameters that are placeholders
   - Error variables that are deliberately ignored in tests
   - Coordinate/destructuring variables in geometric tests

### Recommendations

1. **Clean up template test files** - Remove or document intentional unused parameters
2. **Remove unused imports systematically** - Use `cargo fix` for automatic cleanup
3. **Document intentionally unused variables** - Use `_variable_name` convention
4. **Remove dead test helper functions** - Or document their purpose for future use

---

## Complete Data Export

The complete JSON dataset with all warnings, file paths, line numbers, and code snippets is available at:
`/tmp/structured_warnings.json`

**Total size:** ~50KB JSON data with 108 detailed warning entries

---

## Methodology

1. **Extracted** raw cargo check output from `notes/bf-5kjp4b-cargo-check-output.txt`
2. **Filtered** for test-specific files using regex patterns for:
   - `tests/` directories
   - Files ending in `test.rs`, `_test.rs`
   - Security test harness files (`TH-*.rs`)
   - Test fixture files (`*_fixtures.rs`)
3. **Categorized** warnings into 5 main categories based on warning message patterns
4. **Extracted** file paths, line numbers, and code snippets for each warning
5. **Analyzed** patterns and identified common issues across test files

---

## Next Steps

This structured warning data provides the foundation for:
- Systematic cleanup of test code
- Identification of test template patterns that need adjustment
- Prioritization of warnings by impact and frequency
- Integration with automated fix tools (cargo fix)
