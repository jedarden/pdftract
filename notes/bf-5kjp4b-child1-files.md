# Test File Locations in pdftract Project

**Generated:** 2026-08-09
**Purpose:** Comprehensive catalog of all test files for compilation analysis

## Summary
- **Total test files:** 150+
- **Integration test directories:** 6 primary locations
- **Unit test files:** Embedded in source modules

## Integration Tests (tests/ directories)

### Root Level Integration Tests
**Location:** `/home/coding/pdftract/tests/`

#### Main Integration Test Files
- `integration_test.rs` - Primary integration test suite
- `smoke_test.rs` - Basic smoke tests
- `test_assertion_methods.rs` - Test assertion utilities
- `test_atomic_writer.rs` - Atomic write behavior tests
- `test_bomb_limit.rs` - Resource limit testing
- `test_cases.rs` - General test cases
- `test_extract_content_stream_bytes.rs` - Content stream extraction tests
- `test_fingerprint_debug.rs` - Fingerprint debugging tests
- `test_glob_discovery.rs` - Pattern discovery tests
- `test_helpers.rs` - Test helper utilities
- `test_import_path.rs` - Import path tests
- `test_page_access.rs` - Page access API tests
- `test_parse_fixture.rs` - Fixture parsing tests

#### Debug/Exploratory Test Files
- `debug_a85_filter.rs` - ASCII85 filter debugging
- `debug_content_edit_fingerprint.rs` - Content edit fingerprint tests
- `debug_content_edit_pages.rs` - Content page editing tests
- `debug_content_fixture_fixtures.rs` - Content fixture tests
- `debug_content_fingerprint.rs` - Content fingerprinting
- `debug_content_hash_integration.rs` - Content hash integration tests
- `debug_content_hash_one_glyph.rs` - Single glyph hash tests
- `debug_content_hash.rs` - Content hash tests
- `debug_content_stream_hash.rs` - Content stream hash tests
- `debug_content_streams.rs` - Content stream tests
- `debug_filter_array.rs` - Filter array tests
- `debug_fingerprint_content_edit.rs` - Fingerprint content editing
- `debug_fingerprint_content_hash.rs` - Fingerprint content hash tests
- `debug_fingerprint_content.rs` - Fingerprint content tests
- `debug_fingerprint_contents.rs` - Fingerprint contents tests
- `debug_fingerprint_content_streams.rs` - Fingerprint content streams
- `debug_fingerprint_fixture_content.rs` - Fingerprint fixture content
- `debug_fingerprint_issue.rs` - Fingerprint issue tests
- `debug_fixtures.rs` - General fixture debugging
- `debug_lzw.rs` - LZW compression debugging
- `debug_missing_mediabox.rs` - Missing mediabox tests
- `debug_page_count.rs` - Page count verification
- `debug_parse_content_edit.rs` - Parse content editing
- `debug_parse.rs` - General parse debugging
- `debug_parse_simple.rs` - Simple parse tests
- `debug_span_access.rs` - Span access tests

#### Document Model Tests
- `document_model.rs` - Document model integration
- `document_model/mod.rs` - Document model module
- `document_model/generate_expected_json.rs` - Expected JSON generation
- `document_model/generate_expected.rs` - Expected output generation
- `document_model/fixtures/generate_fixtures.rs` - Fixture generation
- `document_model/fixtures/src/main.rs` - Fixture generator main

#### Encryption Tests
- `encryption_errors.rs` - Encryption error handling
- `encryption_fixtures.rs` - Encryption fixture tests
- `encryption_fixtures_usage_example.rs` - Encryption usage examples

#### Fingerprint Tests
- `fingerprint_fixtures.rs` - Fingerprint fixtures
- `fingerprint_reproducibility.rs` - Fingerprint reproducibility tests
- `fingerprint_test_single_one.rs` - Single fingerprint tests

#### SDK Conformance Tests
- `sdk-conformance/fixtures/generate_fixtures.rs` - SDK fixture generator
- `sdk-conformance/fixtures/generate_proper_fixtures.rs` - Proper fixture generator
- `sdk-conformance/fixtures/generate_stub_pdfs_fixed.rs` - Stub PDF generator (fixed)
- `sdk-conformance/fixtures/generate_stub_pdfs.rs` - Stub PDF generator
- `sdk-conformance/fixtures/generate_stubs.rs` - Stub generator
- `sdk-conformance/fixtures/gen_fixtures_main.rs` - Fixture generator main

