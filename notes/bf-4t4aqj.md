# Test Directory Inventory for pdftract

## Summary of Test Locations

This document catalogs all test directories and integration test locations in the pdftract project as of 2026-08-09.

---

## 1. Top-Level Integration Test Directory

### Location: `./tests/`

**Purpose:** Primary integration test suite for the pdftract library (rust testing). Contains smoke tests, conformance tests, and property-based tests.

**Key Subdirectories:**
- `tests/proptest/` - Property-based tests using proptest
- `tests/proptest-regressions/` - Regression files for proptest
- `tests/sdk-conformance/` - SDK conformance test suite
- `tests/integration/` - Integration tests
- `tests/remote/` - Remote fetch/integration tests
- `tests/fixtures/` - PDF fixtures for testing
- `tests/document_model/` - Document model validation tests

**Test Files (115+ .rs files):**
- `integration_test.rs` - Main integration test entry
- `smoke_test.rs` - Basic functionality smoke tests
- `test_cases.rs` - General test case runner
- `test_fingerprint_*.rs` - Multiple fingerprint-related tests
- `test_page_access.rs` - Page access validation
- `test_parse_fixture.rs` - Parse fixture tests
- `encryption_*.rs` - Encryption-related tests
- `proptest/*.rs` - Property-based tests (cmap_parser, document_model, lexer, object_parser, stream_decoder, stream, xref)
- `TH-*.rs` - Threat hypothesis tests (security tests)
- `test_helpers.rs` - Test helper utilities
- `document_model.rs` - Document model validation

---

## 2. CLI Test Directory

### Location: `./crates/pdftract-cli/tests/`

**Purpose:** Tests for the pdftract CLI tool, including MCP integration tests and security validation.

**Test Files (23 .rs files):**
- `mcp-stdio.rs` - MCP stdio protocol tests
- `mcp-http.rs` - MCP HTTP protocol tests
- `mcp-tools-integration.rs` - MCP tools integration
- `mcp-cli-args.rs` - CLI argument parsing
- `conformance.rs` - CLI conformance tests
- `test_*.rs` - Various CLI feature tests
- `TH-*.rs` - Security threat hypothesis tests (path traversal, SSRF, XSS, etc.)

---

## 3. Core Library Test Directory

### Location: `./crates/pdftract-core/tests/`

**Purpose:** Integration and smoke tests for the pdftract-core library, including encryption, remote fetching, and OCR integration.

**Key Test Categories:**
- `encryption_*.rs` - Encryption tests (AES-128, AES-256, RC4)
- `test_*.rs` - Core functionality tests
- `TH-*.rs` - Threat hypothesis tests
- `remote_*.rs` - Remote fetching tests
- `test_helpers/` - Test helper modules
- `conformance.rs` - Core conformance validation

**Test Files (70+ .rs files):**
- `encryption_aes_128_test.rs`
- `encryption_aes_256_test.rs`
- `encryption_rc4_test.rs`
- `test_sdk_smoke.rs` - SDK smoke tests
- `test_basic_extraction.rs` - Basic extraction validation
- `TH-01-stream-bomb.rs` through `TH-10-cache-poison.rs` - Security tests
- `ocr_integration.rs` - OCR integration tests
- `orphaned_process_verification_test.rs` - Process cleanup verification

---

## 4. Library FFI Test Directory

### Location: `./crates/pdftract-libpdftract/tests/`

**Purpose:** Tests for the C FFI library (libpdftract).

**Test Files:**
- `test_parse.rs` - Parse functionality tests

---

## 5. Python SDK Test Directory

### Location: `./crates/pdftract-py/tests/`

**Purpose:** Python SDK tests using pytest.

**Test Files (.py):**
- `smoke_test.py` - Smoke tests
- `test_conformance.py` - Conformance validation
- `test_page_access.py` - Page access tests
- `test_span_access.py` - Span access tests
- `test_type_assertions.py` - Type validation
- `test_search_integration.py` - Search functionality

---

## 6. .NET SDK Test Directory

### Location: `./pdftract-dotnet/tests/`

**Purpose:** .NET SDK tests using C# testing frameworks.

**Test Files:**
- `SourceTests.cs` - Source tests
- `ConformanceTests.cs` - Conformance validation

