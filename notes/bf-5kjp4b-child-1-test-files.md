# Test File Inventory - pdftract

Generated: 2026-08-08
Bead: bf-5kjp4b-child-1

## Overview

This document catalogs all test files in the pdftract repository, organized by location and crate/module. Test files are categorized into:

1. **Main integration tests** (`tests/`)
2. **Per-crate test directories** (`crates/*/tests/`)
3. **SDK test files** (Python, PHP, .NET, C)
4. **Security test harness files** (TH-NN series)
5. **Tool and helper test scripts**

---

## 1. Main Integration Tests (`tests/`)

Location: `/home/coding/pdftract/tests/`

### Core Integration Test Files
- `integration_test.rs` - Main integration test suite
- `smoke_test.rs` - Basic smoke tests
- `lib.rs` - Integration test library module
- `mod.rs` - Integration test module loader

### Content Stream & Fingerprint Debug Tests
- `debug_a85_filter.rs` - ASCII85 filter debugging
- `debug_content_edit_fingerprint.rs` - Content editing fingerprint tests
- `debug_content_edit_pages.rs` - Page-level content editing
- `debug_content_fingerprint_fixtures.rs` - Fingerprint fixture generation
- `debug_content_fingerprint.rs` - Content fingerprint debugging
- `debug_content_hash_integration.rs` - Content hash integration
- `debug_content_hash_one_glyph.rs` - Single glyph hash tests
- `debug_content_hash.rs` - Content hash debugging
- `debug_content_stream_hash.rs` - Stream hash debugging
- `debug_content_streams.rs` - Content stream parsing
- `debug_filter_array.rs` - PDF filter array tests
- `debug_fingerprint_content_edit.rs` - Fingerprint content editing
- `debug_fingerprint_content_hash.rs` - Fingerprint content hashing
- `debug_fingerprint_content.rs` - Content fingerprinting
- `debug_fingerprint_contents.rs` - Contents fingerprinting
- `debug_fingerprint_content_streams.rs` - Stream fingerprinting
- `debug_fingerprint_fixture_content.rs` - Fixture content fingerprinting
- `debug_fingerprint_issue.rs` - Fingerprint issue reproduction
- `debug_fixtures.rs` - Fixture debugging utilities
- `debug_lzw.rs` - LZW compression debugging
- `debug_missing_mediabox.rs` - Missing MediaBox error handling
- `debug_page_count.rs` - Page count verification
- `debug_parse_content_edit.rs` - Parse content editing
- `debug_parse.rs` - General parsing debug
- `debug_parse_simple.rs` - Simple parse tests
- `debug_span_access.rs` - Span access debugging

### Document Model Tests
- `document_model.rs` - Document model tests
- `document_model/mod.rs` - Document model test module
- `document_model/generate_expected_json.rs` - JSON expected output
- `document_model/generate_expected.rs` - Expected output generation
- `document_model/fixtures/generate_fixtures.rs` - Fixture generation
- `document_model/fixtures/src/main.rs` - Fixture generator binary

### Encryption Tests
- `encryption_errors.rs` - Encryption error handling
- `encryption_fixtures.rs` - Encryption fixture generation
- `encryption_fixtures_usage_example.rs` - Fixture usage examples

### Fingerprint Tests
- `fingerprint_fixtures.rs` - Fingerprint test fixtures
- `fingerprint_reproducibility.rs` - Fingerprint reproducibility
- `fingerprint_test_single_one.rs` - Single-case fingerprint tests

### General Test Utilities & Helpers
- `fixture_discovery.rs` - Test fixture discovery
- `forms_integration.rs` - Forms integration tests
- `gen_lexer_golden.rs` - Lexer golden file generation
- `json_schema.rs` - JSON schema validation
- `lib.rs` - Test library entry point
- `list_pdf_fixtures.rs` - PDF fixture listing
- `log_secret_fuzz.rs` - Secret fuzzing tests
- `mod.rs` - Test module loader
- `object_parser.rs` - Object parser tests
- `proptest-panic-verification.rs` - Proptest panic verification
- `stream_decoder_fixtures.rs` - Stream decoder fixtures
- `test_assertion_methods.rs` - Test assertion utilities
- `test_atomic_writer.rs` - Atomic writer tests
- `test_bomb_limit.rs` - Bomb limit tests
- `test_cases.rs` - Test case definitions
- `test_extract_content_stream_bytes.rs` - Content stream byte extraction
- `test_fingerprint_debug.rs` - Fingerprint debugging
- `test_glob_discovery.rs` - Glob pattern discovery
- `test_helpers.rs` - Test helper functions
- `test_import_path.rs` - Import path tests
- `test_page_access.rs` - Page access tests
- `test_parse_fixture.rs` - Fixture parsing tests
- `test_search_python.py` - Python search integration tests
- `verify_encryption_fixtures.rs` - Encryption fixture verification

