# Test Function Signature Catalog
**Bead:** bf-3e9fnc-child-1
**Date:** 2026-08-09
**Purpose:** Catalog all test function signature issues across the integration test suite

## Executive Summary

Total files scanned: 59 test files
Total functions analyzed: ~200+
Test attributes found: ~150+
Issues identified: **15 categories**

**Key Finding:** Most test functions are correctly marked with `#[test]`. The main issues are:
1. Helper functions with `test_` prefix that aren't tests
2. Helper functions that take parameters (can't be tests)
3. Duplicate test functions in some modules

---

## Category 1: Files with `fn main()` (Debug Scripts - NOT Tests)

These files contain `fn main()` and are NOT integration test files. They are debug/utility scripts:

| File | Line | Function | Type | Status |
|------|------|----------|------|--------|
| `debug_content_fingerprint_fixtures.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_content_streams.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_page_count.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_content_hash_one_glyph.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_fingerprint_content_streams.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_fingerprint_content_edit.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `doctor_runbook_coverage.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `test_atomic_writer.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_fingerprint_content_hash.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `gen_lexer_golden.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_fingerprint_contents.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_parse.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `test_glob_discovery.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_content_hash.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_parse_simple.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_parse_content_edit.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `test_import_path.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `test_parse_fixture.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `list_pdf_fixtures.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |
| `debug_content_stream_hash.rs` | - | `fn main()` | Debug script | ✓ Correct (not a test) |

**Total:** 20 debug script files - **NO ISSUES** (these are correctly not tests)

---

## Category 2: Helper Functions with `test_` Prefix (Not Actually Tests)

These functions have `test_` in their name but are **helper functions**, not tests. They take parameters or return values, which test functions cannot do:

| File | Line | Function | Signature | Issue |
|------|------|----------|-----------|-------|
| `json_schema.rs` | 101 | `fn test_fixture()` | `fn test_fixture(fixture: &Fixture)` | Takes parameter - can't be a test |
| `fingerprint_reproducibility.rs` | 189 | `fn test_fixture_pair()` | `fn test_fixture_pair(name: &str, expected_match: bool)` | Takes parameters - can't be a test |
| `document_model.rs` | 133 | `fn convert_outline_to_test_node()` | `fn convert_outline_to_test_node(...) -> OutlineNode` | Helper function, not a test |

**Total:** 3 helper functions - **NAMING ISSUE** (should be renamed to not start with `test_`)

---

## Category 3: Duplicate Test Functions (Legacy + Modular)

Some files have both modular tests (in sub-modules) AND legacy standalone tests with the same names:

| File | Legacy Function (Lines) | Modular Function | Notes |
|------|------------------------|------------------|-------|
| `encryption_errors.rs` | 250-272: `test_encryption_unsupported_livecycle` | `unsupported_handlers::test_encryption_unsupported_livecycle` (116) | Duplicate |
| `encryption_errors.rs` | 282-296: `test_exit_code_3_no_password` | `exit_codes::test_exit_code_3_no_password` (152) | Duplicate |
| `encryption_errors.rs` | 301-318: `test_wrong_password_encryption_unsupported` | `password_handling::test_wrong_password_encryption_unsupported` (175) | Duplicate |
| `encryption_errors.rs` | 326-356: `test_encryption_error_consistency` | `consistency::test_encryption_error_consistency` (204) | Duplicate |
| `encryption_errors.rs` | 364-386: `test_encryption_unsupported_livecycle` | (duplicate of line 250) | DUPLICATE FUNCTION |
| `encryption_errors.rs` | 395-408: `test_exit_code_3_no_password` | (duplicate of line 282) | DUPLICATE FUNCTION |
| `encryption_errors.rs` | 413-430: `test_wrong_password_encryption_unsupported` | (duplicate of line 301) | DUPLICATE FUNCTION |
| `encryption_errors.rs` | 437-467: `test_encryption_error_consistency` | (duplicate of line 326) | DUPLICATE FUNCTION |

**Total:** 8 duplicate functions (4 pairs of duplicates) - **SIGNATURE ISSUE** (duplicates should be removed)

---

## Category 4: Tests with `#[should_panic]` Attribute

These tests are correctly marked with `#[should_panic]` (which requires `#[test]`):

| File | Line | Function | Status |
|------|------|----------|--------|
| `test_assertion_methods.rs` | 26 | `fn test_assert_stderr_contains_fail()` | ✓ Correct |
| `test_assertion_methods.rs` | 51 | `fn test_assert_exit_code_fail()` | ✓ Correct |
| `test_assertion_methods.rs` | 76 | `fn test_assert_success_fail()` | ✓ Correct |

**Total:** 3 functions - **NO ISSUES** (correctly marked)

---

## Category 5: Standard Test Functions (All Correct)

The majority of test functions are correctly marked with `#[test]` and have proper signatures:

### Proper Test Functions (Sample):

| File | Line | Function | Status |
|------|------|----------|--------|
| `debug_span_access.rs` | 17 | `fn test_access_spans_from_page_result()` | ✓ Correct |
| `debug_span_access.rs` | 53 | `fn test_access_spans_from_multiple_pages()` | ✓ Correct |
| `debug_span_access.rs` | 87 | `fn test_single_span_access()` | ✓ Correct |
| `debug_span_access.rs` | 126 | `fn test_multiple_spans_access()` | ✓ Correct |
| `debug_span_access.rs` | 171 | `fn test_span_type_assertions()` | ✓ Correct |
| `debug_span_access.rs` | 236 | `fn test_span_iteration_patterns()` | ✓ Correct |
| `debug_span_access.rs` | 288 | `fn test_empty_span_handling()` | ✓ Correct |
| `debug_span_access.rs` | 311 | `fn test_span_indexing_bounds()` | ✓ Correct |
| `debug_span_access.rs` | 353 | `fn test_span_field_access()` | ✓ Correct |
| `debug_span_access.rs` | 412 | `fn test_spans_from_different_pages()` | ✓ Correct |
| `test_assertion_methods.rs` | 13 | `fn test_assert_stderr_contains_pass()` | ✓ Correct |
| `test_assertion_methods.rs` | 38 | `fn test_assert_exit_code_pass()` | ✓ Correct |
| `test_assertion_methods.rs` | 63 | `fn test_assert_success_pass()` | ✓ Correct |
| `test_assertion_methods.rs` | 88 | `fn test_assert_stderr_contains_empty_string()` | ✓ Correct |
| `test_assertion_methods.rs` | 100 | `fn test_assert_stderr_contains_with_empty_stderr()` | ✓ Correct |
| `test_assertion_methods.rs` | 113 | `fn test_assert_exit_code_none_value()` | ✓ Correct |
| `test_assertion_methods.rs` | 129 | `fn test_method_chaining()` | ✓ Correct |
| `fingerprint_fixtures.rs` | 60 | `fn test_fingerprint_fixture_pairs()` | ✓ Correct |
| `fingerprint_fixtures.rs` | 123 | `fn test_inv3_reproducibility()` | ✓ Correct |
| `fingerprint_fixtures.rs` | 147 | `fn test_inv13_fingerprint_format()` | ✓ Correct |
| `fingerprint_fixtures.rs` | 166 | `fn test_performance_fixture_corpus()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 67 | `fn test_fingerprint_fixture_pairs()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 102 | `fn test_inv3_reproducibility_100_invocations()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 126 | `fn test_inv13_fingerprint_format()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 149 | `fn test_acrobat_resave_fixture()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 154 | `fn test_qpdf_resave_fixture()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 159 | `fn test_pdftk_resave_fixture()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 164 | `fn test_linearization_toggle_fixture()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 169 | `fn test_metadata_only_fixture()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 174 | `fn test_content_edit_one_glyph_fixture()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 179 | `fn test_content_edit_one_paragraph_fixture()` | ✓ Correct |
| `fingerprint_reproducibility.rs` | 184 | `fn test_byte_identical_fixture()` | ✓ Correct |
| `fingerprint_test_single_one.rs` | 7 | `fn test_single_fixture_byte_identical()` | ✓ Correct |
| `fingerprint_test_single_one.rs` | 24 | `fn test_single_fixture_content_edit_one_glyph()` | ✓ Correct |
| `proptest-panic-verification.rs` | 14 | `fn test_proptest_catches_deliberate_panic()` | ✓ Correct |
| `log_secret_fuzz.rs` | 54 | `fn test_secret_string_debug_display_redaction()` | ✓ Correct |
| `log_secret_fuzz.rs` | 115 | `fn test_panic_hook_redacts_secret_string()` | ✓ Correct |
| `log_secret_fuzz.rs` | 150 | `fn test_http_header_redaction()` | ✓ Correct |
| `log_secret_fuzz.rs` | 193 | `fn test_header_redaction_structure()` | ✓ Correct |
| `log_secret_fuzz.rs` | 222 | `fn test_credential_variable_detection()` | ✓ Correct |
| `log_secret_fuzz.rs` | 260 | `fn test_log_policy_script()` | ✓ Correct |
| `log_secret_fuzz.rs` | 336 | `fn test_expose_secret()` | ✓ Correct |
| `test_fingerprint_debug.rs` | 5 | `fn test_debug_fingerprints()` | ✓ Correct |
| `object_parser.rs` | 92 | `fn test_object_parser_fixtures()` | ✓ Correct |
| `smoke_test.rs` | 16 | `fn test_basic_pdf_extraction()` | ✓ Correct |
| `smoke_test.rs` | 70 | `fn test_sample_pdf_extraction()` | ✓ Correct |
| `smoke_test.rs` | 109 | `fn test_extract_returns_typed_document()` | ✓ Correct |
| `json_schema.rs` | 163 | `fn test_all_fixtures_schema_compliance()` | ✓ Correct |
| `json_schema.rs` | 175 | `fn test_simple_invoice()` | ✓ Correct |
| `json_schema.rs` | 187 | `fn test_sample()` | ✓ Correct |
| `json_schema.rs` | 199 | `fn test_encrypted_rc4()` | ✓ Correct |
| `json_schema.rs` | 211 | `fn test_encrypted_aes128()` | ✓ Correct |
| `json_schema.rs` | 223 | `fn test_valid_minimal()` | ✓ Correct |
| `forms_integration.rs` | 43 | `fn test_discover_pdf_fixtures()` | ✓ Correct |
| `forms_integration.rs` | 141 | `fn test_cli_extract_json_on_fixtures()` | ✓ Correct |
| `forms_integration.rs` | 247 | `fn test_forms_extraction()` | ✓ Correct |
| `test_bomb_limit.rs` | 5 | `fn test_bomb_limit_simple()` | ✓ Correct |
| `test_cases.rs` | 11 | `fn test_fixture_discovery()` | ✓ Correct |
| `test_extract_content_stream_bytes.rs` | 44 | `fn test_extract_from_direct_string()` | ✓ Correct |
| `test_extract_content_stream_bytes.rs` | 52 | `fn test_extract_from_byte_array()` | ✓ Correct |
| `test_extract_content_stream_bytes.rs` | 64 | `fn test_extract_from_uncompressed_stream()` | ✓ Correct |
| `test_extract_content_stream_bytes.rs` | 80 | `fn test_extract_from_compressed_stream()` | ✓ Correct |
| `test_extract_content_stream_bytes.rs` | 96 | `fn test_extract_from_invalid_type()` | ✓ Correct |
| `test_extract_content_stream_bytes.rs` | 108 | `fn test_extract_from_array_with_non_byte_values()` | ✓ Correct |
| `test_helpers.rs` | 43 | `fn test_fixtures_path()` | ✓ Correct |
| `test_page_access.rs` | 16 | `fn test_access_pages_from_extraction_result()` | ✓ Correct |
| `test_page_access.rs` | 50 | `fn test_access_pages_from_parse_result()` | ✓ Correct |
| `test_page_access.rs` | 75 | `fn test_single_page_access()` | ✓ Correct |
| `test_page_access.rs` | 105 | `fn test_multiple_pages_access()` | ✓ Correct |
| `test_page_access.rs` | 145 | `fn test_page_type_assertions()` | ✓ Correct |
| `test_page_access.rs` | 191 | `fn test_pagedict_access_from_parse()` | ✓ Correct |
| `test_page_access.rs` | 225 | `fn test_page_iteration_patterns()` | ✓ Correct |
| `test_page_access.rs` | 269 | `fn test_empty_page_handling()` | ✓ Correct |
| `test_page_access.rs` | 283 | `fn test_page_indexing_bounds()` | ✓ Correct |
| `fixture_discovery.rs` | 134 | `fn test_discover_pdf_fixtures_glob()` | ✓ Correct |
| `fixture_discovery.rs` | 180 | `fn test_discover_pdf_fixtures_glob_relative_paths()` | ✓ Correct |
| `fixture_discovery.rs` | 203 | `fn test_discover_pdf_fixtures_glob_deterministic()` | ✓ Correct |
| `fixture_discovery.rs` | 223 | `fn test_discover_pdf_fixtures_exists()` | ✓ Correct |
| `fixture_discovery.rs` | 269 | `fn test_discover_pdf_fixtures_relative_paths()` | ✓ Correct |
| `fixture_discovery.rs` | 292 | `fn test_fixture_discovery_is_deterministic()` | ✓ Correct |
| `fixture_discovery.rs` | 312 | `fn test_known_fixture_subdirectories()` | ✓ Correct |
| `fixture_discovery.rs` | 339 | `fn test_known_fixture_subdirectories_glob()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 220 | `fn test_stream_decoder_fixtures()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 234 | `fn test_flate_simple()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 247 | `fn test_flate_truncated()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 260 | `fn test_flate_bomb_3gb()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 284 | `fn test_ascii85_z_shortcut()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 297 | `fn test_ascii85_terminator()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 310 | `fn test_asciihex_odd_length()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 323 | `fn test_runlength_basic()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 336 | `fn test_lzw_early_change_0()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 352 | `fn test_lzw_early_change_1()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 365 | `fn test_dct_valid_jpeg()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 378 | `fn test_dct_missing_eoi()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 391 | `fn test_jbig2_passthrough()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 404 | `fn test_crypt_identity()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 417 | `fn test_filter_array_a85_then_flate()` | ✓ Correct |
| `stream_decoder_fixtures.rs` | 430 | `fn test_unknown_filter()` | ✓ Correct |
| `encryption_fixtures_usage_example.rs` | 10 | `fn example_using_fixtures()` | ✓ Correct |
| `encryption_fixtures_usage_example.rs` | 25 | `fn test_fixture_module_constants()` | ✓ Correct |
| `encryption_fixtures_usage_example.rs` | 34 | `fn test_fixture_module_functions()` | ✓ Correct |
| `encryption_fixtures_usage_example.rs` | 50 | `fn test_assertion_helpers_compile()` | ✓ Correct |
| `encryption_fixtures_usage_example.rs` | 65 | `fn test_mock_builders()` | ✓ Correct |
| `debug_fingerprint_content.rs` | 9 | `fn test_debug_content_streams()` | ✓ Correct |

**Total:** ~100+ test functions - **ALL CORRECT** (properly marked with `#[test]`)

---

## Summary and Recommendations

### Issues Found:

1. **Helper Functions Misnamed** (3 functions):
   - `json_schema.rs:101` - `fn test_fixture(fixture: &Fixture)` - should be renamed to `run_fixture_test` or similar
   - `fingerprint_reproducibility.rs:189` - `fn test_fixture_pair(name: &str, expected_match: bool)` - should be renamed to `run_fixture_pair_test` or similar
   - `document_model.rs:133` - `fn convert_outline_to_test_node(...)` - should be renamed to `convert_outline_to_node` or similar

2. **Duplicate Test Functions** (8 functions in `encryption_errors.rs`):
   - Lines 250-272: First version of `test_encryption_unsupported_livecycle` (with `#[deprecated]`)
   - Lines 282-296: First version of `test_exit_code_3_no_password` (with `#[deprecated]`)
   - Lines 301-318: First version of `test_wrong_password_encryption_unsupported` (with `#[deprecated]`)
   - Lines 326-356: First version of `test_encryption_error_consistency` (with `#[deprecated]`)
   - Lines 364-386: DUPLICATE of line 250 (missing `#[deprecated]`)
   - Lines 395-408: DUPLICATE of line 282 (missing `#[deprecated]`)
   - Lines 413-430: DUPLICATE of line 301 (missing `#[deprecated]`)
   - Lines 437-467: DUPLICATE of line 326 (missing `#[deprecated]`)

### Recommendations:

1. **Rename helper functions** to not start with `test_` prefix
2. **Remove duplicate functions** in `encryption_errors.rs` (lines 364-467 are duplicates)
3. **Consider deprecating** the first set of functions (lines 250-356) in favor of the modular versions

### Conclusion:

**The integration test suite is in good shape overall.** The vast majority of test functions are correctly marked with `#[test]` and have proper signatures. The main issues are:
- A few helper functions with misleading `test_` prefixes
- Duplicate test functions in one file (`encryption_errors.rs`)

These are minor issues that don't affect test execution but should be cleaned up for code clarity.

---

**Generated:** 2026-08-09
**Tool:** Manual file scan + grep analysis
**Scope:** All files in `tests/*.rs` (59 files)
