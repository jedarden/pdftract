# pdftract Test File Inventory

**Generated:** 2026-08-09  
**Purpose:** Complete listing of all test files in the pdftract project for bead bf-5kjp4b

## Summary Statistics

- **Total .rs files with test functions:** 184 files
- **Total .rs files with 'test' in filename:** 729 files  
- **Main tests/ directory:** 91 .rs files
- **Source files with inline tests:** 4 files
- **Fuzz targets:** 7 files

---

## 1. Main Integration Tests Directory (`/tests/`)

**Location:** `/home/coding/pdftract/tests/`  
**Type:** Integration tests, fixtures, conformance tests  
**Count:** 91 .rs files

### Core Test Files

| File | Purpose |
|------|---------|
| `lib.rs` | Test library entry point |
| `mod.rs` | Test module declarations |
| `smoke_test.rs` | Basic smoke tests |
| `document_model.rs` | Document model tests |
| `json_schema.rs` | JSON schema validation |
| `fixture_discovery.rs` | Test fixture discovery |
| `test_helpers.rs` | Test helper utilities |
| `test_cases.rs` | Test case definitions |

### Encryption Tests

| File | Purpose |
|------|---------|
| `encryption_errors.rs` | Encryption error handling |
| `encryption_fixtures.rs` | Encryption test fixtures |
| `encryption_fixtures_usage_example.rs` | Usage examples |
| `verify_encryption_fixtures.rs` | Fixture verification |

### Fingerprint Tests

| File | Purpose |
|------|---------|
| `fingerprint_fixtures.rs` | Fingerprint test fixtures |
| `fingerprint_reproducibility.rs` | Reproducibility tests |
| `fingerprint_test_single_one.rs` | Single glyph tests |
| `debug_fingerprint_*.rs` | Multiple debug fingerprint files |
| `fingerprint_debug_content_edit.rs` | Content edit fingerprinting |

### Content Hash & Stream Tests

| File | Purpose |
|------|---------|
| `debug_content_hash*.rs` | Content hash debug/tests |
| `debug_content_stream_hash.rs` | Stream hash tests |
| `debug_content_streams.rs` | Content stream parsing |
| `stream_decoder_fixtures.rs` | Decoder fixtures |

### Object Parser Tests

| File | Purpose |
|------|---------|
| `object_parser.rs` | Object parser tests |
| `debug_filter_array.rs` | Filter array tests |
| `debug_parse*.rs` | Various parse debug files |

### Proptest Tests

| File | Purpose |
|------|---------|
| `proptest-panic-verification.rs` | Proptest panic verification |
| `proptest/stream.rs` | Stream proptests |
| `proptest/cmap_parser.rs` | CMAP parser proptests |
| `proptest/object_parser.rs` | Object parser proptests |
| `proptest/lexer.rs` | Lexer proptests |
| `proptest/stream_decoder.rs` | Stream decoder proptests |
| `proptest/xref.rs` | XREF proptests |
| `proptest/document_model.rs` | Document model proptests |

### Integration Tests

| File | Purpose |
|------|---------|
| `integration_test.rs` | Integration test entry |
| `integration/hybrid_fixtures.rs` | Hybrid fixture tests |
| `remote/integration.rs` | Remote integration tests |
| `remote/mod.rs` | Remote test modules |

### SDK Conformance Tests

| File | Purpose |
|------|---------|
| `sdk-conformance/fixtures/gen_fixtures_main.rs` | Fixture generation |
| `sdk-conformance/fixtures/generate_proper_fixtures.rs` | Proper fixture generation |
| `sdk-conformance/fixtures/generate_stub_pdfs.rs` | Stub PDF generation |
| `sdk-conformance/fixtures/generate_fixtures.rs` | Fixtures generator |
| `sdk-conformance/fixtures/generate_stubs.rs` | Stubs generator |
| `sdk-conformance/fixtures/generate_stub_pdfs_fixed.rs` | Fixed stub generator |

### Document Model Tests