### Doctor & Coverage Tests
- `doctor_runbook_coverage.rs` - Runbook coverage verification

### Property-Based Tests (`tests/proptest/`)
- `proptest/cmap_parser.rs` - CMAP parser property tests
- `proptest/document_model.rs` - Document model property tests
- `proptest/lexer.rs` - Lexer property tests
- `proptest/object_parser.rs` - Object parser property tests
- `proptest/stream_decoder.rs` - Stream decoder property tests
- `proptest/stream.rs` - Stream property tests
- `proptest/xref.rs` - Xref property tests

### Remote & HTTP Tests (`tests/remote/`)
- `remote/mod.rs` - Remote test module
- `remote/integration.rs` - Remote integration tests
- `remote/fixtures/generate_linearized.rs` - Linearized PDF fixtures
- `remote/fixtures/generate_multipage.rs` - Multipage fixtures

### SDK Conformance Tests (`tests/sdk-conformance/`)
- `sdk-conformance/fixtures/generate_fixtures.rs` - Fixture generation
- `sdk-conformance/fixtures/generate_proper_fixtures.rs` - Proper fixtures
- `sdk-conformance/fixtures/generate_stub_pdfs_fixed.rs` - Fixed stub PDFs
- `sdk-conformance/fixtures/generate_stub_pdfs.rs` - Stub PDF generation
- `sdk-conformance/fixtures/generate_stubs.rs` - Stub generation
- `sdk-conformance/fixtures/gen_fixtures_main.rs` - Fixture generator binary

### Stream Decoder Tests (`tests/stream_decoder/`)
- `stream_decoder/fixtures/generate_lzw_fixtures.rs` - LZW fixture generation
- `stream_decoder/fixtures/gen_lzw.rs` - LZW generator
- `stream_decoder/fixtures/gen_stream_lzw.rs` - Stream LZW generator
- `stream_decoder/fixtures/regen_lzw_fixtures.rs` - LZW fixture regeneration

### C Client Tests (`tests/c-client/`)
- `c-client/gen_test_pdf.rs` - C client test PDF generation

### Object Parser Tests (`tests/object_parser/`)
- `object_parser/fixtures/gen_deep_nesting.rs` - Deep nesting fixtures

### Integration Tests (`tests/integration/`)
- `integration/advanced/profiles.rs` - Advanced profile tests
- `integration/hybrid_fixtures.rs` - Hybrid fixture tests

### Fixtures (`tests/fixtures/`)
- `fixtures/mod.rs` - Fixtures module
- `fixtures/hybrid/mod.rs` - Hybrid fixtures module

### Python Conformance (`tests/python-conformance/`)
- `python-conformance/test_conformance.py` - Python conformance tests

### SDK Tests (`tests/sdk/`)
- `sdk/test_python_sdk.py` - Python SDK tests

### Conformance Tests (`tests/conformance/`)
- `conformance/test_conformance.py` - Conformance test suite

---

## 2. Per-Crate Test Directories (`crates/*/tests/`)

### pdftract-cli Tests (`crates/pdftract-cli/tests/`)

**CLI Integration Tests:**
- `cli_invocation_fixtures.rs` - CLI invocation fixtures
- `comparison_mode_test.rs` - Comparison mode testing
- `conformance.rs` - Conformance testing
- `fixture_discovery.rs` - Fixture discovery
- `forms_integration.rs` - Forms integration
- `multi_output_validation.rs` - Multi-output format validation
- `pdftract_invocation.rs` - Direct pdftract binary invocation
- `root-path-protection.rs` - Root path protection tests
- `single_page_access.rs` - Single-page access tests

**Document Type Tests:**
- `test_book_chapter.rs` - Book/chapter document tests
- `test_contract.rs` - Contract document tests
- `test_encryption_errors.rs` - Encryption error handling
- `test_encryption_unsupported.rs` - Unsupported encryption tests
- `test_form.rs` - Form document tests
- `test_hash_exit_codes.rs` - Hash command exit codes
- `test_header_flag.rs` - Header flag tests
- `test_legal_filing.rs` - Legal filing document tests
- `test_scientific_paper.rs` - Scientific paper document tests
- `test_slide_deck.rs` - Slide deck document tests

