# Truncated Flate Test File Structure Analysis

**Date:** 2026-08-01  
**Bead:** bf-3n4i8i  
**Source:** `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`

## Overview

This is an integration test suite for truncated FlateDecode stream recovery. The tests verify that pdftract handles truncated/incomplete zlib compressed streams gracefully rather than crashing.

## Purpose

Tests behavior when encountering truncated FlateDecode streams that can occur from:
- Corrupted PDF downloads
- Partially written PDF files
- Malicious truncation

## Fixture

- **File:** `tests/fixtures/malformed/truncated-flate.pdf`
- **Helper:** `fixture_path()` function returns path to fixture
- Located at: `crates/pdftract-core/tests/fixtures/malformed/truncated-flate.pdf`

## Test Functions (9 tests)

### 1. `test_truncated_flate_fixture_exists` (lines 32-47)
**Purpose:** Prerequisite test verifying the fixture file exists and is non-empty.

### 2. `test_truncated_flate_parses_as_pdf` (lines 54-70)
**Purpose:** Verify the truncated file parses as valid PDF structure despite truncated streams.

### 3. `test_truncated_flate_emits_diagnostics` (lines 79-89)
**Purpose:** Verify truncated streams produce diagnostics (currently scaffolded - diagnostic API not yet exposed through `parse_pdf_file`).

### 4. `test_truncated_flate_partial_content_accessible` (lines 96-114)
**Purpose:** Verify page content remains accessible even when some streams are truncated.

### 5. `test_truncated_flate_extraction_result_structure` (lines 121-179)
**Purpose:** Test extraction using `PdfExtractor`, examining result structure and error/diagnostic fields. Includes debug output of full extraction result as JSON.

### 6. `test_truncated_flate_materialize_pages` (lines 192-244)
**Purpose:** Verify `materialize_pages()` loads page structure without panic, and that results are cached/stable across calls. Core verification for bead bf-45n42.

### 7. `test_truncated_flate_extract_page_returns_result` (lines 263-311)
**Purpose:** Verify `extract_page()` is callable and yields `Result<PageExtraction>`. Calls `extract_page()` unconditionally (even when page slice is empty) and handles both Ok/Err outcomes.

### 8. `test_truncated_flate_opens_with_extractor` (lines 320-346)
**Purpose:** Smoke test verifying `PdfExtractor::open()` handles truncated-flate.pdf without panic/crash. Validates fingerprint and page_count.

### 9. `test_truncated_flate_emits_stream_decode_error` (lines 359-397)
**Purpose:** Verify extraction emits `STREAM_DECODE_ERROR` diagnostic in `metadata.diagnostics`. Uses `extract_pdf()` to get full `ExtractionResult`. Follows error assertion pattern from bf-2h1nt research.

## Imports and Dependencies

```rust
use anyhow::Result;
use pdftract_core::document::{parse_pdf_file, PageExtraction, PdfExtractor};
use pdftract_core::extract::{extract_pdf, ExtractionOptions};
use std::path::PathBuf;
```

## Test Flow Pattern

Most tests follow this pattern:
1. Get fixture path via `fixture_path()`
2. Parse/open with `parse_pdf_file()` or `PdfExtractor::open()`
3. Call specific method being tested
4. Assert on result/behavior
5. Print debug output for visibility

## Helper Functions

- `fixture_path()` (lines 22-26): Returns PathBuf to the truncated-flate.pdf fixture

## Key Findings

1. **Fixture yields empty page slice:** Due to truncation, `materialize_pages()` returns an empty slice on this fixture.

2. **Error handling approach:** Tests verify graceful degradation - no panics, proper error returns, diagnostic emission.

3. **Caching behavior:** `materialize_pages()` caches results - repeated calls return same data without re-flattening.

4. **Type visibility:** Test 7 explicitly types result as `Result<PageExtraction>` to make structure visible to compiler.

5. **Diagnostic assertion pattern:** Test 9 uses `.iter().any(|d| d.contains("STREAM_DECODE_ERROR"))` pattern from bf-2h1nt research.

6. **Scaffold test:** Test 3 is scaffolded - notes that diagnostic API is not yet exposed through `parse_pdf_file`.

## Related Beads

- bf-45n42: parent bead for `materialize_pages()` verification
- bf-2goux: parent of bead bf-45n42
- bf-2h1nt: research bead on error assertion patterns