| File | Purpose |
|------|---------|
| `document_model/generate_expected_json.rs` | Expected JSON generator |
| `document_model/generate_expected.rs` | Expected value generator |
| `document_model/mod.rs` | Document model modules |
| `document_model/fixtures/generate_fixtures.rs` | Fixture generation |
| `document_model/fixtures/src/main.rs` | Fixture generator entry |

### Object Parser Tests

| File | Purpose |
|------|---------|
| `object_parser/fixtures/gen_deep_nesting.rs` | Deep nesting fixture generator |

### Stream Decoder Tests

| File | Purpose |
|------|---------|
| `stream_decoder/fixtures/gen_stream_lzw.rs` | LZW stream generator |
| `stream_decoder/fixtures/gen_lzw.rs` | LZW generator |
| `stream_decoder/fixtures/generate_lzw_fixtures.rs` | LZW fixture generator |
| `stream_decoder/fixtures/regen_lzw_fixtures.rs` | LWW fixture regenerator |

### Remote Tests

| File | Purpose |
|------|---------|
| `remote/fixtures/generate_linearized.rs` | Linearized PDF generator |
| `remote/fixtures/generate_multipage.rs` | Multipage PDF generator |

### Other Test Files

| File | Purpose |
|------|---------|
| `test_assertion_methods.rs` | Assertion method tests |
| `test_atomic_writer.rs` | Atomic writer tests |
| `test_bomb_limit.rs` | Bomb limit tests |
| `test_extract_content_stream_bytes.rs` | Content stream byte extraction |
| `test_fingerprint_debug.rs` | Fingerprint debug tests |
| `test_glob_discovery.rs` | Glob pattern discovery |
| `test_import_path.rs` | Import path tests |
| `test_page_access.rs` | Page access tests |
| `test_parse_fixture.rs` | Parse fixture tests |
| `debug_span_access.rs` | Span access debug |
| `debug_missing_mediabox.rs` | Missing MediaBox debug |
| `debug_lzw.rs` | LZW debug |
| `debug_a85_filter.rs` | ASCII85 filter debug |
| `log_secret_fuzz.rs` | Secret fuzz logging |
| `forms_integration.rs` | Forms integration tests |
| `gen_lexer_golden.rs` | Lexer golden file generation |
| `doctor_runbook_coverage.rs` | Runbook coverage tests |

### C Client Tests

| File | Purpose |
|------|---------|
| `c-client/gen_test_pdf.rs` | C client test PDF generation |

### Fixtures

| File | Purpose |
|------|---------|
| `fixtures/mod.rs` | Fixture module declarations |
| `fixtures/hybrid/mod.rs` | Hybrid fixtures |

---

## 2. Source Files with Inline Tests (`/src/`)

**Location:** `/home/coding/pdftract/src/`  
**Type:** Unit tests embedded in source files via `#[cfg(test)]` modules  
**Count:** 4 files

| File | Test Module |
|------|-------------|
| `src/graphics_state/color.rs` | Color tests |
| `src/graphics_state/matrix.rs` | Matrix transformation tests |
| `src/graphics_state/stack.rs` | Graphics state stack tests |
| `src/graphics_state/state.rs` | State management tests |

---

## 3. CLI Tests (`/crates/pdftract-cli/tests/`)

**Location:** `/home/coding/pdftract/crates/pdftract-cli/tests/`  
**Type:** CLI integration tests  
**Count:** 24 .rs files

| File | Purpose |
|------|---------|
| `mcp-stdio.rs` | MCP stdio tests |
| `mcp-cli-args.rs` | MCP CLI argument tests |
| `mcp-http.rs` | MCP HTTP tests |
| `mcp-tools-integration.rs` | MCP tools integration |
| `test_header_flag.rs` | Header flag tests |
| `fixture_discovery.rs` | Fixture discovery tests |
| `test_contract.rs` | Contract tests |
| `conformance.rs` | CLI conformance tests |
| `forms_integration.rs` | Forms integration |
| `test_form.rs` | Form-specific tests |
| `test_book_chapter.rs` | Book/chapter tests |
| `single_page_access.rs` | Single page access tests |
| `root-path-protection.rs` | Root path protection tests |
| `test_slide_deck.rs` | Slide deck tests |
| `test_scientific_paper.rs` | Scientific paper tests |
| `comparison_mode_test.rs` | Comparison mode tests |
| `pdftract_invocation.rs` | pdftract invocation tests |
| `test_hash_exit_codes.rs` | Hash exit code tests |
| `TH-02-path-traversal.rs` | Path traversal security tests |
| `TH-05-ssrf-block.rs` | SSRF blocking tests |
| `TH-08-log-audit.rs` | Log audit tests |
| `TH-09-inspector-xss.rs` | Inspector XSS tests |
| `test_legal_filing.rs` | Legal filing tests |
| `test_encryption_errors.rs` | Encryption error tests |
| `test_encryption_unsupported.rs` | Unsupported encryption tests |
| `multi_output_validation.rs` | Multi-output validation tests |