**MCP Tests:**
- `mcp-cli-args.rs` - MCP CLI argument handling
- `mcp-http.rs` - MCP over HTTP
- `mcp-stdio.rs` - MCP over stdio
- `mcp-tools-integration.rs` - MCP tools integration

**Security Test Harness (TH-NN series):**
- `TH-02-path-traversal.rs` - Path traversal security tests
- `TH-05-ssrf-block.rs` - SSRF blocking tests
- `TH-08-log-audit.rs` - Log audit tests
- `TH-09-inspector-xss.rs` - Inspector XSS tests

### pdftract-core Tests (`crates/pdftract-core/tests/`)

**Core Functionality Tests:**
- `acceptance_crit_verification.rs` - Acceptance criteria verification
- `cjk_encoding.rs` - CJK encoding tests
- `classifier_corpus.rs` - Classifier corpus tests
- `cmap_unmapped_glyphs.rs` - CMAP unmapped glyph tests
- `conformance.rs` - Core conformance tests
- `document_model.rs` - Document model tests
- `encoding_recovery.rs` - Encoding recovery
- `error_recovery_integration.rs` - Error recovery integration
- `http_range_integration.rs` - HTTP range request integration
- `json_schema.rs` - JSON schema validation
- `memory_guard.rs` - Memory guard tests
- `memory_guard_tests.rs` - Memory guard test suite
- `object_parser.rs` - Object parser tests
- `object_parser_proptest.rs` - Object parser property tests
- `ocr_integration.rs` - OCR integration tests
- `page_classification.rs` - Page classification
- `schema_validate_fixtures.rs` - Schema fixture validation
- `struct_tree_coverage.rs` - Struct tree coverage
- `test_basic_extraction.rs` - Basic extraction tests
- `test_cycle_detection.rs` - Cycle detection
- `test_decoder_debug.rs` - Decoder debugging
- `test_filter_array_debug.rs` - Filter array debugging
- `test_fixture_read.rs` - Fixture reading tests
- `test_fixtures.rs` - General fixture tests
- `test_lzw_debug.rs` - LZW debugging
- `test_page_access.rs` - Page access tests
- `test_sdk_extraction_simple.rs` - Simple SDK extraction
- `test_sdk_smoke.rs` - SDK smoke tests
- `test_truncated_flate_recovery.rs` - Truncated FLATE recovery
- `test_type3_integration.rs` - Type 3 font integration
- `test_xref_debug.rs` - Xref debugging
- `th06_checksum_test.rs` - Checksum tests
- `verify_proptest_catches_bugs.rs` - Proptest bug verification
- `xref_helpers.rs` - Xref helper functions
- `xref_integration_test.rs` - Xref integration tests

**Encryption Tests:**
- `encryption_aes_128_test.rs` - AES-128 encryption tests
- `encryption_aes_256_test.rs` - AES-256 encryption tests
- `encryption_integration_tests.rs` - Encryption integration
- `encryption_rc4_test.rs` - RC4 encryption tests

**Fingerprint Tests:**
- `debug_fingerprint.rs` - Fingerprint debugging
- `debug_fingerprint_fixtures.rs` - Fingerprint fixtures
- `fingerprint_debug_content_edit.rs` - Content editing fingerprint
- `fingerprint_reproducibility.rs` - Fingerprint reproducibility

**Debug & Generation Tools:**
- `debug_content_streams.rs` - Content stream debugging
- `debug_page_parsing.rs` - Page parsing debugging
- `debug_serialization.rs` - Serialization debugging
- `generate_document_model_golden.rs` - Document model golden files
- `stream_decoder_fixtures.rs` - Stream decoder fixtures

**Remote & HTTP Tests:**
- `remote_fetch_integration.rs` - Remote fetch integration
- `remote_fetch_sequence.rs` - Remote fetch sequencing
- `remote_forward_scan_disable.rs` - Forward scan disable
- `remote_http_source_tests.rs` - HTTP source tests
- `remote_integration.rs` - Remote integration tests
- `remote_mock_server_tests.rs` - Mock server tests
- `remote_tls_tests.rs` - TLS tests
- `test_416_debug.rs` - HTTP 416 error debugging