#### Stream Decoder Tests
- `stream_decoder_fixtures.rs` - Stream decoder fixtures
- `stream_decoder/fixtures/generate_lzw_fixtures.rs` - LZW fixture generator
- `stream_decoder/fixtures/gen_lzw.rs` - LZW generator
- `stream_decoder/fixtures/gen_stream_lzw.rs` - Stream LZW generator
- `stream_decoder/fixtures/regen_lzw_fixtures.rs` - LZW fixture regeneration

#### Property-Based Tests
- `proptest-panic-verification.rs` - Proptest panic verification
- `proptest/cmap_parser.rs` - CMAP parser property tests
- `proptest/document_model.rs` - Document model property tests
- `proptest/lexer.rs` - Lexer property tests
- `proptest/object_parser.rs` - Object parser property tests
- `proptest/stream_decoder.rs` - Stream decoder property tests
- `proptest/stream.rs` - Stream property tests
- `proptest/xref.rs` - XREF property tests

#### Object Parser Tests
- `object_parser.rs` - Object parser tests
- `object_parser/fixtures/gen_deep_nesting.rs` - Deep nesting fixture generator

#### Fixtures and Discovery
- `fixture_discovery.rs` - Fixture discovery tests
- `fixtures/mod.rs` - Fixtures module
- `fixtures/hybrid/mod.rs` - Hybrid fixtures module
- `list_pdf_fixtures.rs` - PDF fixture listing

#### C Client Tests
- `c-client/gen_test_pdf.rs` - C client test PDF generator

#### Forms Integration
- `forms_integration.rs` - Forms integration tests

#### JSON Schema Tests
- `json_schema.rs` - JSON schema validation tests

#### Lexer Golden Tests
- `gen_lexer_golden.rs` - Lexer golden file generation

#### Log Secret Fuzzing
- `log_secret_fuzz.rs` - Secret logging fuzz tests

#### Doctor/Runbook Coverage
- `doctor_runbook_coverage.rs` - Doctor runbook coverage verification

#### Remote Integration Tests
- `remote/mod.rs` - Remote tests module
- `remote/integration.rs` - Remote integration tests
- `remote/fixtures/generate_linearized.rs` - Linearized PDF generator
- `remote/fixtures/generate_multipage.rs` - Multipage PDF generator

#### Module Files
- `lib.rs` - Test library entry
- `mod.rs` - Tests module

### Crate-Specific Integration Tests

#### pdftract-core
**Location:** `/home/coding/pdftract/crates/pdftract-core/tests/`

- `encryption_aes_128_test.rs` - AES-128 encryption tests
- `encryption_aes_256_test.rs` - AES-256 encryption tests
- `encryption_rc4_test.rs` - RC4 encryption tests
- `orphaned_process_verification_test.rs` - Orphaned process verification (TH-03 fix validation)
- `test_416_debug.rs` - GitHub issue 416 debug tests
- `test_basic_extraction.rs` - Basic extraction tests
- `test_cycle_detection.rs` - Cycle detection tests
- `test_decoder_debug.rs` - Decoder debugging
- `test_filter_array_debug.rs` - Filter array debugging
- `test_fixture_read.rs` - Fixture reading tests
- `test_fixtures.rs` - General fixture tests
- `test_lzw_debug.rs` - LZW debugging tests
- `test_page_access.rs` - Page access API tests
- `test_page_helper_error_handling.rs` - Page helper error handling
- `test_page_helper_extract_page.rs` - Page helper extraction
- `test_sdk_extraction_simple.rs` - Simple SDK extraction tests
- `test_sdk_smoke.rs` - SDK smoke tests
- `test_truncated_flate_recovery.rs` - Truncated FLATE recovery
- `test_type3_integration.rs` - Type 3 font integration
- `test_xref_debug.rs` - XREF debugging
- `th06_checksum_test.rs` - TH-06 checksum tests
- `xref_integration_test.rs` - XREF integration tests

#### pdftract-cli
**Location:** `/home/coding/pdftract/crates/pdftract-cli/tests/`

- `comparison_mode_test.rs` - Comparison mode tests
- `test_book_chapter.rs` - Book chapter PDF tests
- `test_contract.rs` - Contract PDF tests
- `test_encryption_errors.rs` - Encryption error tests
- `test_encryption_unsupported.rs` - Unsupported encryption tests
- `test_form.rs` - Form PDF tests
- `test_hash_exit_codes.rs` - Hash exit code tests
- `test_header_flag.rs` - Header flag tests
- `test_legal_filing.rs` - Legal filing PDF tests
- `test_scientific_paper.rs` - Scientific paper PDF tests
- `test_slide_deck.rs` - Slide deck PDF tests