---

## 4. Core Library Tests (`/crates/pdftract-core/tests/`)

**Location:** `/home/coding/pdftract/crates/pdftract-core/tests/`  
**Type:** Core library integration and unit tests  
**Count:** 70+ .rs files

### Security Tests (TH-* series)

| File | Purpose |
|------|---------|
| `TH-01-stream-bomb.rs` | Stream bomb protection |
| `TH-03-mcp-no-auth.rs` | MCP authentication bypass |
| `TH-04-js-presence.rs` | JavaScript presence detection |
| `TH-05-ssrf-block.rs` | SSRF blocking |
| `TH-07-ps-leak.rs` | PostScript leak prevention |
| `TH-10-cache-poison.rs` | Cache poisoning prevention |

### Encryption Tests

| File | Purpose |
|------|---------|
| `encryption_aes_128_test.rs` | AES-128 encryption tests |
| `encryption_aes_256_test.rs` | AES-256 encryption tests |
| `encryption_integration_tests.rs` | Encryption integration tests |
| `encryption_rc4_test.rs` | RC4 encryption tests |

### Remote Fetch Tests

| File | Purpose |
|------|---------|
| `remote_fetch_integration.rs` | Remote fetch integration |
| `remote_fetch_sequence.rs` | Fetch sequence tests |
| `remote_forward_scan_disable.rs` | Forward scan disable |
| `remote_http_source_tests.rs` | HTTP source tests |
| `remote_integration.rs` | Remote integration |
| `remote_mock_server_tests.rs` | Mock server tests |
| `remote_tls_tests.rs` | TLS tests |

### Content & Parsing Tests

| File | Purpose |
|------|---------|
| `cjk_encoding.rs` | CJK encoding tests |
| `cmap_unmapped_glyphs.rs` | CMAP unmapped glyphs |
| `classifier_corpus.rs` | Classifier corpus tests |
| `document_model.rs` | Document model tests |
| `encoding_recovery.rs` | Encoding recovery |
| `error_recovery_integration.rs` | Error recovery |

### Page & XREF Tests

| File | Purpose |
|------|---------|
| `page_classification.rs` | Page classification |
| `test_page_access.rs` | Page access tests |
| `test_page_helper_error_handling.rs` | Page helper error handling |
| `test_page_helper_extract_page.rs` | Page extraction |
| `test_page_iter_validation.rs` | Page iterator validation |
| `xref_helpers.rs` | XREF helper tests |
| `xref_integration_test.rs` | XREF integration |

### Object Parser Tests

| File | Purpose |
|------|---------|
| `object_parser.rs` | Object parser tests |
| `object_parser_proptest.rs` | Object parser proptests |

### Stream & Decoder Tests

| File | Purpose |
|------|---------|
| `stream_decoder_fixtures.rs` | Stream decoder fixtures |
| `test_truncated_flate_recovery.rs` | Truncated FLATE recovery |
| `test_lzw_debug.rs` | LZW debug tests |

### Type3 Font Tests

| File | Purpose |
|------|---------|
| `test_type3_integration.rs` | Type3 font integration |

### Fingerprint & Hash Tests

| File | Purpose |
|------|---------|
| `fingerprint_debug_content_edit.rs` | Fingerprint content edit |
| `fingerprint_reproducibility.rs` | Fingerprint reproducibility |
| `debug_fingerprint.rs` | Fingerprint debug |
| `debug_fingerprint_fixtures.rs` | Fingerprint fixtures |