**Security Test Harness (TH-NN series):**
- `TH-01-stream-bomb.rs` - Stream bomb protection
- `TH-03-mcp-no-auth.rs` - MCP authentication bypass
- `TH-04-js-presence.rs` - JavaScript presence detection
- `TH-05-ssrf-block.rs` - SSRF blocking
- `TH-07-ps-leak.rs` - Process secret leak
- `TH-10-cache-poison.rs` - Cache poisoning tests

**Helper Modules:**
- `test_helpers/mod.rs` - Test helper module
- `test_helpers/process_guard.rs` - Process guard utilities
- `orphaned_process_verification_test.rs` - Orphaned process verification

**Unmapped Glyph Tests:**
- `unmapped_glyph_names_config.rs` - Unmapped glyph configuration

### pdftract-libpdftract Tests (`crates/pdftract-libpdftract/tests/`)

- `test_parse.rs` - FFI parse function tests

### pdftract-py Tests (`crates/pdftract-py/tests/`)

**Python Binding Tests:**
- `smoke_test.py` - Python binding smoke tests
- `test_conformance.py` - Conformance testing
- `test_page_access.py` - Page access tests
- `test_page_access_simple.py` - Simple page access
- `test_search_integration.py` - Search integration (Python)
- `test_search_scaffold.rs` - Search scaffold (Rust)
- `test_search_integration.rs` - Search integration (Rust)
- `test_span_access.py` - Span access tests
- `test_span_access_simple.py` - Simple span access
- `test_type_assertions.py` - Type assertions
- `test_types.py` - Type tests

---

## 3. SDK Test Files

### Python SDK Tests (`crates/pdftract-py/` root level)

- `test_contract_methods.py` - Contract method tests
- `test_fallback_smoke.py` - Fallback smoke tests

### PHP SDK Tests (`sdk/php/tests/`)

- `ConformanceTest.php` - PHP conformance test suite
- `verify_psr3_logger.php` - PSR-3 logger verification

### C Client Tests (`tests/c-client/` & root level)

C conformance binaries and test generators:
- `tests/c-client/gen_test_pdf.rs`
- Root-level C test binaries (compiled, not source)

### .NET SDK Tests (`pdftract-dotnet/tests/`)

No .NET test files found in current scan.

---

## 4. Security Test Harness Files (TH-NN Series)

**CLI Security Tests:**
- `crates/pdftract-cli/tests/TH-02-path-traversal.rs` - Path traversal
- `crates/pdftract-cli/tests/TH-05-ssrf-block.rs` - SSRF blocking
- `crates/pdftract-cli/tests/TH-08-log-audit.rs` - Log auditing
- `crates/pdftract-cli/tests/TH-09-inspector-xss.rs` - Inspector XSS

**Core Security Tests:**
- `crates/pdftract-core/tests/TH-01-stream-bomb.rs` - Stream bomb
- `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs` - MCP auth bypass
- `crates/pdftract-core/tests/TH-04-js-presence.rs` - JS detection
- `crates/pdftract-core/tests/TH-05-ssrf-block.rs` - SSRF blocking
- `crates/pdftract-core/tests/TH-07-ps-leak.rs` - Process secret leak
- `crates/pdftract-core/tests/TH-10-cache-poison.rs` - Cache poisoning

---

## 5. Tool & Helper Test Scripts (Root Level)

### Test Analysis Scripts
- `analyze_false_positive_tests.py` - False positive analysis
- `analyze_tests.py` - Test analysis utility
- `check_false_positive_tests.py` - False positive checking
- `check_false_positive_tests_v2.py` - False positive checking v2
- `check_false_positive_tests_v3.py` - False positive checking v3
- `check_false_positive_tests_v4.py` - False positive checking v4
- `check_src_test_signatures.rs` - Source test signature verification
- `check_test_signatures.py` - Test signature checking
- `check_test_signatures_v2.py` - Test signature checking v2
- `check_test_signatures_v3.py` - Test signature checking v3
- `verify_test_attributes.py` - Test attribute verification
- `verify_test_conventions.py` - Test convention verification
- `verify_test_signatures.py` - Test signature verification

### Test Standalone Scripts
- `scripts/generate_test_corpus.py` - Test corpus generation
- `tools/test_extract_functions.py` - Extract function tests
- `tools/test_extract_markdown_bug.py` - Markdown bug tests
- `tools/test_markdown_extraction.py` - Markdown extraction tests
- `tools/test_rust_markdown.rs` - Rust markdown tests
- `tools/test_rust_sdk.rs` - Rust SDK tests