#### pdftract-libpdftract
**Location:** `/home/coding/pdftract/crates/pdftract-libpdftract/tests/`

- `test_parse.rs` - Parse tests
- `tests/` (subdirectory with additional tests)

#### pdftract-py
**Location:** `/home/coding/pdftract/crates/pdftract-py/tests/`

- `test_search_integration.rs` - Search integration tests
- `test_search_scaffold.rs` - Search scaffold tests

## Unit Tests (Embedded in Source)

### pdftract-core Source Tests
**Location:** `/home/coding/pdftract/crates/pdftract-core/src/`

- `font/test_glyph_helper.rs` - Glyph helper unit tests
- `font/type3_charproc_test.rs` - Type 3 charproc unit tests
- `font/type3_rasterizer_test.rs` - Type 3 rasterizer unit tests

## Example Test Files

### Root Level Examples
**Location:** `/home/coding/pdftract/examples/`

- `test_ascii85.rs` - ASCII85 encoding test
- `test_export.rs` - Export functionality test
- `test_parse_fixture.rs` - Fixture parsing test
- `test_simple_extract.rs` - Simple extraction test
- `test_source.rs` - Source test
- `test_url_host.rs` - URL host test

### pdftract-core Examples
**Location:** `/home/coding/pdftract/crates/pdftract-core/examples/`

#### Debug Examples
- `debug_fingerprint_test.rs` - Fingerprint test debug
- `debug/test_debug.rs` - General debug test
- `debug/test_decode_simple.rs` - Simple decode test
- `debug/test_docstrum.rs` - Docstrum test
- `debug/test_flate_png.rs` - FLATE PNG test
- `debug/test_forward_scan.rs` - Forward scan test
- `debug/test_inline_image.rs` - Inline image test
- `debug/test_lzw_api.rs` - LZW API test
- `debug/test_lzw_debug.rs` - LZW debug test
- `debug/test_resolve.rs` - Resolve test
- `debug/test_root.rs` - Root test
- `debug/test_trailer.rs` - Trailer test
- `debug/test_xref_entries.rs` - XREF entries test
- `debug/test_xref.rs` - XREF test

#### Test Examples
- `test_cycle_detection_simple.rs` - Simple cycle detection test
- `test_fingerprint_debug.rs` - Fingerprint debug test
- `test_normalize_simple.rs` - Simple normalization test
- `test_pages_check.rs` - Pages check test

### Standalone Test Files (Root)
- `test_debug2.rs` - Debug test 2
- `test_debug.rs` - Debug test
- `test_fixture_discovery_simple.rs` - Simple fixture discovery
- `test_ref_type.rs` - Reference type test
- `test_round.rs` - Round test

## Tool Tests
**Location:** `/home/coding/pdftract/tools/`

- `test_rust_markdown.rs` - Rust markdown test
- `test_rust_sdk.rs` - Rust SDK test

## Excluded Files
- `.claude/worktrees/` - Worktree files (not part of main project)
- `target/` - Build artifacts (gitignored)

## Test File Categories

### By Type
1. **Integration Tests:** Files in `tests/` directories (114+ files)
2. **Unit Tests:** Files embedded in `src/` with `_test.rs` suffix (3 files)
3. **Example Tests:** Standalone example files demonstrating features (25+ files)
4. **Property Tests:** Files using proptest for fuzzing (8 files)
5. **Debug Tests:** Exploratory/debugging test files (30+ files)

### By Purpose
1. **Functional Tests:** Core functionality verification
2. **Regression Tests:** Bug fix verification (TH-03, TH-06, issue 416, etc.)
3. **Fixture Tests:** Fixture generation and validation
4. **Integration Tests:** Cross-component integration
5. **Smoke Tests:** Quick validation tests
6. **Conformance Tests:** SDK conformance verification
7. **Fuzz Tests:** Property-based testing with proptest

### By Dependency Level
1. **Unit Tests:** No external dependencies (embedded in source)
2. **Crate Tests:** Per-crate integration tests
3. **Root Tests:** Full integration tests
4. **CLI Tests:** Command-line interface tests
5. **SDK Tests:** Python SDK tests

## Notes
- Total excludes worktree and build artifacts
- Some test files are generators (create fixtures, not run tests)
- Debug files are exploratory and may not be part of regular CI
- Property test files require `proptest` crate
- Orphaned process verification is critical for TH-03 fix validation