### Document Model Tests

| File | Purpose |
|------|---------|
| `document_model/generate_expected_json.rs` | Expected JSON generation |
| `document_model/fixtures/generate_fixtures.rs` | Document model fixtures |

### Test Helpers & Utilities

| File | Purpose |
|------|---------|
| `test_helpers/mod.rs` | Test helper modules |
| `test_helpers/process_guard.rs` | Process guard utilities |
| `test_fixtures.rs` | Test fixtures |
| `test_fixture_read.rs` | Fixture reading tests |

### Debug & Verification Tests

| File | Purpose |
|------|---------|
| `debug_page_parsing.rs` | Page parsing debug |
| `debug_serialization.rs` | Serialization debug |
| `debug_content_streams.rs` | Content streams debug |
| `debug_fingerprint_fixtures.rs` | Fingerprint fixtures debug |

### Other Core Tests

| File | Purpose |
|------|---------|
| `acceptance_crit_verification.rs` | Acceptance criteria verification |
| `conformance.rs` | Conformance tests |
| `json_schema.rs` | JSON schema tests |
| `memory_guard.rs` | Memory guard tests |
| `memory_guard_tests.rs` | Memory guard detailed tests |
| `hint_stream_integration.rs` | Hint stream integration |
| `http_range_integration.rs` | HTTP range integration |
| `schema_validate_fixtures.rs` | Schema validation |
| `ocr_integration.rs` | OCR integration |
| `test_basic_extraction.rs` | Basic extraction tests |
| `test_cycle_detection.rs` | Cycle detection tests |
| `test_decoder_debug.rs` | Decoder debug tests |
| `test_filter_array_debug.rs` | Filter array debug |
| `test_sdk_extraction_simple.rs` | SDK extraction simple tests |
| `test_sdk_smoke.rs` | SDK smoke tests |
| `test_416_debug.rs` | 416 error debug |
| `test_xref_debug.rs` | XREF debug |
| `th06_checksum_test.rs` | Checksum test |
| `unmapped_glyph_names_config.rs` | Unmapped glyph config |
| `verify_proptest_catches_bugs.rs` | Proptest verification |
| `struct_tree_coverage.rs` | Struct tree coverage |
| `orphaned_process_verification_test.rs` | Orphaned process verification |
| `generate_document_model_golden.rs` | Document model golden generation |

---

## 5. Python SDK Tests (`/crates/pdftract-py/tests/`)

**Location:** `/home/coding/pdftract/crates/pdftract-py/tests/`  
**Type:** Python SDK integration tests  
**Count:** 2 .rs files

| File | Purpose |
|------|---------|
| `test_search_scaffold.rs` | Search scaffold tests |
| `test_search_integration.rs` | Search integration tests |

---

## 6. libpdftract Tests (`/crates/pdftract-libpdftract/tests/`)

**Location:** `/home/coding/pdftract/crates/pdftract-libpdftract/tests/`  
**Type:** FFI library tests  
**Count:** 1 .rs file

| File | Purpose |
|------|---------|
| `test_parse.rs` | Parse function tests |

---

## 7. Fuzz Tests (`/fuzz/`)

**Location:** `/home/coding/pdftract/fuzz/fuzz_targets/`  
**Type:** Fuzzing targets for AFL++ / libFuzzer  
**Count:** 7 .rs files

| File | Target |
|------|--------|
| `cmap_parser.rs` | CMAP parser fuzzing |
| `content.rs` | Content stream fuzzing |
| `lexer.rs` | Lexer fuzzing |
| `object_parser.rs` | Object parser fuzzing |
| `profile_yaml.rs` | Profile YAML parsing fuzzing |
| `stream_decoder.rs` | Stream decoder fuzzing |
| `xref.rs` | XREF parser fuzzing |

---

## 8. Source Files with Test Modules

**Location:** `/home/coding/pdftract/crates/pdftract-core/src/`  
**Type:** Inline unit tests within source files  
**Count:** Multiple files

### Font Test Files