---

## 7. Java SDK Test Directory

### Location: `./pdftract-java/src/test/java/com/jedarden/pdftract/`

**Purpose:** Java SDK tests using JUnit.

**Test Files:**
- `IntegrationTest.java` - Integration tests
- `ConformanceTest.java` - Conformance validation
- `PdftractTest.java` - General pdftract tests
- `AutoCloseableTest.java` - Resource cleanup tests

---

## 8. Node.js SDK Test Directory

### Location: `./pdftract-node/test/`

**Purpose:** Node.js SDK tests using Vitest.

**Test Files:**
- `conformance.test.ts` - Conformance validation
- `unit.test.ts` - Unit tests
- `codegen/conformance.test.ts` - Code generation conformance

---

## 9. Ruby SDK Test Directory

### Location: `./pdftract-ruby/test/`

**Purpose:** Ruby SDK tests.

**Test Files:**
- `conformance_test.rb` - Conformance validation

---

## 10. PHP SDK Test Directory

### Location: `./sdk/php/tests/`

**Purpose:** PHP SDK tests.

**Test Files:**
- `ConformanceTest.php` - Conformance validation
- `verify_psr3_logger.php` - PSR-3 logger verification

---

## 11. Property-Based Test Regression Directories

### Locations:
- `./proptest-regressions/`
- `./tests/proptest-regressions/`
- `./crates/pdftract-core/proptest-regressions/`

**Purpose:** Store regression test cases for proptest (property-based testing framework) to ensure reproducibility of found bugs.

---

## 12. SDK Conformance Test Suite

### Location: `./tests/sdk-conformance/`

**Purpose:** SDK conformance validation suite with JSON schemas and Python validation script.

**Contents:**
- `cases.json` - Test case definitions
- `schema.json` - Schema for test cases
- `report-schema.json` - Report validation schema
- `validate_suite.py` - Python validation script
- `fixtures/` - PDF fixtures for conformance testing

---

## Test Directory Purposes Summary

| Directory | Purpose | Language/Framework |
|-----------|---------|-------------------|
| `./tests/` | Main integration test suite | Rust (cargo test) |
| `./tests/proptest/` | Property-based tests | Rust (proptest) |
| `./tests/sdk-conformance/` | SDK conformance validation | JSON + Python |
| `./tests/integration/` | Advanced integration tests | Rust |
| `./tests/remote/` | Remote fetch tests | Rust |
| `./crates/pdftract-cli/tests/` | CLI integration tests | Rust |
| `./crates/pdftract-core/tests/` | Core library integration | Rust |
| `./crates/pdftract-libpdftract/tests/` | C FFI library tests | Rust |
| `./crates/pdftract-py/tests/` | Python SDK tests | Python (pytest) |
| `./pdftract-dotnet/tests/` | .NET SDK tests | C# |
| `./pdftract-java/src/test/` | Java SDK tests | Java (JUnit) |
| `./pdftract-node/test/` | Node.js SDK tests | TypeScript (Vitest) |
| `./pdftract-ruby/test/` | Ruby SDK tests | Ruby |
| `./sdk/php/tests/` | PHP SDK tests | PHP |

---

## Security Testing (TH- Tests)

The project includes threat hypothesis tests labeled `TH-NN` across multiple test directories:
- `TH-01`: Stream bomb protection
- `TH-02`: Path traversal protection
- `TH-03`: MCP authentication bypass
- `TH-04`: JavaScript presence detection
- `TH-05`: SSRF blocking
- `TH-06`: Checksum validation
- `TH-07`: PostScript leak protection
- `TH-08`: Log audit
- `TH-09`: Inspector XSS
- `TH-10`: Cache poisoning prevention

---

## Test Discovery Notes

- Total **Rust test files**: 200+ .rs files across all tests/ directories
- **SDK tests**: Cover Python, .NET, Java, Node.js, Ruby, and PHP
- **Security tests**: TH-NN series tests validate threat hypotheses
- **Property-based tests**: Proptest-based fuzzing with regression tracking
- **Conformance suite**: JSON-based test definitions with Python validation

This inventory provides the foundation for understanding where tests are located before searching for unit tests embedded within source code.