### Root-Level Test Files (Development/Debug)
- `test_debug.rs` - General debugging tests
- `test_debug2.rs` - Additional debugging tests
- `test_fixture_discovery_simple.rs` - Simple fixture discovery
- `test_ref_type.rs` - Reference type tests
- `test_sdk_types_smoke.py` - SDK types smoke test
- `test_single_function.py` - Single function tests
- `test_search_python.py` - Python search integration

---

## 6. Inline Test Modules (within source files)

Source files containing `#[cfg(test)]` modules:

### CLI Source Tests
- `crates/pdftract-cli/src/cache_cmd.rs`
- `crates/pdftract-cli/src/classify.rs`
- `crates/pdftract-cli/src/doctor/checks/*.rs` (multiple doctor checks)
- `crates/pdftract-cli/src/grep/mod.rs` and grep modules
- `crates/pdftract-cli/src/hash.rs`
- `crates/pdftract-cli/src/header.rs`
- `crates/pdftract-cli/src/inspect/*.rs` (inspect modules)
- `crates/pdftract-cli/src/mcp/*.rs` (MCP modules)
- `crates/pdftract-cli/src/middleware/*.rs` (middleware modules)
- `crates/pdftract-cli/src/output.rs`
- `crates/pdftract-cli/src/pages.rs`
- `crates/pdftract-cli/src/password.rs`
- `crates/pdftract-cli/src/serve.rs`
- `crates/pdftract-cli/src/url.rs`
- `crates/pdftract-cli/src/validate.rs`

### Core Source Tests
- `crates/pdftract-core/src/annotation/mod.rs`
- `crates/pdftract-core/src/encryption/mod.rs`
- `crates/pdftract-core/src/font/mod.rs`
- `crates/pdftract-core/src/forms/mod.rs`
- `crates/pdftract-core/src/fingerprint/mod.rs`
- `crates/pdftract-core/src/receipts/mod.rs`
- `crates/pdftract-core/src/signature/mod.rs`
- `crates/pdftract-core/src/span/mod.rs`
- `crates/pdftract-core/src/table/mod.rs`
- `crates/pdftract-core/src/profiles/mod.rs`
- `crates/pdftract-core/src/schema/mod.rs`
- `crates/pdftract-core/src/glyph/mod.rs`
- `crates/pdftract-core/src/threads/mod.rs`
- `crates/pdftract-core/src/parser/lexer/mod.rs`

### Other Crates
- `crates/pdftract-inspector-ui/src/lib.rs`
- `crates/pdftract-schema-migrate/src/lib.rs`

---

## Summary Statistics

| Category | File Count |
|----------|------------|
| Main integration tests (`tests/`) | ~100+ |
| CLI tests (`crates/pdftract-cli/tests/`) | ~30 |
| Core tests (`crates/pdftract-core/tests/`) | ~80 |
| Python binding tests (`crates/pdftract-py/tests/`) | ~11 |
| C client tests | ~1 |
| PHP SDK tests | ~2 |
| Security test harness (TH-NN) | ~10 |
| Tool/helper scripts | ~20+ |
| **Total** | **~250+ test files** |

---

## Test File Types by Language

| Language | Extension | Count |
|----------|-----------|-------|
| Rust | `.rs` | ~200+ |
| Python | `.py` | ~30+ |
| PHP | `.php` | ~2 |
| C | `.c` | ~10 (compiled binaries) |

---

## Notes

1. **Worktree files excluded:** Files under `.claude/worktrees/` are excluded from this inventory as they are temporary worktree copies.
2. **Template files excluded:** Files under `templates/` are SDK skeleton templates and not part of the main test suite.
3. **Fixture directories:** Many test files have associated fixture directories (e.g., `fixtures/`, `document_model/fixtures/`) that are not individually listed here but are referenced by test files.
4. **Property tests:** Files under `proptest/` directories use property-based testing with the `proptest` crate.
5. **Conformance tests:** Multiple language-specific conformance tests exist for SDK validation.
6. **Security tests:** TH-NN series tests are security-focused integration tests for specific threat models.

---

## Next Steps

This inventory provides the foundation for:
- Analyzing test warnings across all test files
- Identifying unused test utilities
- Consolidating duplicate test patterns
- Reviewing test coverage across modules
- Standardizing test organization and conventions