| File | Purpose |
|------|---------|
| `src/font/type3_test_fixtures.rs` | Type3 test fixtures |
| `src/font/test_glyph_helper.rs` | Glyph helper tests |
| `src/font/type3_rasterizer_test.rs` | Type3 rasterizer tests |
| `src/font/type3_charproc_test.rs` | Type3 charproc tests |

---

## 9. Example Test Programs

**Location:** Various `examples/` directories  
**Type:** Standalone test/example programs

### Core Examples

| File | Purpose |
|------|---------|
| `crates/pdftract-core/examples/test_normalize_simple.rs` | Normalization tests |
| `crates/pdftract-core/examples/debug_fingerprint_test.rs` | Fingerprint test debug |
| `crates/pdftract-core/examples/test_pages_check.rs` | Pages check |
| `crates/pdftract-core/examples/test_cycle_detection_simple.rs` | Cycle detection tests |
| `crates/pdftract-core/examples/test_fingerprint_debug.rs` | Fingerprint debug |

### Debug Examples

| File | Purpose |
|------|---------|
| `crates/pdftract-core/examples/debug/test_lzw_api.rs` | LZW API debug |
| `crates/pdftract-core/examples/debug/test_root.rs` | Root debug |
| `crates/pdftract-core/examples/debug/test_docstrum.rs` | Docstrum debug |
| `crates/pdftract-core/examples/debug/test_decode_simple.rs` | Simple decode debug |
| `crates/pdftract-core/examples/debug/test_resolve.rs` | Resolve debug |
| `crates/pdftract-core/examples/debug/test_xref_entries.rs` | XREF entries debug |
| `crates/pdftract-core/examples/debug/test_xref.rs` | XREF debug |
| `crates/pdftract-core/examples/debug/test_debug.rs` | General debug |
| `crates/pdftract-core/examples/debug/test_forward_scan.rs` | Forward scan debug |
| `crates/pdftract-core/examples/debug/test_trailer.rs` | Trailer debug |
| `crates/pdftract-core/examples/debug/test_inline_image.rs` | Inline image debug |
| `crates/pdftract-core/examples/debug/test_lzw_debug.rs` | LZW debug |

### Root Examples

| File | Purpose |
|------|---------|
| `examples/test_parse_fixture.rs` | Parse fixture tests |
| `examples/debug_*.rs` | Various debug examples |
| `examples/test_*.rs` | Various test examples |

---

## 10. Generator & Fixture Scripts

**Location:** Root directory and `tools/`  
**Type:** Test fixture generators

| File | Purpose |
|------|---------|
| `generate_expected_json.rs` | Expected JSON generation |
| `gen_unmapped_comprehensive.rs` | Unmapped glyph generation |
| `tools/generate_form_fixtures.rs` | Form fixture generation |
| `tools/generate_invoice_fixture.rs` | Invoice fixture generation |
| `tools/generate_encrypted_pdf_fixtures.rs` | Encrypted PDF generation |
| `tools/generate_sensitive_fixture.rs` | Sensitive fixture generation |

---

## 11. Conformance & SDK Tests

### C Client Conformance

| File | Purpose |
|------|---------|
| `tests/conformance.c` | C conformance tests |
| `tests/conformance_fixed.c` | Fixed conformance tests |
| `tests/conformance_test_simple.c` | Simple conformance tests |
| `tests/test_api_basic.c` | Basic API tests |
| `tests/test_api_null.c` | Null API tests |
| `tests/test_api_real.c` | Real API tests |
| `tests/test_api_valid.c` | Validation API tests |
| `tests/test_debug.c` | Debug tests |
| `tests/test_simple.c` | Simple tests |
| `tests/test_stream.c` | Stream tests |
| `tests/test_valid.c` | Validation tests |

### Compiled Binaries

| File | Purpose |
|------|---------|
| `conformance_test` | Compiled conformance test binary |
| `conformance_run` | Conformance runner |
| `test_api_basic` | Basic API test binary |
| `test_api_null` | Null API test binary |
| `test_api_real` | Real API test binary |
| `test_api_valid` | Validation API test binary |
| `test_debug` | Debug test binary |
| `test_simple_run` | Simple test runner |
| `test_stream` | Stream test binary |
| `test_valid_run` | Validation test runner |

---

## 12. Python Test Scripts

**Location:** Root directory  
**Type:** Python test utilities

| File | Purpose |
|------|---------|
| `test_sdk_types_smoke.py` | SDK types smoke test |
| `test_single_function.py` | Single function test |
| `verify_ndjson_streaming.py` | NDJSON streaming verification |
| `test_search_python.py` | Search test for Python |

---

## Directory Structure Tree

```
/home/coding/pdftract/
├── tests/                          # Main integration tests (91 .rs files)
│   ├── integration/                # Integration test subdirectory
│   ├── remote/                    # Remote fetch tests
│   ├── fixtures/                  # Test fixtures
│   ├── proptest/                  # Property-based tests
│   ├── document_model/            # Document model tests
│   ├── object_parser/             # Object parser tests
│   ├── stream_decoder/            # Stream decoder tests
│   ├── sdk-conformance/           # SDK conformance tests
│   └── c-client/                  # C client tests
├── src/                           # Source with inline tests (4 files)
│   └── graphics_state/           # Graphics state tests
├── crates/
│   ├── pdftract-cli/tests/       # CLI tests (24 files)
│   ├── pdftract-core/tests/      # Core library tests (70+ files)
│   ├── pdftract-py/tests/        # Python SDK tests (2 files)
│   ├── pdftract-libpdftract/tests/ # FFI library tests (1 file)
│   └── pdftract-core/src/font/   # Font test helpers (4 files)
├── fuzz/
│   └── fuzz_targets/             # Fuzz targets (7 files)
├── examples/                     # Example test programs
└── tools/                        # Test utilities

```

---

## Test File Categories

### By Type

| Category | Count | Locations |
|----------|-------|-----------|
| Integration Tests | 91+ | `/tests/` |
| Unit Tests | 4 | `/src/` (inline) |
| CLI Tests | 24 | `/crates/pdftract-cli/tests/` |
| Core Library Tests | 70+ | `/crates/pdftract-core/tests/` |
| SDK Tests | 3 | `/crates/pdftract-py/tests/`, `/crates/pdftract-libpdftract/tests/` |
| Fuzz Tests | 7 | `/fuzz/fuzz_targets/` |
| Conformance Tests | 15+ | `/tests/` (C binaries) |
| Property-Based Tests | 8 | `/tests/proptest/`, core tests |
| Security Tests | 8 | TH-* series files |
| Encryption Tests | 7 | Multiple locations |

### By Purpose

| Purpose | Files |
|---------|-------|
| Encryption | `encryption_*.rs`, `TH-03-*.rs` |
| Security | `TH-*.rs` series (8 files) |
| Remote Fetch | `remote_*.rs` (7+ files) |
| Parsing | `*_parse*.rs`, `object_parser*.rs` |
| Streams | `stream_*.rs`, `*_decoder*.rs` |
| XREF | `xref_*.rs` |
| Fonts | `type3_*.rs`, `glyph_*.rs`, `cmap_*.rs` |
| Fingerprinting | `fingerprint_*.rs` |
| Document Model | `document_model*.rs` |
| Pages | `test_page*.rs` |
| CLI/Invocation | `pdftract_invocation.rs`, `mcp_*.rs` |
| Fixture Generation | `generate_*.rs`, `gen_*.rs` |

---

## Notes

1. **File counts are approximate** - Some generated files and temporary test files may not be permanently tracked
2. **Binary executables** - Compiled C test binaries are present but not counted in .rs totals
3. **Python tests** - Python test scripts are included for completeness
4. **Fuzz tests** - Fuzz targets are included but run separately from unit/integration tests
5. **SDK tests** - Each SDK has its own test structure (not all SDKs are listed here)
6. **Conformance tests** - Include both Rust and C implementations
7. **Integration vs Unit** - Files in `/tests/` are integration tests; inline `#[cfg(test)]` modules are unit tests

---

## Next Steps

This inventory provides the foundation for:
1. **Test signature verification** - Checking function signatures across test files
2. **Coverage analysis** - Understanding which modules are tested
3. **CI pipeline optimization** - Parallelizing independent test suites
4. **Test organization** - Identifying opportunities to consolidate or reorganize tests

**EOF**
